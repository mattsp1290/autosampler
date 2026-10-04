//! MIDI output port enumeration and sending via midir.

use midir::MidiOutput;
use thiserror::Error;

const MIDI_NOTE_ON: u8 = 0x90;
const MIDI_NOTE_OFF: u8 = 0x80;
const MIDI_CHANNEL_MASK: u8 = 0x0F;

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
    let mut sender = MidiSender::connect(port_index)?;
    sender.note_on(channel, note, velocity)
}

/// Send a one-shot Note Off message to the given port.
///
/// Opens a fresh connection, sends the message, then closes it. Use
/// [`MidiSender`] when you need to send multiple messages without the
/// overhead of reconnecting each time.
pub fn send_note_off(port_index: usize, channel: u8, note: u8) -> Result<(), MidiError> {
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
        self.conn
            .send(&[MIDI_NOTE_ON | (channel & MIDI_CHANNEL_MASK), note, velocity])?;
        Ok(())
    }

    /// Send a Note Off message (`0x80 | channel`, note, velocity=0).
    pub fn note_off(&mut self, channel: u8, note: u8) -> Result<(), MidiError> {
        self.conn
            .send(&[MIDI_NOTE_OFF | (channel & MIDI_CHANNEL_MASK), note, 0])?;
        Ok(())
    }
}
