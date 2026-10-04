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
        let midi_in = MidiInput::new("autosampler-in")?;
        let ports = midi_in.ports();
        let port = ports
            .get(port_index)
            .ok_or(MidiInputError::PortNotFound(port_index))?;

        let conn = midi_in.connect(
            port,
            "autosampler-midi-in",
            move |_timestamp, message, _data| {
                if message.len() < 2 {
                    return;
                }

                let status = message[0] & MIDI_STATUS_MASK;
                let channel = message[0] & MIDI_CHANNEL_MASK;

                // Filter by channel if requested
                if filter_channel.is_some_and(|ch| channel != ch) {
                    return;
                }

                let event = match status {
                    MIDI_NOTE_ON if message.len() >= 3 && message[2] > 0 => {
                        // Note On (velocity > 0)
                        Some(MidiInputEvent::NoteOn {
                            note: message[1],
                            velocity: message[2],
                            channel,
                        })
                    }
                    MIDI_NOTE_OFF if message.len() >= 3 => {
                        // Note Off
                        Some(MidiInputEvent::NoteOff {
                            note: message[1],
                            channel,
                        })
                    }
                    MIDI_NOTE_ON if message.len() >= 3 && message[2] == 0 => {
                        // Note On with velocity 0 = Note Off
                        Some(MidiInputEvent::NoteOff {
                            note: message[1],
                            channel,
                        })
                    }
                    _ => None,
                };

                if let Some(evt) = event {
                    on_event(evt);
                }
            },
            (),
        )?;

        Ok(Self { _conn: conn })
    }
}
