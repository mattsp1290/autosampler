#![doc = include_str!("../README.md")]

//! Send MIDI notes to hardware, capture audio inputs, and detect silence boundaries.
//!
//! See [`recorder::HardwareRecorder`] for recording and
//! [`silence_detect::detect_sample_bounds`] for trimming boundaries.

pub mod audio_input;
pub mod midi_input;
pub mod midi_output;
pub mod recorder;
pub mod silence_detect;
mod wav;

pub use wav::WavError;

/// Encoding for recorded WAV samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BitDepth {
    /// Signed 16-bit PCM.
    Int16,
    /// Signed 24-bit PCM (default).
    #[default]
    Int24,
    /// IEEE 32-bit floating point.
    Float32,
}
