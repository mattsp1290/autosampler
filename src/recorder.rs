//! Hardware synth recorder: coordinates MIDI output + audio input capture.
//!
//! Records one note at a time: opens a cpal input stream, sends MIDI note-on,
//! records audio for a configured duration plus a silence-detected tail, sends
//! MIDI note-off, and saves the result as a WAV file.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::BitDepth;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{
    capture::{Capture, SilenceTracker},
    midi_output::{MidiError, MidiSender, note_on_message, validate_channel},
    wav::{WavError, write_wav},
};

/// Errors from hardware recording operations.
#[derive(Debug, thiserror::Error)]
pub enum HardwareRecorderError {
    #[error("MIDI error: {0}")]
    Midi(#[from] MidiError),

    #[error("audio device not found: {0}")]
    DeviceNotFound(String),

    #[error("audio stream error: {0}")]
    StreamError(String),

    #[error("WAV write error: {0}")]
    WavError(#[from] WavError),

    #[error("recording produced no audio")]
    EmptyRecording,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<cpal::BuildStreamError> for HardwareRecorderError {
    fn from(e: cpal::BuildStreamError) -> Self {
        HardwareRecorderError::StreamError(e.to_string())
    }
}

impl From<cpal::PlayStreamError> for HardwareRecorderError {
    fn from(e: cpal::PlayStreamError) -> Self {
        HardwareRecorderError::StreamError(e.to_string())
    }
}

/// Coordinates MIDI output and audio input for hardware synth sampling.
///
/// Each `record_note` call opens a fresh cpal input stream, fires a MIDI
/// note-on, captures audio until silence is detected (or the tail timeout
/// expires), then writes the result to a WAV file.
pub struct HardwareRecorder {
    midi_sender: MidiSender,
    audio_device_name: String,
    preferred_sample_rate: Option<u32>,
    bit_depth: BitDepth,
    midi_channel: u8,
}

impl HardwareRecorder {
    /// Create a new `HardwareRecorder`.
    ///
    /// Opens a persistent MIDI output connection to `midi_port_index`.
    /// The audio input stream is opened fresh for each `record_note` call.
    pub fn new(
        midi_port_index: usize,
        audio_device_name: &str,
        preferred_sample_rate: Option<u32>,
        bit_depth: BitDepth,
        midi_channel: u8,
    ) -> Result<Self, HardwareRecorderError> {
        validate_channel(midi_channel)?;
        let midi_sender = MidiSender::connect(midi_port_index)?;
        Ok(Self {
            midi_sender,
            audio_device_name: audio_device_name.to_string(),
            preferred_sample_rate,
            bit_depth,
            midi_channel,
        })
    }

    /// Record a single note.
    ///
    /// Sends MIDI note-on, records audio for `duration`, sends MIDI
    /// note-off, then waits up to `tail_timeout` for silence before
    /// saving the captured audio to `wav_path`.
    ///
    /// If `cancel` is provided and becomes `true`, the recording exits
    /// early (during the note hold or silence tail) and still saves
    /// whatever audio was captured.
    pub fn record_note(
        &mut self,
        note: u8,
        velocity: u8,
        duration: Duration,
        tail_timeout: Duration,
        wav_path: &Path,
    ) -> Result<(), HardwareRecorderError> {
        self.record_note_cancellable(note, velocity, duration, tail_timeout, wav_path, None)
    }

    /// Record a single note with optional cancellation support.
    pub fn record_note_cancellable(
        &mut self,
        note: u8,
        velocity: u8,
        duration: Duration,
        tail_timeout: Duration,
        wav_path: &Path,
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<(), HardwareRecorderError> {
        note_on_message(self.midi_channel, note, velocity)?;
        // 1. Find the cpal input device by name.
        let device = self.find_input_device()?;

        // 2. Negotiate a compatible stream configuration for this device.
        let config =
            crate::audio_input::negotiate_stream_config(&device, self.preferred_sample_rate)
                .map_err(|e| HardwareRecorderError::StreamError(e.to_string()))?;
        let actual_channels = config.channels;
        let actual_sample_rate = config.sample_rate;
        tracing::debug!(
            channels = actual_channels,
            sample_rate = actual_sample_rate,
            note,
            "negotiated stream config for recording"
        );

        // 3. Shared capture storage, written only by the callback until stop.
        //    Pre-allocate based on expected recording duration to reduce reallocations.
        let capacity = ((duration + tail_timeout).as_secs_f64()
            * actual_sample_rate as f64
            * actual_channels as f64) as usize;
        let capture = Arc::new(Capture::new(capacity));
        let capture_cb = capture.clone();
        let errors_cb = capture.clone();
        let mut tracker = SilenceTracker::new(actual_channels, actual_sample_rate);

        // 4. Build the input stream.
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                capture_cb.push(data, &mut tracker);
            },
            move |err| {
                errors_cb.fail_stream();
                tracing::error!(%err, "audio input stream failed");
            },
            None,
        )?;

        // 5. Start recording.
        stream.play()?;

        // 6. Send MIDI note-on.
        self.midi_sender
            .note_on(self.midi_channel, note, velocity)?;

        wait_for_hold(duration, cancel, &capture);
        // Release the note before capturing its tail. If sending fails,
        // dropping the stream stops capture and propagates the MIDI error.
        self.midi_sender.note_off(self.midi_channel, note)?;
        capture.start_release();
        self.record_and_save(stream, &capture, &config, tail_timeout, wav_path, cancel)
    }

    /// Record a set of preview notes.
    ///
    /// For each `(note, velocity)` pair in `notes`, records a WAV file named
    /// `<NoteName>_v<velocity>.wav` inside `output_dir` and returns the
    /// collected paths.
    pub fn record_preview_notes(
        &mut self,
        notes: &[(u8, u8)],
        duration: Duration,
        tail_timeout: Duration,
        output_dir: &Path,
    ) -> Result<Vec<PathBuf>, HardwareRecorderError> {
        let mut paths = Vec::with_capacity(notes.len());

        for &(note, velocity) in notes {
            let filename = sample_filename(note, velocity);
            let wav_path = output_dir.join(&filename);

            self.record_note(note, velocity, duration, tail_timeout, &wav_path)?;

            paths.push(wav_path);
        }

        Ok(paths)
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Capture the release tail after note-off, stop the stream, and save WAV.
    fn record_and_save(
        &self,
        stream: cpal::Stream,
        capture: &Capture,
        config: &cpal::StreamConfig,
        tail_timeout: Duration,
        wav_path: &Path,
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<(), HardwareRecorderError> {
        let is_cancelled = || cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed));

        // Observe callback-owned statistics without locking audio storage.
        let tail_started = Instant::now();
        loop {
            let remaining = tail_timeout.saturating_sub(tail_started.elapsed());
            if is_cancelled()
                || remaining.is_zero()
                || capture.check_error().is_err()
                || capture.release_is_silent(config.sample_rate)
            {
                break;
            }
            std::thread::sleep(remaining.min(Duration::from_millis(100)));
        }

        drop(stream);
        let captured = capture.finish()?;

        if captured.is_empty() {
            return Err(HardwareRecorderError::EmptyRecording);
        }

        // Truncate to a whole number of frames to prevent panics from
        // partial callback buffers delivered right before stream drop.
        let ch = config.channels as usize;
        let frame_count = captured.len() / ch;
        let aligned = &captured[..frame_count * ch];

        let mut planar = vec![Vec::with_capacity(frame_count); ch];
        for frame in aligned.chunks_exact(ch) {
            for (channel, &sample) in planar.iter_mut().zip(frame) {
                channel.push(sample);
            }
        }
        write_wav(&planar, config.sample_rate, wav_path, self.bit_depth)?;

        Ok(())
    }

    /// Find the cpal input device whose name matches `self.audio_device_name`.
    #[allow(deprecated)] // cpal 0.17 deprecated name() in favour of description()
    fn find_input_device(&self) -> Result<cpal::Device, HardwareRecorderError> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| HardwareRecorderError::StreamError(e.to_string()))?;

        for device in devices {
            let name = device.name().unwrap_or_default();
            if name == self.audio_device_name {
                return Ok(device);
            }
        }

        Err(HardwareRecorderError::DeviceNotFound(
            self.audio_device_name.clone(),
        ))
    }
}

// -----------------------------------------------------------------------------
// Free helpers
// -----------------------------------------------------------------------------

fn wait_for_hold(
    duration: Duration,
    cancel: Option<&std::sync::atomic::AtomicBool>,
    capture: &Capture,
) {
    let start = Instant::now();
    while !cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
        && capture.check_error().is_ok()
    {
        let remaining = duration.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            break;
        }
        std::thread::sleep(remaining.min(Duration::from_millis(100)));
    }
}

fn note_number_to_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (note / 12) as i8 - 1;
    let name = NAMES[(note % 12) as usize];
    format!("{name}{octave}")
}

fn sample_filename(note: u8, velocity: u8) -> String {
    format!("{}_v{velocity}.wav", note_number_to_name(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_filename_matches_rendered_sample_pattern() {
        assert_eq!(sample_filename(60, 127), "C4_v127.wav");
    }

    #[test]
    fn note_names_spot_check() {
        assert_eq!(note_number_to_name(60), "C4");
        assert_eq!(note_number_to_name(69), "A4");
        assert_eq!(note_number_to_name(0), "C-1");
        assert_eq!(note_number_to_name(127), "G9");
        assert_eq!(note_number_to_name(61), "C#4");
    }

    #[test]
    fn invalid_channel_fails_before_connecting() {
        assert!(matches!(
            HardwareRecorder::new(usize::MAX, "unused", None, BitDepth::Int24, 16),
            Err(HardwareRecorderError::Midi(MidiError::InvalidValue {
                field: "channel",
                value: 16
            }))
        ));
    }
}
