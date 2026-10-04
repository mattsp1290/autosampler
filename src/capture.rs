//! Private callback storage and continuous release-silence tracking.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::recorder::HardwareRecorderError;

pub(crate) struct Capture {
    samples: Mutex<Vec<f32>>,
    lost_audio: AtomicBool,
    stream_failed: AtomicBool,
    release_started: AtomicBool,
    silent_frames: AtomicU64,
}

impl Capture {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(capacity)),
            lost_audio: AtomicBool::new(false),
            stream_failed: AtomicBool::new(false),
            release_started: AtomicBool::new(false),
            silent_frames: AtomicU64::new(0),
        }
    }

    pub(crate) fn push(&self, data: &[f32], tracker: &mut SilenceTracker) {
        // Only the callback locks storage until capture stops. Never block it.
        match self.samples.try_lock() {
            Ok(mut samples) => samples.extend_from_slice(data),
            Err(_) => self.lost_audio.store(true, Ordering::Relaxed),
        }
        if self.release_started.load(Ordering::Acquire) {
            tracker.push(data);
            self.silent_frames
                .store(tracker.silent_frames, Ordering::Relaxed);
        }
    }

    pub(crate) fn fail_stream(&self) {
        self.stream_failed.store(true, Ordering::Relaxed);
    }

    pub(crate) fn start_release(&self) {
        self.release_started.store(true, Ordering::Release);
    }

    pub(crate) fn release_is_silent(&self, sample_rate: u32) -> bool {
        self.silent_frames.load(Ordering::Relaxed) >= u64::from(sample_rate).div_ceil(10) * 3
    }

    pub(crate) fn check_error(&self) -> Result<(), HardwareRecorderError> {
        if self.stream_failed.load(Ordering::Relaxed) {
            return Err(HardwareRecorderError::StreamError(
                "audio input stream failed during capture".into(),
            ));
        }
        if self.lost_audio.load(Ordering::Relaxed) {
            return Err(HardwareRecorderError::StreamError(
                "capture lost an audio callback buffer".into(),
            ));
        }
        Ok(())
    }

    /// Call only after dropping the stream; checks errors arriving during stop.
    pub(crate) fn finish(&self) -> Result<Vec<f32>, HardwareRecorderError> {
        let mut samples = self.samples.lock().map_err(|_| {
            HardwareRecorderError::StreamError("audio capture storage was poisoned".into())
        })?;
        self.check_error()?;
        Ok(std::mem::take(&mut *samples))
    }
}

/// Callback-owned 10 ms RMS blocks, spanning callback boundaries.
pub(crate) struct SilenceTracker {
    block_frames: u64,
    block_samples: usize,
    samples_in_block: usize,
    sum_sq: f64,
    silent_frames: u64,
}

impl SilenceTracker {
    pub(crate) fn new(channels: u16, sample_rate: u32) -> Self {
        let block_frames = u64::from((sample_rate / 100).max(1));
        Self {
            block_frames,
            block_samples: block_frames as usize * usize::from(channels),
            samples_in_block: 0,
            sum_sq: 0.0,
            silent_frames: 0,
        }
    }

    fn push(&mut self, data: &[f32]) {
        for &sample in data {
            self.sum_sq += f64::from(sample).powi(2);
            self.samples_in_block += 1;
            if self.samples_in_block == self.block_samples {
                if self.sum_sq / (self.block_samples as f64) < 0.001_f64.powi(2) {
                    self.silent_frames = self.silent_frames.saturating_add(self.block_frames);
                } else {
                    self.silent_frames = 0;
                }
                self.samples_in_block = 0;
                self.sum_sq = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_recording_and_final_stream_error_are_rejected() {
        let capture = Capture::new(100);
        let mut tracker = SilenceTracker::new(1, 48000);
        capture.push(&[0.5; 100], &mut tracker);
        assert!(capture.check_error().is_ok());
        // Simulate an error delivered while stopping, after the last poll.
        capture.fail_stream();
        assert!(capture.finish().is_err());
    }

    #[test]
    fn contention_rejects_recording_instead_of_hiding_a_gap() {
        let capture = Capture::new(100);
        let mut tracker = SilenceTracker::new(1, 48000);
        capture.push(&[0.5; 10], &mut tracker);
        let guard = capture.samples.lock().unwrap();
        capture.push(&[0.5; 10], &mut tracker);
        drop(guard);
        assert!(capture.check_error().is_err());
        assert!(capture.finish().is_err());
    }

    #[test]
    fn release_requires_fresh_continuous_silence_at_all_layouts() {
        for rate in [44100, 48000, 96000] {
            for channels in [1, 2] {
                let capture = Capture::new(rate as usize * channels as usize);
                let mut tracker = SilenceTracker::new(channels, rate);
                // Hold silence must not count as release silence.
                capture.push(
                    &vec![0.0; rate as usize * channels as usize / 2],
                    &mut tracker,
                );
                assert!(!capture.release_is_silent(rate));
                capture.start_release();
                let half_period = rate as usize / 20 * channels as usize;
                for _ in 0..6 {
                    capture.push(&vec![0.5; half_period], &mut tracker);
                    capture.push(&vec![0.0; half_period], &mut tracker);
                    assert!(!capture.release_is_silent(rate));
                }
                // Repeated observations without data never count stale silence.
                for _ in 0..10 {
                    assert!(!capture.release_is_silent(rate));
                }
                // Arbitrary callback boundaries exercise partial analysis blocks.
                let silence = vec![0.0; rate as usize * channels as usize / 3];
                for chunk in silence.chunks(137) {
                    capture.push(chunk, &mut tracker);
                }
                assert!(capture.release_is_silent(rate));
            }
        }
    }
}
