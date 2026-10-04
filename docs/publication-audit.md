# Publication audit — autosampler

Plan: autosampler-plan-ham7, gates L3 and L5. Prepared 2026-10-04.

Publication approval: **APPROVED** by the user on 2026-10-04 ("I approve").
Approval covers this audit, the CI enumeration timeout fix, the empty main
baseline, and the implementation branch pushes. The release remains pending
green CI on the exact revision to be tagged.

## Source revision and explicit allowlist (L3)

Source: private `multisamples`, revision
`a2207788282cf943d9d12cfeab63b4897af808b2`. The source drift check
`git log a220778..HEAD -- <imported paths>` produced no commits.
Fresh import; private history is not copied.

| Source path | Destination | Change |
| --- | --- | --- |
| crates/ms-audio/src/hardware_input.rs | src/audio_input.rs | optional serde; remove IPC derive; require f32 configuration |
| crates/ms-audio/src/hardware_recorder.rs | src/recorder.rs | own BitDepth and note naming; planar data; release MIDI before tail |
| crates/ms-audio/src/midi_input.rs | src/midi_input.rs | optional serde; remove IPC derive; own client name |
| crates/ms-audio/src/midi_output.rs | src/midi_output.rs | optional serde; remove IPC derive; own client name |
| crates/ms-audio/src/silence_detect.rs | src/silence_detect.rs | retain tests; simplify block count |
| crates/ms-audio/src/wav.rs | src/wav.rs | only writer/error; planar data; synthesized round-trip tests |
| crates/ms-core/src/audio.rs | src/lib.rs | only BitDepth variants/default |
| crates/ms-core/src/midi.rs | src/recorder.rs | only private note-number naming function |

All other files are newly authored scaffold, documentation, or synthesized tests.
No private source directories were copied wholesale. No plugin resources,
presets, binaries, recordings, disassembly, database, app, or investigation files
were imported. The optional Syntakt integration test stays in the private
repository because it imports application-specific types and requires hardware.

Commercial product string scan of `src/` and `examples/`:
`rg -n -i 'serum|omnisphere|kontakt|vital|chipsynth|aria|spire|addictive|manis' src examples`
returned no matches. There is no `tests/` directory; tests are in the imported
modules and private WAV writer. Every audio fixture is synthesized in test code.

Full-history exclusion scan:
`git log --all --name-only --format=` is checked before publication against
`serum2_uidesc|\.fxp$|\.nki$|\.vital$|\.vstpreset$|\.vst3/` and the
plan's excluded source directories. No excluded path is in the local history.

## Secrets, paths, and application coupling (L5)

Scans of authored and imported files found no personal absolute path, private
email, credential value, or application import. `src/` and `examples/` contain
no `ms_core`, `ms_audio`, or `specta` reference.

Justified provenance matches for `multisamples`: README, this audit, and the
import commit message identify the source. References in review artifacts are
local evidence and excluded from the crate package. Git's local metadata and
build output are not imported or packaged. The Cargo repository URL is the
intended public destination, not a private path or credential.

`cargo tree -e normal` contains no `ms-*`, `lotel-*`, `tauri`, or `specta` crate.
The locked dependencies are crates.io releases; there are no git or path
dependencies. `cargo package --list` is checked on the committed tree and includes
only Cargo metadata/lockfile, LICENSE, README, this audit, source modules, and
the list_devices example. CI and review artifacts are not packaged.

### Dependency licenses

`cargo license --json` succeeded. The following table covers all locked targets,
including development dependencies. MIT is selected wherever offered; otherwise
Apache-2.0 and Unicode-3.0 permissive terms apply. The r-efi expression includes
LGPL as an alternative; MIT is selected, so no copyleft license is required.
Dependency notices/licenses remain with the dependencies; this crate's MIT
license does not replace them.

| Dependency | Version | Declared license |
| --- | --- | --- |
| alsa | 0.9.1 | Apache-2.0 OR MIT |
| alsa | 0.10.0 | Apache-2.0 OR MIT |
| alsa-sys | 0.3.1 | MIT |
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| autosampler | 0.1.0 | MIT |
| bitflags | 1.3.2 | Apache-2.0 OR MIT |
| bitflags | 2.13.2 | Apache-2.0 OR MIT |
| block | 0.1.6 | MIT |
| block2 | 0.6.2 | MIT |
| bumpalo | 3.20.3 | Apache-2.0 OR MIT |
| bytes | 1.12.1 | MIT |
| cesu8 | 1.1.0 | Apache-2.0 OR MIT |
| cfg-if | 1.0.5 | Apache-2.0 OR MIT |
| combine | 4.6.8 | MIT |
| core-foundation | 0.9.4 | Apache-2.0 OR MIT |
| core-foundation-sys | 0.8.7 | Apache-2.0 OR MIT |
| coreaudio-rs | 0.13.0 | Apache-2.0 OR MIT |
| coremidi | 0.8.0 | MIT |
| coremidi-sys | 3.2.1 | MIT |
| cpal | 0.17.1 | Apache-2.0 |
| dasp_sample | 0.11.0 | Apache-2.0 OR MIT |
| dispatch2 | 0.3.1 | Apache-2.0 OR MIT OR Zlib |
| equivalent | 1.0.2 | Apache-2.0 OR MIT |
| errno | 0.3.14 | Apache-2.0 OR MIT |
| fastrand | 2.5.0 | Apache-2.0 OR MIT |
| futures-core | 0.3.34 | Apache-2.0 OR MIT |
| futures-task | 0.3.34 | Apache-2.0 OR MIT |
| futures-util | 0.3.34 | Apache-2.0 OR MIT |
| getrandom | 0.4.3 | Apache-2.0 OR MIT |
| hashbrown | 0.17.1 | Apache-2.0 OR MIT |
| hound | 3.5.1 | Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| itoa | 1.0.18 | Apache-2.0 OR MIT |
| jni | 0.21.1 | Apache-2.0 OR MIT |
| jni-sys | 0.3.1 | Apache-2.0 OR MIT |
| jni-sys | 0.4.1 | Apache-2.0 OR MIT |
| jni-sys-macros | 0.4.1 | Apache-2.0 OR MIT |
| js-sys | 0.3.106 | Apache-2.0 OR MIT |
| libc | 0.2.190 | Apache-2.0 OR MIT |
| linux-raw-sys | 0.12.1 | Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT |
| lock_api | 0.4.14 | Apache-2.0 OR MIT |
| log | 0.4.34 | Apache-2.0 OR MIT |
| mach2 | 0.5.0 | Apache-2.0 OR BSD-2-Clause OR MIT |
| memchr | 2.8.3 | MIT OR Unlicense |
| midir | 0.10.3 | MIT |
| ndk | 0.9.0 | Apache-2.0 OR MIT |
| ndk-context | 0.1.1 | Apache-2.0 OR MIT |
| ndk-sys | 0.6.0+11769913 | Apache-2.0 OR MIT |
| num-derive | 0.4.2 | Apache-2.0 OR MIT |
| num-traits | 0.2.19 | Apache-2.0 OR MIT |
| num_enum | 0.7.6 | Apache-2.0 OR BSD-3-Clause OR MIT |
| num_enum_derive | 0.7.6 | Apache-2.0 OR BSD-3-Clause OR MIT |
| objc2 | 0.6.4 | MIT |
| objc2-audio-toolbox | 0.3.2 | Apache-2.0 OR MIT OR Zlib |
| objc2-avf-audio | 0.3.2 | Apache-2.0 OR MIT OR Zlib |
| objc2-core-audio | 0.3.2 | Apache-2.0 OR MIT OR Zlib |
| objc2-core-audio-types | 0.3.2 | Apache-2.0 OR MIT OR Zlib |
| objc2-core-foundation | 0.3.2 | Apache-2.0 OR MIT OR Zlib |
| objc2-encode | 4.1.0 | MIT |
| objc2-foundation | 0.3.2 | MIT |
| once_cell | 1.21.4 | Apache-2.0 OR MIT |
| parking_lot | 0.12.5 | Apache-2.0 OR MIT |
| parking_lot_core | 0.9.12 | Apache-2.0 OR MIT |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| pkg-config | 0.3.34 | Apache-2.0 OR MIT |
| proc-macro-crate | 3.5.0 | Apache-2.0 OR MIT |
| proc-macro2 | 1.0.107 | Apache-2.0 OR MIT |
| quote | 1.0.47 | Apache-2.0 OR MIT |
| r-efi | 6.0.0 | Apache-2.0 OR LGPL-2.1-or-later OR MIT |
| redox_syscall | 0.5.18 | MIT |
| rustix | 1.1.5 | Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT |
| rustversion | 1.0.23 | Apache-2.0 OR MIT |
| same-file | 1.0.6 | MIT OR Unlicense |
| scopeguard | 1.2.0 | Apache-2.0 OR MIT |
| serde | 1.0.228 | Apache-2.0 OR MIT |
| serde_core | 1.0.228 | Apache-2.0 OR MIT |
| serde_derive | 1.0.228 | Apache-2.0 OR MIT |
| serde_json | 1.0.151 | Apache-2.0 OR MIT |
| slab | 0.4.12 | MIT |
| smallvec | 1.16.2 | Apache-2.0 OR MIT |
| syn | 2.0.119 | Apache-2.0 OR MIT |
| syn | 3.0.6 | Apache-2.0 OR MIT |
| tempfile | 3.27.0 | Apache-2.0 OR MIT |
| thiserror | 1.0.69 | Apache-2.0 OR MIT |
| thiserror | 2.0.18 | Apache-2.0 OR MIT |
| thiserror-impl | 1.0.69 | Apache-2.0 OR MIT |
| thiserror-impl | 2.0.18 | Apache-2.0 OR MIT |
| tokio | 1.53.2 | MIT |
| toml_datetime | 1.1.1+spec-1.1.0 | Apache-2.0 OR MIT |
| toml_edit | 0.25.15+spec-1.1.0 | Apache-2.0 OR MIT |
| toml_parser | 1.1.3+spec-1.1.0 | Apache-2.0 OR MIT |
| tracing | 0.1.44 | MIT |
| tracing-attributes | 0.1.31 | MIT |
| tracing-core | 0.1.36 | MIT |
| unicode-ident | 1.0.26 | (Apache-2.0 OR MIT) AND Unicode-3.0 |
| walkdir | 2.5.0 | MIT OR Unlicense |
| wasm-bindgen | 0.2.129 | Apache-2.0 OR MIT |
| wasm-bindgen-futures | 0.4.79 | Apache-2.0 OR MIT |
| wasm-bindgen-macro | 0.2.129 | Apache-2.0 OR MIT |
| wasm-bindgen-macro-support | 0.2.129 | Apache-2.0 OR MIT |
| wasm-bindgen-shared | 0.2.129 | Apache-2.0 OR MIT |
| web-sys | 0.3.106 | Apache-2.0 OR MIT |
| winapi-util | 0.1.11 | MIT OR Unlicense |
| windows | 0.56.0 | Apache-2.0 OR MIT |
| windows | 0.62.2 | Apache-2.0 OR MIT |
| windows-collections | 0.3.2 | Apache-2.0 OR MIT |
| windows-core | 0.56.0 | Apache-2.0 OR MIT |
| windows-core | 0.62.2 | Apache-2.0 OR MIT |
| windows-future | 0.3.2 | Apache-2.0 OR MIT |
| windows-implement | 0.56.0 | Apache-2.0 OR MIT |
| windows-implement | 0.60.2 | Apache-2.0 OR MIT |
| windows-interface | 0.56.0 | Apache-2.0 OR MIT |
| windows-interface | 0.59.3 | Apache-2.0 OR MIT |
| windows-link | 0.2.1 | Apache-2.0 OR MIT |
| windows-numerics | 0.3.1 | Apache-2.0 OR MIT |
| windows-result | 0.1.2 | Apache-2.0 OR MIT |
| windows-result | 0.4.1 | Apache-2.0 OR MIT |
| windows-strings | 0.5.1 | Apache-2.0 OR MIT |
| windows-sys | 0.45.0 | Apache-2.0 OR MIT |
| windows-sys | 0.61.2 | Apache-2.0 OR MIT |
| windows-targets | 0.42.2 | Apache-2.0 OR MIT |
| windows-targets | 0.52.6 | Apache-2.0 OR MIT |
| windows-threading | 0.2.1 | Apache-2.0 OR MIT |
| windows_aarch64_gnullvm | 0.42.2 | Apache-2.0 OR MIT |
| windows_aarch64_gnullvm | 0.52.6 | Apache-2.0 OR MIT |
| windows_aarch64_msvc | 0.42.2 | Apache-2.0 OR MIT |
| windows_aarch64_msvc | 0.52.6 | Apache-2.0 OR MIT |
| windows_i686_gnu | 0.42.2 | Apache-2.0 OR MIT |
| windows_i686_gnu | 0.52.6 | Apache-2.0 OR MIT |
| windows_i686_gnullvm | 0.52.6 | Apache-2.0 OR MIT |
| windows_i686_msvc | 0.42.2 | Apache-2.0 OR MIT |
| windows_i686_msvc | 0.52.6 | Apache-2.0 OR MIT |
| windows_x86_64_gnu | 0.42.2 | Apache-2.0 OR MIT |
| windows_x86_64_gnu | 0.52.6 | Apache-2.0 OR MIT |
| windows_x86_64_gnullvm | 0.42.2 | Apache-2.0 OR MIT |
| windows_x86_64_gnullvm | 0.52.6 | Apache-2.0 OR MIT |
| windows_x86_64_msvc | 0.42.2 | Apache-2.0 OR MIT |
| windows_x86_64_msvc | 0.52.6 | Apache-2.0 OR MIT |
| winnow | 1.0.4 | MIT |
| zmij | 1.0.23 | MIT |

## Verification and remaining release gates

CI is configured for Rust 1.93.0 on ubuntu-latest (ALSA headers) and macos-latest,
with default and serde feature sets. Device-opening tests are explicitly ignored.
Local verification (macOS, Rust 1.93.0): default and serde builds/tests, clippy
with warnings denied, formatting, and docs with warnings denied pass. The
README example is a compiled doctest. The list_devices example exits 0 and
lists three audio inputs and two ports in each MIDI direction. No physical
instrument recording has been exercised. Packaged-crate verification passed.
All 140 locked packages have license entries; all 15 initially committed paths
match the intended publication allowlist. New review fixes add only the private
callback capture helper and synthesized regression tests.

Standard review artifacts are local under
`reviews/implement-autosampler-plan-ham7-2026-10-04-111935-8e9f2e45eadd/`.
Both independent reviewers requested changes. The canonical fixer addressed
five Important findings and four deduplicated Suggestions. The user explicitly
approved the one-line CI workflow timeout fix after the automatic fixer flagged
it for manual approval; that fix has now been applied.
Runtime stream failures and lost callback data now reject the recording before
export. Release detection examines continuous fresh callback audio without
observer locking. MIDI values and capability reporting are validated.
Local verification results and review checkpoints are recorded in Beans.

Pending before release:

- Completion of standard-review checkpoint, then the thermonuclear review and second verified push checkpoint.
- Green CI on the exact revision to tag, then annotated v0.1.0 pushed and verified.

No crates.io publication, default-branch merge, or change to the private source
repository is part of this implementation.
