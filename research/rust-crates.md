# Rust crates for audio, carts and MIDI files

Research for issue #5 (child of the map, #1). Checked on 2026-11-16 against
crates.io (API `https://crates.io/api/v1/crates/<name>`), the crates'
GitHub repositories (via `gh api`), docs.rs/READMEs and the PICO-8 manual.
"Downloads (90d)" is crates.io's `recent_downloads`.

## Summary

| Need | Recommendation | Why |
| --- | --- | --- |
| (a) Audio output | **cpal 0.18** | Callback-based, low-level, ALSA by default with optional PipeWire/JACK/PulseAudio backends on Linux, plus every other desktop/mobile/web target. Most-used, actively maintained. |
| (b) PICO-8 Carts | **No crate fits; write our own small reader/writer.** Optionally use `pico8_decompress` to read `.p8.png` | No crate on crates.io parses *and writes* `.p8`/`.p8.png`. The only parser (nano9) is tied to Bevy. The format pieces we need (`__sfx__`, `__music__`, ROM 0x3100–0x42FF) are small. |
| (c) Standard MIDI Files | **midly 0.5** | Complete SMF reader *and* writer, zero-copy, `no_std` optional, widely used. Unlicense. Release cadence is slow, but SMF is a frozen spec. |

## (a) Low-latency audio output

| Crate | Latest | Last release / last commit | Downloads (total / 90d) | Licence | Notes |
| --- | --- | --- | --- | --- | --- |
| [cpal](https://crates.io/crates/cpal) | 0.18.2 | 2026-08-16 / 2026-11-11 | 29.2M / 11.0M | Apache-2.0 | RustAudio org, 4.0k stars. Low-level device + stream API. |
| [rodio](https://crates.io/crates/rodio) | 0.23.0 | 2026-10-11 / 2026-09-07 | 14.3M / 4.1M | MIT OR Apache-2.0 | Playback layer on top of `cpal = "0.18"` (rodio `Cargo.toml`). Sources/sinks/mixing, decoders. |
| [tinyaudio](https://crates.io/crates/tinyaudio) | 2.0.0 | 2025-11-15 / 2026-09-26 | 162k / 67k | MIT | One callback to the default device. ALSA or PulseAudio on Linux. |
| [sdl2](https://crates.io/crates/sdl2) | 0.38.0 | 2025-07-12 / 2026-03-22 | 5.8M / 0.8M | MIT | Bindings to SDL2. 262 open issues, slowing as SDL3 takes over. |
| [sdl3](https://crates.io/crates/sdl3) | 0.22.1 | 2026-11-09 / 2026-11-10 | 303k / 120k | MIT | Bindings to SDL3, active, pre-1.0. |
| [kira](https://crates.io/crates/kira) | 0.12.6 | 2026-10-28 | 1.2M / 314k | MIT OR Apache-2.0 | Game-audio engine (tweens, clocks, mixer) on cpal. |
| [oddio](https://crates.io/crates/oddio) | 0.7.4 | 2023-10-15 | 63k / 9k | MIT OR Apache-2.0 | Stale. |

### cpal (recommended)

- **Backends** (README "Supported Platforms"): Linux default ALSA, optional
  `jack`, `pipewire` and `pulseaudio` Cargo features. Also CoreAudio
  (macOS/iOS), WASAPI + optional ASIO (Windows), AAudio (Android), Web Audio
  / AudioWorklet (wasm). ALSA dev files (`libasound2-dev` / `alsa-lib`) are
  needed to build on Linux even when another backend is used. PipeWire
  backend needs PipeWire ≥ 0.3.53. MSRV 1.85 (PulseAudio 1.88).
- **Latency features**: `realtime` raises the audio callback thread to RT
  priority (needs `CAP_SYS_NICE`/`rtprio`), `realtime-dbus` uses rtkit for
  normal desktop users. `StreamConfig.buffer_size` takes
  `BufferSize::Fixed(frames)` to request small buffers (`src/lib.rs`).
- **API shape** (`examples/beep.rs`): `cpal::default_host()` or
  `cpal::host_from_id(HostId::PipeWire)` → `host.default_output_device()` →
  `device.default_output_config()` → `device.build_output_stream(config,
  |data: &mut [T], info: &CallbackInfo| { ... }, err_fn, None)` →
  `stream.play()`. Our synth fills `data` in the callback: exactly what a
  tracker that renders PICO-8 voices sample-by-sample needs. Note the 0.18
  callback type is `CallbackInfo` (older tutorials show
  `OutputCallbackInfo`), so check examples against 0.18.
- **Maintenance**: releases 0.18.0 (2026-06-06), 0.18.1, 0.18.2
  (2026-08-16), commits through 2026-11-11, 103 open issues.
- **Licence**: Apache-2.0 only (not dual MIT), compatible with an MIT or
  Apache project.

### Why not the others

- **rodio**: adds a `Source`/`Sink` playback model, decoders and mixing we
  don't need, because we synthesize our own samples. It hides the callback
  behind its mixer, which adds a buffering layer. Use cpal directly. rodio is
  worth it only if we later want to play WAV previews.
- **tinyaudio**: the simplest API (`run_output_device(params, callback)`),
  MIT, all platforms. But by its own README it has no device enumeration or
  selection and no format querying, and its Linux backends are only ALSA or
  PulseAudio (no PipeWire/JACK, no RT priority). It's a good fallback if cpal
  ever gets in the way.
- **sdl2 / sdl3**: sensible only if we also use SDL for the window, input and
  rendering. SDL2 bindings are slowing down (last release 2025-07). SDL3
  bindings are active but pre-1.0 and need the SDL3 C library (system or
  bundled build). Revisit together with the UI/windowing decision.
- **kira / oddio**: game-audio engines aimed at sound-effect playback, not
  sample-accurate sequencing. oddio is stale.

## (b) PICO-8 Carts (`.p8`, `.p8.png`)

I searched crates.io for `pico8`, `pico-8` and `p8`. The names `p8`,
`pico8` and `pico8-cart` are not registered. Relevant hits:

| Crate | Latest / updated | Licence | What it does | Fit |
| --- | --- | --- | --- | --- |
| [pico8_decompress](https://crates.io/crates/pico8_decompress) | 0.1.0 / 2025-05-31 | MIT | `extract_bits_from_png(impl Read) -> Vec<u8>` (32 KiB cart ROM from a `.p8.png`) and `decompress()` for the code section (old `:c:` and new PXA formats). Ported from Lexaloffle's official C snippets ([dansanderson/lexaloffle](https://github.com/dansanderson/lexaloffle)). Only dep is `png` 0.17 (optional) and `thiserror`. | **Read-only** `.p8.png` helper. Single 0.1.0 release, 0 stars, one author. Small enough to vendor. |
| [nano9](https://crates.io/crates/nano9) | 0.1.0-alpha.8 / 2026-09-23 | MIT OR Apache-2.0 | PICO-8 compatibility layer for Bevy. `src/pico8/cart/mod.rs` parses `.p8` text (`__lua__`, `__gfx__`, `__gff__`, `__map__`, `__sfx__`, `__music__`) and `.p8.png` (via pico8_decompress), decoding SFX from ROM 0x3200–0x42FF in 68-byte chunks and music from 0x3100. | Parser is good reference code, but it's a Bevy `AssetLoader` inside an alpha engine (Bevy 0.20 dependency). **No writer.** Not usable as a dependency. |
| [pico8-to-lua](https://crates.io/crates/pico8-to-lua) | 0.2.1 / 2026-10-15 | — | Translates PICO-8 Lua dialect. | Not cart I/O. |
| [px8](https://crates.io/crates/px8) | 0.0.4 / 2017-08-02 | MIT | Old fantasy console (repo now UnicornConsole, last push 2023). | Dead. |

**Conclusion: no crate both parses and writes Carts. Write a small
`cart` module ourselves.** The parts a tracker needs are well bounded:

- PICO-8 manual: there are 64 SFX, each with 32 notes (pitch, instrument,
  volume, effect) plus speed and loop start/end. There are 64 music
  patterns of 4 channel SFX ids. Cart ROM is 32k and uses "same layout as
  base ram until 0x4300". The RAM map lists `0x3100 SONG` and `0x3200
  SFX`. So SFX = 64 × 68 bytes at 0x3200–0x42FF, music = 64 × 4 bytes at
  0x3100–0x31FF (layout confirmed by nano9's `Sfx::from_u8` /
  `MusicPattern::from_rom`).
- `.p8` is plain text with `__section__` headers. The manual doesn't specify
  it beyond "you can edit `.p8` files with a text editor", so we write it to
  match what PICO-8 emits and test round-trips against real carts. Writing
  `.p8` lets us **preserve unknown sections verbatim** (`__lua__`, `__gfx__`,
  …) and replace only `__sfx__`/`__music__`.
- `.p8.png` stores the ROM in the 2 low bits of each ARGB channel
  (`pico8_decompress::extract_bits`). Reading is easy with pico8_decompress.
  *Writing* `.p8.png` also needs code re-compression (PXA) and the cart label
  image, so **v1 should export `.p8` only** and import both.

The manual doesn't document the `.p8` text grammar or the PNG encoding, so
those details come from implementations (Lexaloffle's C snippets,
pico8_decompress, nano9) rather than a spec. Treat them as needing
round-trip tests against carts saved by PICO-8 itself.

## (c) Standard MIDI Files

| Crate | Latest | Last release / last commit | Downloads (total / 90d) | Licence | Notes |
| --- | --- | --- | --- | --- | --- |
| [midly](https://crates.io/crates/midly) | 0.5.3 | 2023-01-01 / 2024-06-15 | 626k / 253k | Unlicense | Read + write SMF and live events. Repo moved from `negamartin/midly` to [kovaxis/midly](https://github.com/kovaxis/midly), 202 stars, 5 open issues. |
| [midi-msg](https://crates.io/crates/midi-msg) | 0.9.0 | 2026-05-08 | 51k / 6k | MIT | Aims at the full MIDI 1.0 spec, SMF support behind its default `file` feature. Pre-1.0, small user base. |
| [nodi](https://crates.io/crates/nodi) | 1.0.3 | 2025-01-01 | 66k / 1k | MIT | Playback/timing layer *on top of midly*. |
| [augmented-midi](https://crates.io/crates/augmented-midi) | 1.8.0 | 2024-08-26 | 26k / 0.3k | MIT | "Experimental" nom-based parser in a larger monorepo. |
| [rimd](https://crates.io/crates/rimd) | 0.0.1 | 2016-02-09 | 17k | MIT | Dead. |
| `smf` | — | — | — | MPL-2.0 | Unrelated (illumos Service Management Facility). Name trap. |

### midly (recommended)

- **API** (README / docs.rs): `let smf = midly::Smf::parse(&bytes)?;` →
  `smf.header` (`Format`, `Timing::Metrical(ticks_per_beat)` or
  `Timecode`), `smf.tracks: Vec<Vec<TrackEvent>>`, each with
  `delta: u28` and `kind: TrackEventKind::{Midi{channel, message},
  SysEx, Escape, Meta(MetaMessage::Tempo(..)…)}`. `MidiMessage::NoteOn
  {key, vel}` etc. Writing: `smf.save(path)` / `smf.write(&mut out)`.
  `LiveEvent::parse` also covers real-time MIDI if we later add MIDI input.
- **Properties**: zero-copy (events borrow the input buffer), optional
  `no_std`/no-`alloc`, handles running status and other spec edge cases.
- **Maintenance**: 0.5.3 is from 2023-01-01, last commit 2024-06-15. That's
  slow, but SMF is a 1990s frozen spec and the crate is feature-complete.
  It's still by far the most downloaded SMF crate (~250k / 90 days) and is
  what higher-level crates like nodi build on. Pin `midly = "0.5"`.
- **Licence**: Unlicense (public-domain dedication). Compatible with anything.
  Some organisations' policies reject Unlicense. If that matters, midi-msg
  (MIT) is the fallback.

## Sources

- crates.io API records for every crate above (`/api/v1/crates/<name>`,
  `/versions`, `?q=pico8|pico-8|p8`).
- cpal: [README](https://github.com/RustAudio/cpal/blob/master/README.md),
  `Cargo.toml` `[features]`, `examples/beep.rs`, `src/lib.rs` (`BufferSize`).
- rodio: `Cargo.toml` (`cpal = "0.18"`).
- tinyaudio: [README](https://github.com/mrDIMAS/tinyaudio) ("What this
  crate cannot do", Linux backends), `Cargo.toml`.
- midly: [README](https://github.com/kovaxis/midly), [docs.rs/midly](https://docs.rs/midly).
- PICO-8 manual: <https://www.lexaloffle.com/dl/docs/pico-8_manual.html>
  (SFX/music counts, cart ROM layout, RAM map).
- pico8_decompress: [src/lib.rs](https://github.com/shanecelis/pico8_decompress/blob/main/src/lib.rs), `Cargo.toml`.
- nano9: [src/pico8/cart/mod.rs](https://github.com/shanecelis/nano9/blob/main/src/pico8/cart/mod.rs), `Cargo.toml`.
- Lexaloffle compression snippets: [dansanderson/lexaloffle](https://github.com/dansanderson/lexaloffle).
- GitHub repo metadata (stars, last push/commit, open issues) via `gh api repos/<owner>/<repo>`.
