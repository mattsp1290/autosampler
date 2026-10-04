use std::path::Path;

use crate::BitDepth;
use hound::{SampleFormat, WavSpec, WavWriter};

/// Error type for WAV operations.
#[derive(Debug, thiserror::Error)]
pub enum WavError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("hound error: {0}")]
    Hound(#[from] hound::Error),

    #[error("empty buffer")]
    EmptyBuffer,

    #[error("invalid channel lengths, channel count, or sample rate")]
    InvalidBuffer,
}

/// Write planar samples to a WAV file.
pub(crate) fn write_wav(
    channels: &[Vec<f32>],
    sample_rate: u32,
    path: &Path,
    bit_depth: BitDepth,
) -> Result<(), WavError> {
    if channels.is_empty() || channels[0].is_empty() {
        return Err(WavError::EmptyBuffer);
    }

    let frames = channels[0].len();
    if channels.len() > u16::MAX as usize
        || sample_rate == 0
        || channels.iter().any(|ch| ch.len() != frames)
    {
        return Err(WavError::InvalidBuffer);
    }
    let spec = WavSpec {
        channels: channels.len() as u16,
        sample_rate,
        bits_per_sample: match bit_depth {
            BitDepth::Int16 => 16,
            BitDepth::Int24 => 24,
            BitDepth::Float32 => 32,
        },
        sample_format: match bit_depth {
            BitDepth::Int16 | BitDepth::Int24 => SampleFormat::Int,
            BitDepth::Float32 => SampleFormat::Float,
        },
    };

    let mut writer = WavWriter::create(path, spec)?;

    let num_samples = frames;

    // Write interleaved samples
    for frame in 0..num_samples {
        for channel in channels {
            let sample = channel[frame];
            match bit_depth {
                BitDepth::Int16 => {
                    let val = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    writer.write_sample(val)?;
                }
                BitDepth::Int24 => {
                    let val = (sample.clamp(-1.0, 1.0) * 8_388_607.0) as i32;
                    writer.write_sample(val)?;
                }
                BitDepth::Float32 => {
                    writer.write_sample(sample)?;
                }
            }
        }
    }

    writer.finalize()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_second_stereo_sine_round_trip_at_every_depth() {
        let dir = tempfile::tempdir().unwrap();
        let rate = 48_000;
        let sine: Vec<f32> = (0..rate)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 0.8)
            .collect();
        let channels = vec![sine.clone(), sine.iter().map(|s| s * 0.5).collect()];
        for depth in [BitDepth::Int16, BitDepth::Int24, BitDepth::Float32] {
            let path = dir.path().join(format!("{depth:?}.wav"));
            write_wav(&channels, rate, &path, depth).unwrap();
            let reader = hound::WavReader::open(path).unwrap();
            let spec = reader.spec();
            assert_eq!(spec.channels, 2);
            assert_eq!(spec.sample_rate, rate);
            assert_eq!(reader.duration(), rate);
            let (bits, format) = match depth {
                BitDepth::Int16 => (16, SampleFormat::Int),
                BitDepth::Int24 => (24, SampleFormat::Int),
                BitDepth::Float32 => (32, SampleFormat::Float),
            };
            assert_eq!(spec.bits_per_sample, bits);
            assert_eq!(spec.sample_format, format);
            let samples: Vec<f32> = if format == SampleFormat::Float {
                reader.into_samples::<f32>().map(Result::unwrap).collect()
            } else {
                let scale = if bits == 16 { 32767.0 } else { 8388607.0 };
                reader
                    .into_samples::<i32>()
                    .map(|s| s.unwrap() as f32 / scale)
                    .collect()
            };
            assert_eq!(samples.len(), rate as usize * 2);
            for (frame, pair) in samples.as_chunks::<2>().0.iter().enumerate() {
                assert!((pair[0] - channels[0][frame]).abs() < 0.00004);
                assert!((pair[1] - channels[1][frame]).abs() < 0.00004);
            }
            let peak = samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
            assert!((peak - 0.8).abs() < 0.00004);
        }
    }

    #[test]
    fn rejects_invalid_buffers_before_creating_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.wav");
        for channels in [vec![], vec![vec![]], vec![vec![0.0], vec![]]] {
            assert!(write_wav(&channels, 48000, &path, BitDepth::Int24).is_err());
            assert!(!path.exists());
        }
        assert!(write_wav(&[vec![0.0]], 0, &path, BitDepth::Int24).is_err());
        assert!(!path.exists());
    }
}
