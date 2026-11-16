# Cart text format: `__sfx__` and `__music__`

Research for issue #2 (child of map #1). Question: what is the exact layout of the `__sfx__` and `__music__` sections of a `.p8` text Cart, how does it map to Patterns at `0x3100` and SFX at `0x3200`, and what else is in a Cart so that Export can rewrite only these two sections?

## Sources

Each claim below is tagged with one of these.

| Tag | Source | Trust |
|-----|--------|-------|
| [M] | PICO-8 manual v0.2.7, `docs/sources/pico-8_manual.txt` (SFX Editor ~l.565, Music Editor ~l.657, Memory ~l.2080) | Primary (Lexaloffle) |
| [CL] | PICO-8 changelog, https://www.lexaloffle.com/dl/docs/pico-8_changelog.txt | Primary (Lexaloffle) |
| [Z26] | zep's PICO-8 0.2.6 announcement, https://www.lexaloffle.com/bbs/?tid=140421 | Primary (Lexaloffle) |
| [Z8] | zepto8 source, `src/pico8/cart.cpp` (`load_p8`, `save_p8`), `src/pico8/memory.h`, `src/pico8/sfx.cpp`, `src/synth.cpp`, https://github.com/samhocevar/zepto8 | Well-known reimplementation |
| [F8] | fake-08 source, `source/cart.cpp` (`setSfx`, `setMusic`), `source/PicoRam.h`, `source/Audio.cpp`, https://github.com/jtothebell/fake-08 | Well-known reimplementation |
| [BBS-E] | eruonna, "Data structures for sfx and music", https://www.lexaloffle.com/bbs/?pid=141623 (2015, edited 2018) | Community, reverse-engineered |
| [BBS-R] | ridgek, "Waveform Instrument Encoding", https://lexaloffle.com/bbs/?pid=140311 (Jan 2024, for 0.2.6) | Community, reverse-engineered |
| [CART] | A real `version 43` cart: `dr-jam/GamePlatforms` `carts/basic.p8` on GitHub | Observed file |

The PICO-8 wiki (pico-8.fandom.com) refused automated fetches (HTTP 402), so it is not cited.

## Memory layout (the target of both sections)

| Range | Size | Content | Source |
|-------|------|---------|--------|
| `0x3100–0x31FF` | 64 × 4 bytes | Song: 64 Patterns | [M] [Z8 `static_check_section(song, 0x3100, 0x100)`] |
| `0x3200–0x42FF` | 64 × 68 bytes | 64 SFX; SFX *n* is at `0x3200 + 68·n` | [M] [Z8 `static_check_section(sfx, 0x3200, 0x1100)`] |

The editors work on cart ROM, and `reload()` copies it to base RAM on load, run, and editor exit [M]. Cart ROM uses the same layout as base RAM up to `0x4300` [M].

### One SFX (68 bytes)

| Offset | Content |
|--------|---------|
| 0–63 | 32 notes, 2 bytes each, little-endian `u16` |
| 64 | Editor mode + filters byte |
| 65 | Speed (SPD) |
| 66 | Loop start (or LEN) |
| 67 | Loop end |

Sources: [Z8 `sfx_t`], [F8 `struct sfx`], [BBS-E].

### Note word (16 bits)

| Bits | Field | Range |
|------|-------|-------|
| 0–5 | Pitch (key) | 0–63: C-0 … D#-5; key 33 = A-2 = 440 Hz |
| 6–8 | Instrument (waveform) | 0–7 |
| 9–11 | Volume | 0–7 (0 = empty note) |
| 12–14 | Effect | 0–7 |
| 15 | Custom: the instrument number selects SFX instrument 0–7 instead of a built-in waveform | 0/1 |

Sources: [Z8 `note_t` bitfields], [F8 `note` getters/setters]. The instrument field straddles the two bytes: bits 6–7 of byte 0 and bit 0 of byte 1. Effects 0–7 and their meanings are in [M].

### Byte 64: editor mode + 5 filter switches

This byte is a mixed binary and base-3 encoding, not plain bit flags:

```
byte64 = editor_mode        // bit 0: 0 = pitch (graph) view, 1 = tracker view
       + noiz   * 2         // bit 1
       + buzz   * 4         // bit 2
       + detune * 8         // 0..2  -> (b / 8)  % 3
       + reverb * 24        // 0..2  -> (b / 24) % 3
       + dampen * 72        // 0..2  -> (b / 72) % 3
```

The maximum value is 1 + 2 + 4 + 16 + 48 + 144 = 215, so the byte always fits.

Sources: [Z8 `synth.cpp`: `noiz = filters & 0x2`, `buzz = filters & 0x4`]; [Z8 `sfx.cpp`: `reverb = (filters / 24) % 3`, `dampen = (filters / 72) % 3`]; [F8 `Audio.cpp`: `detune = (filters / 8) % 3`]; editor flag per [Z8 comment "filters + editor flag"] and [BBS-E]. The meaning of each filter (NOIZ only on instrument 6, BUZZ + inst 6 = brown noise, two levels for DETUNE, REVERB, and DAMPEN) is in [M].

The manual calls DETUNE-1 and DETUNE-2 separate switches; in memory they are one 0–2 trit. The same holds for REVERB (delay 2 or 4 ticks) and DAMPEN (two LPF levels) [M].

### Speed, loop, LEN

- Speed: byte 65, the number of ticks per note, 1 = fastest [M]. Emulators clamp 0 to 1 [Z8 "Speed must be 1—255"]. A tick is 183 samples at 22050 Hz [Z8, F8 `22050 / (183 * speed)`]. If speed ≤ 8, arpeggio rates are halved [M].
- Loop: byte 66 = start, byte 67 = end. Looping is off when start ≥ end [M].
- LEN: if end = 0 and start > 0, start is the length in notes. The editor shows "LEN" instead of "LOOP". It is used to make a Pattern shorter than 32 rows, for example in 3/4 time [M]. Added in 0.2.2, and "Can be >= 32" [CL]. In playback, emulators truncate at `start` when `end == 0` [Z8 `sfx.cpp`, F8 `Audio.cpp`].
- Pattern length is the length of the left-most non-looping channel [M], [CL 0.1.2].

### SFX instruments and waveform instruments

- **SFX instrument** (0.1.11+ [CL]): note bit 15 = 1 makes the 3-bit instrument field index SFX 0–7 [Z8, F8]. In the editor it shows green instead of pink [M]. The note's pitch is added relative to C-2, volume is multiplied, effects are layered on top, and the instrument SFX's filters are enabled for the note [M]. An SFX instrument is retriggered only on a pitch change or after a zero-volume note; effect 3 inverts this [M].
- **Waveform instrument** (0.2.6+ [CL] [Z26]): SFX 0–7 can be toggled to hold "64 8-bit samples of a short looping waveform" instead of notes [Z26]. There is an optional "bass" toggle, one octave down [Z26]. Waveform instruments observe detune, reverb, and dampen [Z26]. zep did not publish the storage bits. The reverse-engineered layout [BBS-R] is:
  - byte 66 (loop start) bit 7 (`0x80`) set: this SFX is a waveform instrument;
  - byte 65 (speed) bit 0 set: bass, one octave down;
  - byte 64 is still editor mode + filters; byte 67 is unused;
  - bytes 0–63 are the 64 samples.
  
  **Confidence: medium.** Only one community source gives this, and neither zepto8 nor fake-08 implements waveform instruments. A search summary also reported an earlier wording that put the flag in byte 65. Verify by saving a waveform instrument in PICO-8 0.2.6+ and diffing the `__sfx__` line.

### Pattern (4 bytes at `0x3100 + 4·p`)

Each byte is one channel (0–3):

| Bits | Meaning |
|------|---------|
| 0–5 | SFX index 0–63 |
| 6 | Channel disabled/muted (1 = channel does not play) |
| 7 | One flag per byte (see below) |

| Byte | Bit 7 flag |
|------|-----------|
| 0 | Loop start (begin) |
| 1 | Loop back (end): jump back to the nearest earlier loop start, or Pattern 0 |
| 2 | Stop |
| 3 | Undocumented ("mode" in both emulators; no published meaning) |

Sources: [Z8 `song_t`], [F8 `song`], [BBS-E] (bits 0–5 SFX, bit 6 mute, bit 7 flags). Flow-control semantics are in [M].

## Text encoding in `.p8`

General rules [Z8 `p8_reader`, F8 `Cart` parser]:

- The header is line 1, `pico-8 cartridge // http://www.pico-8.com`, and line 2, `version N`. N is the cart format version; 41, 42, and 43 are common in the wild, and [CART] uses 43. The version was bumped at 0.2.0 [CL]. Export should copy the header unchanged.
- A section starts at a line that is exactly `__name__`. Data continues until the next such line or EOF.
- Data sections are lowercase hex. zepto8 skips non-hex characters and is tolerant of upper case; an optional UTF-8 BOM is accepted [Z8].
- Trailing rows that equal the default state are omitted (0.1.11e+ [CL]). A missing row means "default", not "absent". Emulators fill in defaults: zepto8 sets missing Patterns to channels `0x41 0x42 0x43 0x44` (all disabled) [Z8 `init_rom`], and fake-08 sets missing SFX speed to 16 [F8 `setSfx`].
- [CART] ends with `__music__` data followed by one blank line.

### `__sfx__`: one line per SFX, 168 hex characters, in index order

```
EE SS LS LE  (P P I V E) x 32
\_8 chars_/  \__ 160 chars __/
```

- Characters 0–7 hold bytes 64, 65, 66, 67 verbatim as 2 hex digits each: editor/filters, speed, loop start, loop end [F8 `setSfx`], [Z8 `load_p8`].
- Then come 32 notes of 5 hex digits each [F8], [Z8]:
  - `PP` (2 digits): pitch 0x00–0x3F. Only 6 bits are stored; the upper 2 bits are dropped.
  - `I` (1 digit): instrument 0–7. Values 8–F mean the custom bit (note bit 15) is set and the instrument is SFX instrument `I − 8`. fake-08 uses `custom = waveform > 7`; zepto8 reads bit `0x800` of the 20-bit group.
  - `V` (1 digit): volume 0–7.
  - `E` (1 digit): effect 0–7.
- Note order in the text is natural: note 0 is first, with no nibble swapping. Only `__gfx__` swaps nibbles [Z8].

Example from [CART]: `000100001e050…` gives editor/filters `00`, speed `01`, loop `00`/`00`, and then note 0 `1e050` = pitch 0x1E, instrument 0, volume 5, effect 0.

The text encoding covers all 16 bits of each note word (6 + 1 + 3 + 3 + 3). A waveform instrument's 64 sample bytes therefore round-trip through the same note encoding: Export must pack samples into notes rather than writing them raw.

**Caution: zepto8's `save_p8` is not a reference for writing.** It writes the instrument digit with only 3 bits and the effect digit as `(byte1 >> 4) & 0xF`, so the custom bit lands in the effect digit. That contradicts its own loader and fake-08. Follow the loader and fake-08: the custom bit goes in the instrument digit.

### `__music__`: one line per Pattern, in index order

```
FF CCCCCCCC
```

- `FF` (2 hex digits) is the flag byte: bit 0 = loop start, bit 1 = loop back, bit 2 = stop, bit 3 = the undocumented channel-3 bit [F8 `setMusic`], [Z8 `save_p8`].
- One space follows.
- Then come 4 channel bytes, 2 hex digits each. Bits 0–6 are the SFX index plus the disabled bit (`0x40`); bit 7 is never written here.
- Mapping to memory: `mem[ch] = chan_byte[ch] | (((FF >> ch) & 1) << 7)` [F8], [Z8].
- An empty channel is written as `0x40 | (ch + 1)`, so `41 42 43 44`. Any value with bit 6 set is disabled.

Example from [CART]: `03 01424344` is Pattern 0 with loop start and loop back set. Channel 0 plays SFX 1, and channels 1–3 are disabled.

## Version differences

| Version | Change | Source |
|---------|--------|--------|
| 0.1.2 | Pattern length = first non-looping channel | [CL] |
| 0.1.4 | "Unused high bits in SFX section not saved in .p8 format" fixed: all note bits now round-trip | [CL] |
| ≤ 0.1.10 | Byte 64 = editor mode only; note bit 15 unused | [BBS-E], [CL] |
| 0.1.11 | Custom SFX instruments (note bit 15, instrument digit 8–F). 0.1.11c: "Loading carts from earlier versions alters SFX data not intended for audio", so PICO-8 migrates old carts by header version | [CL] |
| 0.1.11e | `.p8` skips trailing rows that match the default state | [CL] |
| 0.2.0 | Cart version number bumped | [CL] |
| 0.2.2 | 5 filters added to byte 64; LEN (loop end = 0) added and can be ≥ 32 | [CL] |
| 0.2.4c | `__meta:*__` sections added for external tools | [CL] |
| 0.2.6 | Waveform instruments (SFX 0–7, byte 66 bit 7 per [BBS-R]; bass bit) | [CL], [Z26], [BBS-R] |
| 0.2.7 | Bug fixes only for waveform instruments; no format change found | [CL] |

Practical consequence: if picotracker writes filter bits, LEN, or waveform instruments, the Cart needs PICO-8 0.2.2+ or 0.2.6+. Export should never lower the header `version`.

## Other sections (leave untouched on Export)

| Section | Content | Encoding |
|---------|---------|----------|
| header (2 lines) | magic line + `version N` | text |
| `__lua__` | Lua source, with PICO-8 glyphs as Unicode | text; may contain anything |
| `__gfx__` | Sprite sheet `0x0000–0x1FFF`, up to 128 lines × 128 hex characters, one pixel per character (nibble-swapped relative to memory) | hex |
| `__label__` | 128 × 128 cart label, one character per pixel, base 32 `0-9a-v` (extended palette, 0.2.0g+) | base32 |
| `__gff__` | Sprite flags `0x3000–0x30FF`, 2 lines × 256 hex characters | hex |
| `__map__` | Map `0x2000–0x2FFF`, up to 32 lines × 256 hex characters. Map rows 32–63 share memory with gfx `0x1000` | hex |
| `__sfx__` | **rewritten by Export** | hex |
| `__music__` | **rewritten by Export** | hex |
| `__meta:<name>__` | Arbitrary tool data (0.2.4c+) | free text |

The usual order is `__lua__`, `__gfx__`, `__label__`, `__gff__`, `__map__`, `__sfx__`, `__music__` [Z8 `save_p8`], [CART], but every section is optional. Readers treat any `__xxx__` line as a section boundary and ignore unknown ones [Z8 `r_any`].

Recommended Export strategy:

1. Split the file into line ranges by section-header lines.
2. Replace only the bodies of `__sfx__` and `__music__`, and keep every other byte, including line endings and `__meta:*__` blocks.
3. If a section is absent, insert it in canonical position: `__sfx__` after `__map__`, `__gff__`, `__label__`, `__gfx__`, or `__lua__` (the last of those present), and `__music__` right after `__sfx__`.
4. Write rows 0 through the last non-default row, and write the defaults for any gaps between used rows. Defaults are an all-zero SFX row with speed 16, and Pattern `00 41424344`.

Edge case: a Lua long string containing a line that is exactly `__sfx__` would be misread as a section by PICO-8 and by every emulator. It is not worth handling.

## Open points

- Byte 66 bit 7 as the waveform flag, and byte 65 bit 0 as bass, come from one reverse-engineered source. Confirm against a 0.2.6+ save before relying on them.
- The meaning of Pattern byte 3 bit 7 (text flag bit 3) is unknown. Preserve it on import and write 0 for new Patterns.
- PICO-8's exact default speed for an untouched SFX is not confirmed from a primary source. fake-08 assumes 16. Confirm with a blank cart.
