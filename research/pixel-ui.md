# Rust options for a keyboard-driven pixel UI

Research for issue #6 (child of #1). Checked 2026-11-16 against the crates'
repositories, source and crates.io metadata. Versions are the latest on
crates.io on that date.

## Question

Which Rust option fits a fixed-grid, keyboard-driven window like the Dirtywave
M8's screen (Song, Chain and Phrase views drawn as a text grid, edited with
modifier-held navigation), Linux first, alongside a real-time audio thread?

## Answer

**Use `winit` + `pixels`, drawing the grid yourself into a small framebuffer
with an embedded bitmap font.** `winit` gives the best keyboard events of any
option (an explicit `repeat` flag on every key event, a `ModifiersChanged`
event, physical and logical keys), and native Wayland and X11 by default.
`pixels` gives a fixed low-resolution framebuffer that the GPU scales to the
window on whole-pixel boundaries, which is the M8 look. The grid renderer is a
glyph blitter of a few dozen lines. Audio runs on its own `cpal` thread and
talks to the UI through lock-free queues; no option here constrains that.

If the GPU dependency (`wgpu`) proves a nuisance, swap `pixels` for
`softbuffer` (same `winit` window, CPU presentation, scaling done by hand);
the glyph blitter and input code don't change.

Runner-up: `macroquad`, the fastest to get running, but its Linux default is
X11 only, with Wayland marked unstable, and its `is_key_pressed` drops key
repeats.

## Comparison

| Option | Version (date) | Licence | Grid and bitmap-font work | Key repeat and modifiers | Linux Wayland / X11 | Maintenance |
|---|---|---|---|---|---|---|
| **winit + pixels** | winit 0.30.13 (2026-10-24), pixels 0.17.2 (2026-07-14) | winit Apache-2.0; pixels MIT | All yours: blit glyphs into an RGBA buffer. GPU integer scaling and optional shaders included | Best: `KeyEvent.repeat`, `WindowEvent::ModifiersChanged` | Both, native, by default | Both active |
| winit + softbuffer | softbuffer 0.4.8 (2025-12-13) | MIT OR Apache-2.0 | As above, plus you scale to the window size yourself on the CPU | As winit | Both, tier 1 | Active (rust-windowing org) |
| hand-rolled wgpu | wgpu 30.0.2 (2026-11-12) | MIT OR Apache-2.0 | Most work: pipeline, textures, glyph atlas, shaders | As winit | Both | Very active |
| macroquad | 0.4.16 (2026-07-30) | MIT OR Apache-2.0 | `draw_texture_ex` with a source rect over a font atlas; TTF text built in | `is_key_pressed` ignores repeats; repeats only via the raw miniquad event stream | Defaults to X11 only; Wayland "currently unstable" | Active, one maintainer, 349 open issues |
| minifb | 0.29.0 (2026-10-03) | MIT OR Apache-2.0 | Like softbuffer: you fill a `u32` buffer; scaling built in | `KeyRepeat::Yes/No` plus configurable delay and rate; modifiers by polling `is_key_down` | Both, default features | Active |
| raylib (raylib-rs) | 6.0.0 (2026-06-10) | Zlib | Bitmap fonts and texture atlases built in | `IsKeyPressedRepeat`, polling | X11 by default; Wayland behind a feature | Active; C library, needs cmake and a C toolchain |
| egui / eframe | 0.37.0 (2026-10-15) | MIT OR Apache-2.0 | Widget toolkit: the grid becomes a custom painter; pixel fonts need a TTF and fight its scaling | `key_pressed` includes repeats; `consume_key(modifiers, key)` | Both (winit) | Very active |
| iced | 0.14.0 (2025-12-07) | MIT | Elm-style toolkit: grid as a `Canvas`; text via its own shaping, bitmap fonts awkward | `KeyPressed { modifiers, repeat, .. }`, but the type is documented as "largely incomplete" | Both (winit) | Very active, but last release 11 months ago |
| ggez | 0.10.0 (2026-06-03) | MIT | TTF via `glyph_brush`; bitmap font by sprite batch | Callbacks on winit | Both (winit) | Active |
| notan | 0.14.0 (2026-01-17) | MIT OR Apache-2.0 | Draw API with fonts | Polling | Both (winit) | "Slow maintenance mode" |

## Findings by criterion

### Text and grid rendering

The M8 screen is a fixed grid of monospaced characters, so a toolkit's text
layout, shaping and widgets buy nothing. What matters is drawing glyph *n* at
cell *(col, row)* in a palette colour, pixel-exact.

- **Framebuffer options** (`pixels`, `softbuffer`, `minifb`) have no text at
  all. You embed a bitmap font (a PNG or a `const` array) and copy glyph
  pixels into the buffer. For a 4×6 or 8×8 font on a ~320×240 surface this is
  trivial and fully under your control. `pixels` adds "hardware accelerated
  scaling on perfect pixel boundaries" and custom shaders
  ([pixels README](https://github.com/parasyte/pixels#features)); `pixels`
  says of itself that "you're in charge of managing a window environment,
  event loop, and input handling", which is exactly the `winit` part.
- **Game frameworks** (`macroquad`, `raylib`, `ggez`, `notan`) draw sprites
  from an atlas, so a bitmap font is a texture plus source rects. Less code
  than a blitter, but you inherit their loop and window backend.
- **GUI toolkits** (`egui`, `iced`) are built around scalable vector text and
  widgets. A pixel grid means a custom painter or canvas and a pixel font
  converted to TTF, then fighting anti-aliasing and DPI scaling. Most work for
  the least fit.
- **Hand-rolled wgpu** means writing what `pixels` already is.

### Key combos, repeats and modifier-held navigation

M8-style editing (hold a modifier, tap or hold arrows to move or change
values) needs: press versus repeat told apart, modifier state on every event,
and physical keys independent of layout.

- **winit** `KeyEvent` has `repeat: bool`, "`true` if and only if this event
  is the result of one of those repeats", plus `physical_key` and `logical_key`;
  modifier state arrives as `WindowEvent::ModifiersChanged(Modifiers)`
  ([winit 0.30.13 `src/event.rs`](https://github.com/rust-windowing/winit/blob/v0.30.13/src/event.rs)).
  You can use OS repeat or ignore it and run your own repeat timer for exact,
  M8-like acceleration. Anything built on winit (`pixels`, `softbuffer`,
  `egui`, `iced`, `ggez`, `notan`) has this underneath, if its API exposes it.
- **macroquad**: `key_down_event` only inserts into `keys_pressed` when
  `repeat == false`, so `is_key_pressed` never fires on repeats
  ([macroquad `src/lib.rs`](https://github.com/not-fl3/macroquad/blob/master/src/lib.rs),
  [`src/input.rs`](https://github.com/not-fl3/macroquad/blob/master/src/input.rs)).
  Repeats are only reachable through `utils::register_input_subscriber` and
  `repeat_all_miniquad_input`, which replay miniquad's `key_down_event(keycode,
  keymods, repeat)` ([miniquad `src/event.rs`](https://github.com/not-fl3/miniquad/blob/master/src/event.rs)).
  Workable, but a second input path.
- **minifb**: `get_keys_pressed(KeyRepeat::Yes)` and `set_key_repeat_delay` /
  `set_key_repeat_rate` ([minifb `src/lib.rs`](https://github.com/emoon/rust_minifb/blob/master/src/lib.rs)):
  convenient, polling only, modifiers via `is_key_down(Key::LeftShift)`.
- **raylib**: `IsKeyPressed` and `IsKeyPressedRepeat`
  ([raylib.h](https://github.com/raysan5/raylib/blob/master/src/raylib.h)).
- **egui**: `key_pressed` "includes key-repeat events"
  ([egui `input_state/mod.rs`](https://github.com/emilk/egui/blob/main/crates/egui/src/input_state/mod.rs)),
  and `consume_key(Modifiers, Key)` handles combos. Its focus system will
  compete for keys if any widgets are used.
- **iced**: `keyboard::Event::KeyPressed` carries `modifiers` and `repeat`,
  with a doc note that the type "is largely incomplete"
  ([iced `core/src/keyboard/event.rs`](https://github.com/iced-rs/iced/blob/master/core/src/keyboard/event.rs)).

### Running alongside a real-time audio thread

None of these libraries owns the audio thread you need. In every case the
synth runs in a `cpal` output callback on its own thread, and the UI talks to
it through lock-free single-producer queues (commands in, playhead and meters
out). The real constraints are on the UI side:

- The window and event loop must stay on the main thread (a macOS rule that
  `winit` enforces everywhere). That is no problem when audio is elsewhere.
- **macroquad** keeps its state in a global, non-`Send` context accessed by
  `get_context()`, so all drawing and input stays on its main thread; fine,
  but it also bundles its own audio (`quad-snd`) you would leave unused.
- **raylib** and **ggez** bundle audio too (raylib's miniaudio, ggez's
  `rodio`); also left unused.
- **egui** repaints only on input unless asked, so a moving playhead needs
  `request_repaint`. With `winit` + `pixels` you choose: redraw on input and
  on playhead messages, which keeps the UI thread idle and away from the
  audio thread's CPU.

### Linux (Wayland and X11) and other platforms

- **winit** enables `x11` and `wayland` in its default features
  ([winit 0.30.13 `Cargo.toml`](https://github.com/rust-windowing/winit/blob/v0.30.13/Cargo.toml)),
  and runs on Windows, macOS, Android, iOS and the web. Everything built on it
  inherits that. `softbuffer` lists Wayland, X11, Win32 and AppKit as tier 1
  ([softbuffer README](https://github.com/rust-windowing/softbuffer#platform-support)).
  `pixels` targets Vulkan, Metal, DirectX 12 and OpenGL ES3 via `wgpu`.
- **miniquad/macroquad**: `LinuxBackend` "defaults to `X11Only`. The Wayland
  implementation is currently unstable"
  ([miniquad `src/conf.rs`](https://github.com/not-fl3/miniquad/blob/master/src/conf.rs)).
  On a pure Wayland session it runs through XWayland or needs
  `WaylandWithX11Fallback`.
- **minifb**: `wayland` and `x11` are both default features
  ([minifb `Cargo.toml`](https://github.com/emoon/rust_minifb/blob/master/Cargo.toml)).
- **raylib-rs**: Linux uses GLFW on X11; Wayland needs the `wayland` feature
  plus GLFW and Wayland dev packages, and every build needs cmake and a C
  compiler ([raylib-rs README](https://github.com/raylib-rs/raylib-rs)).
- **ggez**: "Fully supported: Windows, Linux, MacOS"
  ([ggez README](https://github.com/ggez/ggez#supported-platforms)).
- **notan**: Linux on OpenGL 3.3 ([notan README](https://github.com/Nazariglez/notan)).

### Maintenance and licences

Data from crates.io and the GitHub API on 2026-11-16.

- All candidates have released in 2026 except `iced` (0.14.0, 2025-12-07;
  repository still very active) and `softbuffer` (0.4.8, 2025-12-13;
  repository active today).
- **notan** describes itself as in "slow maintenance mode"
  ([notan README](https://github.com/Nazariglez/notan#status)). Avoid.
- **macroquad** is effectively one maintainer with 349 open issues.
- **winit** has a 0.31 beta on crates.io (0.31.0-beta.4, 2026-10-24); expect
  an API migration in the life of the project. `pixels` 0.17 still depends on
  `winit` 0.30 and `wgpu` 29 (one behind wgpu 30), so its releases trail its
  dependencies.
- Licences are all permissive: MIT, Apache-2.0, dual MIT/Apache-2.0, Zlib
  (raylib) and BSD-3-Clause (`tiny-skia`, if used). Note that **`winit` is
  Apache-2.0 only**, not dual-licensed; that is fine for a permissive project
  and compatible with GPLv3, but not with GPLv2-only. The picotracker repo has
  no licence file yet.

## Not recommended, and why

- **egui, iced**: built for widget UIs with scalable text; the M8 grid would
  fight their layout, focus and font systems.
- **Hand-rolled wgpu**: reinvents `pixels`.
- **notan**: slow maintenance.
- **raylib-rs**: good API fit, but a C build dependency and Wayland behind a
  feature.
- **ggez**: a full game framework with TTF-first text for what is a blitter.
- **minifb**: a decent fit, but polling only, with no event loop or `repeat`
  per event; `softbuffer` is the better CPU fallback because it keeps `winit`.

## Open points for later tickets

- Pick the bitmap font. PICO-8's own font would suit, but check its licence
  first.
- Pick the logical resolution (the original M8 screen is 320×240) and the integer scale.
- The audio-to-UI messaging (ring buffers such as `rtrb`) belongs with the
  audio-engine research, not here.
