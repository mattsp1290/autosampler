//! Audio input device enumeration using cpal.

use cpal::traits::{DeviceTrait, HostTrait};

/// A discovered audio input device.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AudioInputDevice {
    /// Human-readable device name.
    pub name: String,
    /// Stable-ish identifier (device name used as ID since cpal does not
    /// expose persistent hardware IDs on all platforms).
    pub id: String,
}

/// Errors that can occur during hardware audio enumeration.
#[derive(Debug, thiserror::Error)]
pub enum HardwareAudioError {
    #[error("failed to enumerate input devices: {0}")]
    DeviceEnumeration(String),

    #[error("failed to query device name: {0}")]
    DeviceName(String),

    #[error("audio device not found: {0}")]
    DeviceNotFound(String),

    #[error("no supported input configuration found for device")]
    NoSupportedConfig,
}

/// List all available audio input devices on the default host.
#[allow(deprecated)] // cpal 0.17 deprecates name() in favour of description(), but description() returns DeviceDescription not String
pub fn list_input_devices() -> Result<Vec<AudioInputDevice>, HardwareAudioError> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| HardwareAudioError::DeviceEnumeration(e.to_string()))?;

    let mut result = Vec::new();
    for device in devices {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let id = name.clone();
        result.push(AudioInputDevice { name, id });
    }

    Ok(result)
}

/// Return the default audio input device, or `None` if none is available.
#[allow(deprecated)]
pub fn get_default_input_device() -> Option<AudioInputDevice> {
    let host = cpal::default_host();
    let device = host.default_input_device()?;
    let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
    let id = name.clone();
    Some(AudioInputDevice { name, id })
}

/// Preferred sample rates to try, in priority order.
/// 48000 first because USB audio interfaces for hardware synths commonly use it.
const PREFERRED_RATES: [u32; 4] = [48000, 44100, 96000, 88200];

/// Well-known sample rates to report in device capability queries.
const KNOWN_RATES: [u32; 6] = [44100, 48000, 88200, 96000, 176400, 192000];

/// Negotiate a compatible `StreamConfig` for the given input device.
///
/// If `preferred_sample_rate` is `Some(rate)`, tries that rate first.
/// Otherwise tries rates in priority order (48000, 44100, 96000, 88200).
/// Requires f32 samples and prefers stereo over mono. Falls back to the device's default config
/// if supported configs can't be enumerated.
pub fn negotiate_stream_config(
    device: &cpal::Device,
    preferred_sample_rate: Option<u32>,
) -> Result<cpal::StreamConfig, HardwareAudioError> {
    let ranges: Vec<_> = device
        .supported_input_configs()
        .map(|iter| iter.collect())
        .unwrap_or_default();
    if let Some(config) = select_stream_config(&ranges, preferred_sample_rate) {
        return Ok(config);
    }
    let default = device
        .default_input_config()
        .map_err(|e| HardwareAudioError::DeviceEnumeration(e.to_string()))?;
    if default.sample_format() != cpal::SampleFormat::F32 || default.channels() == 0 {
        return Err(HardwareAudioError::NoSupportedConfig);
    }
    Ok(default.into())
}

fn select_stream_config(
    ranges: &[cpal::SupportedStreamConfigRange],
    preferred_sample_rate: Option<u32>,
) -> Option<cpal::StreamConfig> {
    let usable: Vec<_> = ranges
        .iter()
        .filter(|r| r.sample_format() == cpal::SampleFormat::F32 && r.channels() > 0)
        .collect();
    let prefer_stereo = usable.iter().any(|r| r.channels() == 2);
    let candidates: Vec<_> = usable
        .into_iter()
        .filter(|r| !prefer_stereo || r.channels() == 2)
        .collect();
    let rates = preferred_sample_rate.into_iter().chain(PREFERRED_RATES);
    for rate in rates {
        for range in &candidates {
            if let Some(config) = (**range).try_with_sample_rate(rate) {
                return Some(config.into());
            }
        }
    }
    candidates
        .first()
        .map(|range| (**range).with_max_sample_rate().into())
}

fn capabilities(
    ranges: &[cpal::SupportedStreamConfigRange],
    negotiated: &cpal::StreamConfig,
) -> AudioDeviceConfig {
    let supported_sample_rates = KNOWN_RATES
        .into_iter()
        .filter(|&rate| {
            ranges.iter().any(|range| {
                range.sample_format() == cpal::SampleFormat::F32
                    && range.channels() == negotiated.channels
                    && range.min_sample_rate() <= rate
                    && rate <= range.max_sample_rate()
            })
        })
        .collect();
    AudioDeviceConfig {
        supported_sample_rates,
        channels: negotiated.channels,
        default_sample_rate: negotiated.sample_rate,
    }
}

/// Supported audio configurations for a device, suitable for display.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AudioDeviceConfig {
    /// Well-known rates supported with f32 samples at the preferred channel count.
    pub supported_sample_rates: Vec<u32>,
    /// Negotiated channel count (stereo preferred among f32 configurations).
    pub channels: u16,
    /// The rate that auto-negotiation would select.
    pub default_sample_rate: u32,
}

/// Query the supported input configurations for a named audio device.
#[allow(deprecated)]
pub fn query_device_config(device_name: &str) -> Result<AudioDeviceConfig, HardwareAudioError> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let device = host
        .input_devices()
        .map_err(|e| HardwareAudioError::DeviceEnumeration(e.to_string()))?
        .find(|d| d.name().unwrap_or_default() == device_name)
        .ok_or_else(|| HardwareAudioError::DeviceNotFound(device_name.to_string()))?;

    let ranges: Vec<_> = device
        .supported_input_configs()
        .map_err(|e| HardwareAudioError::DeviceEnumeration(e.to_string()))?
        .collect();

    let negotiated = negotiate_stream_config(&device, None)?;
    Ok(capabilities(&ranges, &negotiated))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(
        channels: u16,
        rate: u32,
        format: cpal::SampleFormat,
    ) -> cpal::SupportedStreamConfigRange {
        cpal::SupportedStreamConfigRange::new(
            channels,
            rate,
            rate,
            cpal::SupportedBufferSize::Unknown,
            format,
        )
    }

    #[test]
    fn capabilities_match_mixed_format_and_channel_negotiation() {
        let ranges = [
            range(2, 96000, cpal::SampleFormat::I16),
            range(1, 48000, cpal::SampleFormat::F32),
        ];
        let config = select_stream_config(&ranges, Some(96000)).unwrap();
        let caps = capabilities(&ranges, &config);
        assert_eq!(caps.channels, 1);
        assert_eq!(caps.default_sample_rate, 48000);
        assert_eq!(caps.supported_sample_rates, [48000]);

        let ranges = [
            range(1, 96000, cpal::SampleFormat::F32),
            range(2, 44100, cpal::SampleFormat::F32),
        ];
        let config = select_stream_config(&ranges, Some(96000)).unwrap();
        let caps = capabilities(&ranges, &config);
        assert_eq!(caps.channels, 2);
        assert_eq!(caps.default_sample_rate, 44100);
        assert_eq!(caps.supported_sample_rates, [44100]);
        assert!(select_stream_config(&[range(2, 48000, cpal::SampleFormat::I16)], None).is_none());
    }

    #[test]
    #[ignore = "requires a working audio host; enumeration can hang in headless environments"]
    fn list_input_devices_returns_ok() {
        // Enumeration should not error even if no devices are present.
        let result = list_input_devices();
        assert!(result.is_ok(), "list_input_devices failed: {:?}", result);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn audio_input_device_is_serializable() {
        let dev = AudioInputDevice {
            name: "Test Mic".to_string(),
            id: "Test Mic".to_string(),
        };
        let json = serde_json::to_string(&dev).expect("serialize failed");
        let back: AudioInputDevice = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(back.name, dev.name);
        assert_eq!(back.id, dev.id);
    }
}
