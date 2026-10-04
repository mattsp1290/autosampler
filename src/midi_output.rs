//! MIDI output port enumeration and sending via midir.

use midir::MidiOutput;
use thiserror::Error;

const MIDI_NOTE_ON: u8 = 0x90;
const MIDI_NOTE_OFF: u8 = 0x80;

/// A discovered MIDI output port.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MidiOutputPort {
    pub name: String,
    pub index: usize,
}

/// Errors from MIDI output operations.
#[derive(Debug, Error)]
pub enum MidiError {
    #[error("invalid MIDI {field}: {value}")]
    InvalidValue { field: &'static str, value: u8 },

    #[error("midir init error: {0}")]
    Init(#[from] midir::InitError),

    #[error("midir connect error: {0}")]
    Connect(#[from] midir::ConnectError<MidiOutput>),

    #[error("MIDI send error: {0}")]
    Send(#[from] midir::SendError),

    #[error("port name error: {0}")]
    PortName(#[from] midir::PortInfoError),

    #[error("MIDI output port {0} not found")]
    PortNotFound(usize),
}

/// Enumerate all available MIDI output ports.
pub fn list_midi_output_ports() -> Result<Vec<MidiOutputPort>, MidiError> {
    let midi_out = MidiOutput::new("autosampler")?;
    let ports = midi_out.ports();
    let mut result = Vec::with_capacity(ports.len());
    for (index, port) in ports.iter().enumerate() {
        let name = midi_out.port_name(port)?;
        result.push(MidiOutputPort { name, index });
    }
    Ok(result)
}

/// Send a one-shot Note On message to the given port.
///
/// Opens a fresh connection, sends the message, then closes it. Use
/// [`MidiSender`] when you need to send multiple messages without the
/// overhead of reconnecting each time.
pub fn send_note_on(
    port_index: usize,
    channel: u8,
    note: u8,
    velocity: u8,
) -> Result<(), MidiError> {
    note_on_message(channel, note, velocity)?;
    let mut sender = MidiSender::connect(port_index)?;
    sender.note_on(channel, note, velocity)
}

/// Send a one-shot Note Off message to the given port.
///
/// Opens a fresh connection, sends the message, then closes it. Use
/// [`MidiSender`] when you need to send multiple messages without the
/// overhead of reconnecting each time.
pub fn send_note_off(port_index: usize, channel: u8, note: u8) -> Result<(), MidiError> {
    note_off_message(channel, note)?;
    let mut sender = MidiSender::connect(port_index)?;
    sender.note_off(channel, note)
}

/// A persistent MIDI output connection for reuse across multiple sends.
///
/// Prefer this over the free [`send_note_on`] / [`send_note_off`] helpers
/// when sending many messages to the same port (e.g. during a sampling
/// session) to avoid the per-message connection overhead.
pub struct MidiSender {
    conn: midir::MidiOutputConnection,
}

impl MidiSender {
    /// Open a connection to the MIDI output port at `port_index`.
    pub fn connect(port_index: usize) -> Result<Self, MidiError> {
        let midi_out = MidiOutput::new("autosampler")?;
        let ports = midi_out.ports();
        let port = ports
            .get(port_index)
            .ok_or(MidiError::PortNotFound(port_index))?;
        let conn = midi_out.connect(port, "autosampler-out")?;
        Ok(Self { conn })
    }

    /// Send a Note On message (`0x90 | channel`, note, velocity).
    pub fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), MidiError> {
        self.conn.send(&note_on_message(channel, note, velocity)?)?;
        Ok(())
    }

    /// Send a Note Off message (`0x80 | channel`, note, velocity=0).
    pub fn note_off(&mut self, channel: u8, note: u8) -> Result<(), MidiError> {
        self.conn.send(&note_off_message(channel, note)?)?;
        Ok(())
    }
}

fn validate(field: &'static str, value: u8, max: u8) -> Result<(), MidiError> {
    if value > max {
        return Err(MidiError::InvalidValue { field, value });
    }
    Ok(())
}

pub(crate) fn validate_channel(channel: u8) -> Result<(), MidiError> {
    validate("channel", channel, 15)
}

pub(crate) fn note_on_message(channel: u8, note: u8, velocity: u8) -> Result<[u8; 3], MidiError> {
    validate_channel(channel)?;
    validate("note", note, 127)?;
    validate("velocity", velocity, 127)?;
    Ok([MIDI_NOTE_ON | channel, note, velocity])
}

fn note_off_message(channel: u8, note: u8) -> Result<[u8; 3], MidiError> {
    validate_channel(channel)?;
    validate("note", note, 127)?;
    Ok([MIDI_NOTE_OFF | channel, note, 0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_message_boundaries_and_velocity_zero() {
        assert_eq!(note_on_message(15, 127, 127).unwrap(), [0x9f, 127, 127]);
        assert_eq!(note_on_message(0, 60, 0).unwrap(), [0x90, 60, 0]);
        assert_eq!(note_off_message(15, 127).unwrap(), [0x8f, 127, 0]);
        for args in [(16, 60, 127), (0, 128, 127), (0, 60, 128)] {
            assert!(matches!(
                note_on_message(args.0, args.1, args.2),
                Err(MidiError::InvalidValue { .. })
            ));
        }
        assert!(note_off_message(16, 60).is_err());
        assert!(note_off_message(0, 128).is_err());
    }

    #[test]
    fn invalid_one_shot_values_fail_before_device_access() {
        assert!(matches!(
            send_note_on(usize::MAX, 16, 60, 127),
            Err(MidiError::InvalidValue { .. })
        ));
        assert!(matches!(
            send_note_off(usize::MAX, 0, 128),
            Err(MidiError::InvalidValue { .. })
        ));
    }
}
