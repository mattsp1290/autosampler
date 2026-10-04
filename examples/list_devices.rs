use autosampler::{audio_input, midi_input, midi_output};

fn main() {
    println!("Audio inputs:");
    match audio_input::list_input_devices() {
        Ok(devices) if devices.is_empty() => println!("  no audio input devices"),
        Ok(devices) => {
            for device in devices {
                println!("  {}", device.name);
            }
        }
        Err(error) => println!("  audio inputs unavailable: {error}"),
    }
    println!("MIDI inputs:");
    match midi_input::list_midi_input_ports() {
        Ok(ports) if ports.is_empty() => println!("  no MIDI input ports"),
        Ok(ports) => {
            for port in ports {
                println!("  {}: {}", port.index, port.name);
            }
        }
        Err(error) => println!("  MIDI inputs unavailable: {error}"),
    }
    println!("MIDI outputs:");
    match midi_output::list_midi_output_ports() {
        Ok(ports) if ports.is_empty() => println!("  no MIDI output ports"),
        Ok(ports) => {
            for port in ports {
                println!("  {}: {}", port.index, port.name);
            }
        }
        Err(error) => println!("  MIDI outputs unavailable: {error}"),
    }
}
