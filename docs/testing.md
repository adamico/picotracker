# Testing

What proves each area of the app. How to write a test, and the five seams, are in `CODING_STANDARDS.md`.

## What proves each area

| Area | Proof |
|---|---|
| Cart reading and writing | Carts saved by real PICO-8 with expected bytes, plus read-write round trips |
| Cart Import | Expected `.ptrk` snapshots and warning lists; an Import with no warnings renders identical samples to the original Cart through our synth |
| MIDI Import | Small MIDI files built in test code, with expected Projects |
| Export | The Exported Cart renders sample for sample the same as the Project's Export Tracks through our synth |
| Playback | Comparison with stored measurements of real PICO-8, plus exact snapshots of our synth's output |
| Keyboard UI | Actions drive the headless App; tests assert on the Project and grid text, plus one pixel snapshot of the font and palette |
| `.ptrk` files | One comprehensive fixture per released format version loads to the expected Project; saving and reloading gives an identical Project |

Sound-equality tests render one pass of the chosen song, from its first Pattern to its loop-back or stop.

## Export stays inside the Claim

A property test runs over random Carts, Claims and Projects:

- **`.p8`:** every byte outside the claimed SFX and Pattern lines is identical, including line endings and `__meta:*__` sections.
- **`.p8.png`:** every decoded pixel is identical, except the claimed bytes and the SHA-1.
- **Failed Export:** the file is left byte for byte unchanged.

## Reference set

The set is pinned to PICO-8 0.2.7. It contains the 28-item coverage Cart, every checked PICO-8 behaviour, one case for each of the four synth fixes, Waveform Instruments, and the behaviours measured in [Measure the last unchecked behaviours](https://github.com/adamico/picotracker/issues/29).

- The repo stores one measurement fingerprint per case, the driver Carts and a one-command recording script. WAV recordings stay on the owner's machine.
- Only Carts we generate ourselves are committed, re-saved through real PICO-8 where the file format matters.
- Random and malformed Carts run as property tests.
- Community Carts stay in a local, gitignored folder for an optional smoke test.

| Fingerprint measure | Limit |
|---|---|
| Duration | ±0.5% |
| Pitch, for tonal notes | ±5 cents |
| Level per Tick, most cases | ±0.5 dB |
| Level, noise | ±2 dB |
| Level, NOIZ, BUZZ and DETUNE 1 | ±3 dB |

Loosening a limit requires an A/B listening check by the owner, noted in the commit.

Synth snapshots render with a fixed noise seed, convert to 16-bit samples and store a hash per reference case. Hashes are checked on Linux x86_64 only. A commit that changes a hash must also pass the fingerprint comparison.

## What runs where

- **CI, on every push:** formatting, lints, all tests including the fingerprint comparison, `cargo-deny`, and compile checks on Windows and macOS. PICO-8 never runs in CI.
- **Locally before a draft release is published:** re-record the reference set and confirm the fingerprints; open a set of Exported `.p8` and `.p8.png` Carts in real PICO-8, record them and compare them. Any failure blocks the release.
- **By hand on each release draft, from the unpacked archive:** Import a `.p8` and a `.p8.png` Cart; Import a MIDI file; edit in the Song, Chain and Phrase views; listen to playback; Export 4 Tracks and play the Cart in PICO-8.
