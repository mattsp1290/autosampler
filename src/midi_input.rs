//! MIDI input port enumeration and event listening via midir.

use midir::MidiInput;
use thiserror::Error;

const MIDI_NOTE_ON: u8 = 0x90;
const MIDI_NOTE_OFF: u8 = 0x80;
const MIDI_STATUS_MASK: u8 = 0xF0;
const MIDI_CHANNEL_MASK: u8 = 0x0F;

/// A discovered MIDI input port.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MidiInputPort {
    pub name: String,
    pub index: usize,
}

/// Events received from a MIDI input device.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind"))]
pub enum MidiInputEvent {
    #[cfg_attr(feature = "serde", serde(rename = "note_on"))]
    NoteOn { note: u8, velocity: u8, channel: u8 },
    #[cfg_attr(feature = "serde", serde(rename = "note_off"))]
    NoteOff { note: u8, channel: u8 },
}

/// Errors from MIDI input operations.
#[derive(Debug, Error)]
pub enum MidiInputError {
    #[error("invalid MIDI input channel: {0}; expected 0–15")]
    InvalidChannel(u8),

    #[error("midir init error: {0}")]
    Init(#[from] midir::InitError),

    #[error("midir connect error: {0}")]
    Connect(#[from] midir::ConnectError<MidiInput>),

    #[error("port name error: {0}")]
    PortName(#[from] midir::PortInfoError),

    #[error("MIDI input port {0} not found")]
    PortNotFound(usize),
}

/// Enumerate all available MIDI input ports.
pub fn list_midi_input_ports() -> Result<Vec<MidiInputPort>, MidiInputError> {
    let midi_in = MidiInput::new("autosampler-in")?;
    let ports = midi_in.ports();
    let mut result = Vec::with_capacity(ports.len());
    for (index, port) in ports.iter().enumerate() {
        let name = midi_in.port_name(port)?;
        result.push(MidiInputPort { name, index });
    }
    Ok(result)
}

/// A persistent MIDI input connection that forwards events via a callback.
///
/// The connection stays open until this struct is dropped. The callback
/// is invoked on midir's internal thread for each incoming MIDI message.
pub struct MidiInputListener {
    // The connection is kept alive by this field. Dropping it disconnects.
    _conn: midir::MidiInputConnection<()>,
}

impl MidiInputListener {
    /// Connect to the MIDI input port at `port_index` and start listening.
    ///
    /// `filter_channel`: if `Some(ch)`, only forward events on that MIDI channel.
    /// If `None`, forward events on all channels.
    ///
    /// `on_event` is called from midir's internal thread on each MIDI message.
    /// **Do not panic inside `on_event`** — a panic will abort the MIDI thread.
    pub fn connect(
        port_index: usize,
        filter_channel: Option<u8>,
        on_event: impl Fn(MidiInputEvent) + Send + 'static,
    ) -> Result<Self, MidiInputError> {
        if let Some(channel) = filter_channel.filter(|&channel| channel > 15) {
            return Err(MidiInputError::InvalidChannel(channel));
        }
        let midi_in = MidiInput::new("autosampler-in")?;
        let ports = midi_in.ports();
        let port = ports
            .get(port_index)
            .ok_or(MidiInputError::PortNotFound(port_index))?;

        let conn = midi_in.connect(
            port,
            "autosampler-midi-in",
            move |_timestamp, message, _data| {
                if let Some(evt) = decode_note(message, filter_channel) {
                    on_event(evt);
                }
            },
            (),
        )?;

        Ok(Self { _conn: conn })
    }
}

fn decode_note(message: &[u8], filter_channel: Option<u8>) -> Option<MidiInputEvent> {
    let &[status, note, velocity, ..] = message else {
        return None;
    };
    let channel = status & MIDI_CHANNEL_MASK;
    if note > 127 || velocity > 127 || filter_channel.is_some_and(|ch| ch != channel) {
        return None;
    }
    match (status & MIDI_STATUS_MASK, velocity) {
        (MIDI_NOTE_ON, 1..=127) => Some(MidiInputEvent::NoteOn {
            note,
            velocity,
            channel,
        }),
        (MIDI_NOTE_ON, 0) | (MIDI_NOTE_OFF, _) => Some(MidiInputEvent::NoteOff { note, channel }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn note_decoding_covers_status_filter_and_velocity_zero() {
        assert!(matches!(
            decode_note(&[0x9f, 127, 127], Some(15)),
            Some(MidiInputEvent::NoteOn {
                note: 127,
                velocity: 127,
                channel: 15
            })
        ));
        for message in [[0x92, 60, 0], [0x82, 60, 64]] {
            assert!(matches!(
                decode_note(&message, None),
                Some(MidiInputEvent::NoteOff {
                    note: 60,
                    channel: 2
                })
            ));
        }
        assert!(decode_note(&[0x92, 60, 127], Some(1)).is_none());
        for message in [
            &[][..],
            &[0x90][..],
            &[0x90, 60][..],
            &[0xb0, 60, 127][..],
            &[0xf8, 0, 0][..],
            &[0x90, 128, 127][..],
            &[0x80, 60, 128][..],
        ] {
            assert!(decode_note(message, None).is_none(), "{message:?}");
        }
    }

    #[test]
    fn invalid_filter_fails_before_device_access() {
        assert!(matches!(
            MidiInputListener::connect(usize::MAX, Some(16), |_| {}),
            Err(MidiInputError::InvalidChannel(16))
        ));
    }
}
