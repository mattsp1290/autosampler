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
    let ranges: Vec<_> = match device.supported_input_configs() {
        Ok(iter) => iter
            .filter(|r| r.sample_format() == cpal::SampleFormat::F32)
            .collect(),
        Err(_) => {
            // Can't enumerate — fall back to the device default.
            let default = device
                .default_input_config()
                .map_err(|e| HardwareAudioError::DeviceEnumeration(e.to_string()))?;
            if default.sample_format() != cpal::SampleFormat::F32 {
                return Err(HardwareAudioError::NoSupportedConfig);
            }
            return Ok(default.into());
        }
    };

    if ranges.is_empty() {
        // No supported configs reported — try the default.
        let default = device
            .default_input_config()
            .map_err(|e| HardwareAudioError::DeviceEnumeration(e.to_string()))?;
        if default.sample_format() != cpal::SampleFormat::F32 {
            return Err(HardwareAudioError::NoSupportedConfig);
        }
        return Ok(default.into());
    }

    // Partition into stereo and non-stereo, preferring stereo.
    let (stereo, other): (Vec<_>, Vec<_>) = ranges.into_iter().partition(|r| r.channels() == 2);

    let candidates = if !stereo.is_empty() { stereo } else { other };

    // Build the list of rates to try.
    let rates_to_try: Vec<u32> = if let Some(pref) = preferred_sample_rate {
        // Preferred first, then fallback order (excluding duplicate).
        let mut rates = vec![pref];
        rates.extend(PREFERRED_RATES.iter().filter(|&&r| r != pref));
        rates
    } else {
        PREFERRED_RATES.to_vec()
    };

    // Try each rate against each candidate range.
    for &rate in &rates_to_try {
        for range in &candidates {
            if let Some(supported) = (*range).try_with_sample_rate(rate) {
                let cfg: cpal::StreamConfig = supported.into();
                tracing::debug!(
                    sample_rate = cfg.sample_rate,
                    channels = cfg.channels,
                    "negotiated audio input config"
                );
                return Ok(cfg);
            }
        }
    }

    // Last resort: use max sample rate of the first candidate.
    if let Some(range) = candidates.into_iter().next() {
        let cfg: cpal::StreamConfig = range.with_max_sample_rate().into();
        tracing::debug!(
            sample_rate = cfg.sample_rate,
            channels = cfg.channels,
            "negotiated audio input config (last resort max rate)"
        );
        return Ok(cfg);
    }

    Err(HardwareAudioError::NoSupportedConfig)
}

/// Supported audio configurations for a device, suitable for display.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AudioDeviceConfig {
    /// Sample rates the device supports (from the set of well-known rates).
    pub supported_sample_rates: Vec<u32>,
    /// Preferred channel count (2 if stereo available, else 1).
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

    // Collect supported sample rates from well-known set.
    let mut supported_rates: Vec<u32> = Vec::new();
    for &rate in &KNOWN_RATES {
        for range in &ranges {
            if range.min_sample_rate() <= rate && rate <= range.max_sample_rate() {
                supported_rates.push(rate);
                break;
            }
        }
    }

    // Negotiate once up front — reused for both early-return and normal paths.
    let negotiated = negotiate_stream_config(&device, None)?;
    let default_sample_rate = negotiated.sample_rate;

    if ranges.is_empty() {
        return Ok(AudioDeviceConfig {
            supported_sample_rates: vec![],
            channels: negotiated.channels,
            default_sample_rate,
        });
    }

    // Determine channels.
    let has_stereo = ranges.iter().any(|r| r.channels() == 2);
    let channels = if has_stereo { 2 } else { ranges[0].channels() };

    Ok(AudioDeviceConfig {
        supported_sample_rates: supported_rates,
        channels,
        default_sample_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
