//! Silence detection for automatic sample boundary trimming.

use std::path::Path;

use hound::WavReader;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SampleBounds {
    /// Inclusive start of the retained range, including block rounding and margin.
    pub start_frame: u64,
    /// Exclusive end of the retained range, including block rounding and margin.
    pub end_frame: u64,
    /// Total number of frames in the file.
    pub total_frames: u64,
}

#[derive(Debug, Error)]
pub enum SilenceDetectError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),
    #[error("empty WAV file")]
    EmptyFile,
    #[error("{0}")]
    Other(String),
}

/// Detect sample boundaries by scanning for silence from both ends.
///
/// Returns the first and last frame positions where audio exceeds
/// `threshold_db` (e.g., -60.0). Scans the entire file using RMS
/// over small blocks.
///
/// The returned bounds include a small margin (1024 frames) before
/// the first non-silent frame and after the last non-silent frame,
/// clamped to file boundaries.
pub fn detect_sample_bounds(
    wav_path: &Path,
    threshold_db: f64,
) -> Result<SampleBounds, SilenceDetectError> {
    if !threshold_db.is_finite() {
        return Err(SilenceDetectError::Other(
            "silence threshold must be finite".into(),
        ));
    }
    let mut reader = WavReader::open(wav_path)?;
    let spec = reader.spec();
    let num_channels = spec.channels as u64;
    let total_samples = reader.len() as u64;

    if total_samples == 0 || num_channels == 0 {
        return Err(SilenceDetectError::EmptyFile);
    }

    let total_frames = total_samples / num_channels;
    if total_frames == 0 {
        return Err(SilenceDetectError::EmptyFile);
    }

    // Convert threshold from dB to linear amplitude
    let threshold_linear = 10.0_f64.powf(threshold_db / 20.0);

    // Read all samples as f64
    let samples: Vec<f64> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = match spec.bits_per_sample {
                16 => i16::MAX as f64 + 1.0,
                24 => (1i64 << 23) as f64,
                32 => i32::MAX as f64 + 1.0,
                other => {
                    return Err(SilenceDetectError::Other(format!(
                        "Unsupported bit depth: {other}"
                    )));
                }
            };
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f64 / max_val))
                .collect::<Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.map(|v| v as f64))
            .collect::<Result<Vec<_>, _>>()?,
    };

    let channels = num_channels as usize;
    let block_size = 512;
    let margin = 1024;
    let complete_samples = &samples[..total_frames as usize * channels];
    let mut audible = complete_samples
        .chunks(block_size * channels)
        .enumerate()
        .filter(|(_, block)| block_rms(block) > threshold_linear)
        .map(|(index, block)| {
            let start = (index * block_size) as u64;
            (start, start + (block.len() / channels) as u64)
        });

    let Some(first) = audible.next() else {
        // Entire file is silence: retain the full range.
        return Ok(SampleBounds {
            start_frame: 0,
            end_frame: total_frames,
            total_frames,
        });
    };
    let last = audible.next_back().unwrap_or(first);
    Ok(SampleBounds {
        start_frame: first.0.saturating_sub(margin),
        end_frame: (last.1 + margin).min(total_frames),
        total_frames,
    })
}

fn block_rms(samples: &[f64]) -> f64 {
    let sum_sq: f64 = samples.iter().map(|&sample| sample * sample).sum();
    (sum_sq / samples.len() as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};
    use tempfile::tempdir;

    fn write_test_wav(path: &Path, samples: &[f32], sample_rate: u32) {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn detect_bounds_with_silence_padding() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wav");

        // 1000 frames silence, 2000 frames tone, 1000 frames silence
        let mut samples = vec![0.0f32; 1000];
        samples.extend(vec![0.5f32; 2000]); // loud section
        samples.extend(vec![0.0f32; 1000]);

        write_test_wav(&path, &samples, 48000);

        let bounds = detect_sample_bounds(&path, -60.0).unwrap();
        assert_eq!(bounds.total_frames, 4000);
        // Start should be near 0 (1000 - 1024 margin, clamped to 0)
        assert!(bounds.start_frame == 0);
        // End should be near 4000 (3000 + 1024, clamped to 4000)
        assert!(bounds.end_frame == 4000);
    }

    #[test]
    fn detect_bounds_longer_silence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wav");

        // 5000 frames silence, 2000 frames tone, 5000 frames silence
        let mut samples = vec![0.0f32; 5000];
        samples.extend(vec![0.5f32; 2000]);
        samples.extend(vec![0.0f32; 5000]);

        write_test_wav(&path, &samples, 48000);

        let bounds = detect_sample_bounds(&path, -60.0).unwrap();
        assert_eq!(bounds.total_frames, 12000);
        // Start: block containing frame 5000 starts at 4608 (block 9 × 512),
        // minus 1024 margin = 3584. Must be strictly less than 5000.
        assert!(bounds.start_frame < 5000);
        assert!(bounds.start_frame >= 3500); // 4608 - 1024 = 3584
        // End: block containing frame 7000 ends at 7168 (block 14 × 512),
        // plus 1024 margin = 8192. Must be strictly greater than 7000.
        assert!(bounds.end_frame > 7000);
        assert!(bounds.end_frame <= 8200); // 7168 + 1024 = 8192
    }

    #[test]
    fn detect_bounds_all_silence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wav");

        let samples = vec![0.0f32; 4000];
        write_test_wav(&path, &samples, 48000);

        let bounds = detect_sample_bounds(&path, -60.0).unwrap();
        assert_eq!(bounds.start_frame, 0);
        assert_eq!(bounds.end_frame, 4000);
    }

    #[test]
    fn nonfinite_threshold_is_rejected() {
        for threshold in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                detect_sample_bounds(Path::new("does-not-exist.wav"), threshold),
                Err(SilenceDetectError::Other(_))
            ));
        }
    }

    #[test]
    fn stereo_activity_and_partial_last_block_use_frame_bounds() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stereo.wav");
        let spec = WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        // 10,123 frames: signal only on the right, reaching a partial final block.
        for frame in 0..10123 {
            writer.write_sample(0.0_f32).unwrap();
            writer
                .write_sample(if frame >= 9000 { 0.5_f32 } else { 0.0 })
                .unwrap();
        }
        writer.finalize().unwrap();
        let bounds = detect_sample_bounds(&path, -60.0).unwrap();
        assert_eq!(bounds.start_frame, 7680); // block 17 starts at 8704, minus 1024
        assert_eq!(bounds.end_frame, 10123);
        assert_eq!(bounds.total_frames, 10123);
    }

    #[test]
    fn detect_bounds_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wav");

        let samples: Vec<f32> = vec![];
        write_test_wav(&path, &samples, 48000);

        let result = detect_sample_bounds(&path, -60.0);
        assert!(result.is_err());
    }
}
