# autosampler

Send MIDI notes to hardware instruments, record an audio input to WAV, and find
silence boundaries for trimming. Extracted from `multisamples` revision
`a2207788282cf943d9d12cfeab63b4897af808b2`, without its application dependencies.
MIT licensed. The API is unstable and may change before 1.0.

```rust,no_run
use std::{path::Path, time::Duration};
use autosampler::{BitDepth, audio_input, recorder::HardwareRecorder, silence_detect};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let devices = audio_input::list_input_devices()?;
let input = devices.first().ok_or("no audio input device")?;
// MIDI port index 0; MIDI channel 0 means channel 1 on the instrument.
let mut recorder = HardwareRecorder::new(0, &input.name, Some(48_000), BitDepth::Int24, 0)?;
let path = Path::new("C4_v127.wav");
recorder.record_note(60, 127, Duration::from_secs(1), Duration::from_secs(3), path)?;
let bounds = silence_detect::detect_sample_bounds(path, -60.0)?;
println!("keep frames {}..{}", bounds.start_frame, bounds.end_frame);
# Ok(())
# }
```

Run `cargo run --example list_devices` to find audio inputs and MIDI port indices.
Names are used as audio device identifiers; duplicate names may be ambiguous.
The recorder requires an input configuration with f32 samples, prefers stereo,
and falls back to a supported sample rate. It writes 16-bit PCM, 24-bit PCM, or
32-bit float WAV. MIDI notes and velocities are 0–127; channels are 0–15.

`record_note_cancellable` accepts an atomic cancellation flag. Cancellation
releases the note and saves whatever was captured. `record_preview_notes` writes
`<NoteName>_v<velocity>.wav` paths; create the output directory first. These calls
block the calling thread. MIDI input callbacks execute on the MIDI thread and
must return promptly without panicking.

Silence detection returns frame bounds with a 1024-frame margin; `end_frame` is
exclusive. It does not rewrite or trim the WAV. An entirely silent file returns
the full range. Apply the bounds in your own editor or export pipeline.

Enable the optional `serde` feature for serializable device, MIDI event, and
bit-depth types. There are no default features or dependencies on the original
application.

Rust 1.93 or newer is required. macOS is used by the author; Linux builds are
configured in CI with `libasound2-dev`. Windows is not tested. CI runs build,
tests, clippy, formatting, and documentation on macOS and Linux, with and without
serde. Hardware tests are ignored because CI has no audio/MIDI devices.

```sh
cargo build --all-targets
cargo test
cargo test --features serde
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

Git tags are the release mechanism; crates.io publication is outside this plan.
See [the publication audit](docs/publication-audit.md) for import and dependency
license evidence.
