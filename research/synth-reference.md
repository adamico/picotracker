# Reference for PICO-8's synth

Research for issue #4. Question: which open implementation best describes PICO-8's audio, how accurate and how licensed is each, which one should picotracker's emulation follow, and can a Rust tracker (licence not yet chosen) reuse its code?

Researched 2026-11-16 against PICO-8 v0.2.7 (the local manual, `docs/sources/pico-8_manual.txt`, and the official changelog). Source links are pinned to the commit read.

## Answer

**Follow zepto8's `src/pico8/sfx.cpp` + `src/synth.cpp` + `src/filter.cpp`, with fake-08's later bug fixes.** It is the only family that models all 8 waveforms with BUZZ variants, all 8 effects, all 5 filters, SFX instruments with PICO-8's retrigger rule, the 183-sample tick at 22050 Hz, LEN, and the left-most non-looping channel rule. Its constants were fitted to PICO-8's own WAV exports. Licence: zepto8 is **WTFPL v2**, so a Rust port is allowed under any licence the project picks. fake-08's own changes are **MIT**, which needs only its copyright and permission notice kept.

**No open implementation supports waveform instruments** (PICO-8 0.2.6, 2024). That part must be built from zep's 0.2.6 announcement and a community byte-layout post, and checked against real PICO-8.

## Candidates

| Project | Language | Licence | Audio status | Verdict |
|---|---|---|---|---|
| [zepto8](https://github.com/samhocevar/zepto8) (Sam Hocevar; synth rewritten by Nusan, Oct 2024) | C++ | WTFPL v2 (per-file headers; repo has no LICENSE file) | Most complete. Waveforms, effects, filters, SFX instruments, LEN, pattern rule. No waveform instruments. One REVERB bug. | **Reference** |
| [fake-08](https://github.com/jtothebell/fake-08) (Jon Bell) | C++ | MIT for fake-08; its `synth.cpp` keeps zepto8's WTFPL header; `Audio.cpp` is "based on zepto8's" | Port of zepto8's Oct 2024 synth (Dec 2025), plus fixes: REVERB level 2 and vibrato (2026). No waveform instruments. | **Use for its fixes** |
| [nano9](https://github.com/shanecelis/nano9) (Shane Celis) | Rust (Bevy) | MIT OR Apache-2.0 | Alpha. Waveform amplitudes taken from zepto8, 183-sample tick. No filters, no waveform instruments, `sfx()` length rejected, pattern length = longest channel. | Useful Rust shape only |
| [pemsa](https://github.com/egordorichev/pemsa) (Egor Dorichev) | C++ | MIT | Inactive since 2022. Waveforms copied from picolove. Effects and SFX instruments present. No filters; pattern length = slowest channel × 32. | Not accurate enough |
| [picolove](https://github.com/picolove/picolove) | Lua (LÖVE) | zlib | Pre-0.2.2 model. Guessed waveforms, effects, no filters, no SFX instruments; pattern speed = fastest channel. | Historical |
| [retro8](https://github.com/Jakz/retro8) (Jakz) | C++ | GPL-3.0 | Wrong tick (`44100/128 × (speed+1)` samples), no filters. | Avoid (inaccurate, and GPL) |

Lexaloffle has no open-source PICO-8 synth. The PICO-8 web player is closed.

## Facts, with the source of each

### Sample rate and tick length

- PICO-8 mixes at **22050 Hz**. The changelog for 0.1.12 says "Audio sampling rate is wrong when device/driver doesn't support 22050MHz" ([changelog](https://www.lexaloffle.com/dl/docs/pico-8_changelog.txt)).
- One tick is **183 samples at 22050 Hz**, about 120.49 Hz, a little under 1/120 s. A note lasts `speed` ticks (manual line 579: "the number of 'ticks' to play each note for"). zepto8 takes this from PICO-8's WAV exports: "PICO-8 exports instruments as 22050 Hz WAV files with 183 samples per speed unit per note" ([sfx.cpp#L86-L88](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L86-L88)). pemsa and picolove use `7350/61` notes per second, which is the same ratio, 22050/183 ([pemsa](https://github.com/egordorichev/pemsa/blob/6c13c5879c800af33543f702a353285cfa9e6fb0/src/pemsa/audio/pemsa_audio_channel.cpp#L225)). Users on the Lexaloffle forum report the same 183-sample figure ([BBS](https://lexaloffle.com/bbs/?pid=91665)). zep has not stated it in a post we could find. It is measured, not documented.
- The reverb buffers are 366 and 732 samples long, that is, 2 and 4 ticks (manual line 651: "echo with a delay of 2 or 4 ticks").

### Note data and pitch

- Each SFX has 32 notes: pitch C0..C5 (0..63), instrument 0..7, volume 0..7 and effect 0..7. Each SFX also has a speed and loop start/end values (manual 567–585). In RAM an SFX takes 68 bytes at `0x3200 + 68*n`. Byte 64 holds the filters and editor mode, byte 65 the speed, bytes 66–67 the loop start and end ([zepto8 memory.h](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/memory.h#L45-L57)). Each note's high bit (bit 15) selects an SFX instrument ("custom").
- Pitch to frequency is `440 * 2^((key-33)/12)`, so key 33 is A-4 ([sfx.cpp](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp)).

### The 8 waveforms

zepto8's [`synth.cpp`](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/synth.cpp) says its "Multipliers were measured from PICO-8 WAV exports. Waveforms are inferred from those exports by guessing what the original formulas could be." For phase `t ∈ [0,1)`:

| # | Name | zepto8 formula (amplitude) | BUZZ variant |
|---|---|---|---|
| 0 | triangle | `1-|4t-2|`, ×0.5 | 75% triangle + 25% tilted saw |
| 1 | tilted saw | rises to `a=0.875`, then falls, ×0.5 | `a=0.975` |
| 2 | saw | `t<.5 ? t : t-1`, ×0.653 | ×0.83 with a −0.085 offset on alternate periods |
| 3 | square | ±0.25, duty 50% | duty 40% |
| 4 | pulse | ±0.25, duty 31.6% | duty 25.5% |
| 5 | organ | two triangles of unequal height, /9 | first triangle is cut |
| 6 | noise | white noise through a one-pole low-pass whose cutoff follows pitch; louder at low keys | BUZZ with NOIZ off gives brown noise (manual 655) |
| 7 | phaser | two triangles at frequency ratio 109/110, /6 | adds overtones at 2× and 4× |

pemsa and picolove use simpler, older guesses (for example pulse duty 0.3125, phaser ratio 127/128). They have no BUZZ variants.

### The 8 effects

The manual describes them at lines 621–632. zepto8's implementation ([sfx.cpp#L151-L196](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L151-L196)):

- **1 slide.** The manual says "slide to the next note", but zepto8 found that it actually slides *from the previous* note and volume to this one, linearly across the note. The default previous key is C-2 (24).
- **2 vibrato.** A triangle LFO at `7.5 × notes-per-second`, half a semitone deep. fake-08 fixed the shape in its own port ([commit 2026-03-07](https://github.com/jtothebell/fake-08/commits/master/source/Audio.cpp)); zepto8's code already has the corrected form.
- **3 drop.** Frequency × (1 − note progress), falling to 0.
- **4 fade in / 5 fade out.** Volume × progress, or × (1 − progress).
- **6 / 7 arpeggio.** Steps through the 4-note group that holds the note (`note & ~3`), at 4 or 8 ticks per note. When SFX speed ≤ 8 these become 2 and 4 (manual 633; changelog 0.1.6).

### The filters (since 0.2.2)

Byte 64 of an SFX packs the filters. Bit 0 is the editor mode, bit 1 NOIZ, bit 2 BUZZ. DETUNE, REVERB and DAMPEN are packed in base 3: `detune = (b/8)%3`, `reverb = (b/24)%3`, `dampen = (b/72)%3` ([sfx.cpp#L374-L375, L458](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L374)). The manual describes them at lines 642–655, and the changelog lists them under 0.2.2.

- **NOIZ.** Applies only to instrument 6: "pure white noise". zepto8 multiplies the noise by a saw at the note's pitch.
- **BUZZ.** Changes each waveform; see the table above.
- **DETUNE-1/2.** Adds a second voice at half amplitude. Level 1 is a slight offset (200/199; phaser 49/50; triangle a fourth, 3/4). Level 2 is about an octave up (400/199; organ two octaves up, 800/199, played on a triangle; triangle a fifth, 3/2). Noise is excluded ([sfx.cpp#L458-L474](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L458)).
- **REVERB.** A feedback echo, mixed in at 0.5. Level 1 uses a 2-tick delay (366 samples), level 2 a 4-tick delay (732 samples).
  - **zepto8 bug:** it multiplies the 4-tick echo by the level-1 flag (`value += chan_reverb1_value * reverb_4[...]`, [sfx.cpp#L406](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L406)), so REVERB 2 is silent.
  - fake-08 uses `chan_reverb2_value` ([Audio.cpp#L764-L765](https://github.com/jtothebell/fake-08/blob/814991a2571ad3970e386cef48f3b148aa1c27b9/source/Audio.cpp#L764)). Follow fake-08 here.
- **DAMPEN.** RBJ-cookbook high-shelf biquads: level 1 is −6 dB at 2400 Hz, level 2 −12 dB at 1000 Hz ([vm.h#L110-L111](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/vm.h#L110), [filter.cpp](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/filter.cpp)). The manual says "low pass filter at 2 different levels". The shelf is zepto8's fit to that.
- The hardware registers at `0x5f40–0x5f43` (half-rate, reverb, lowpass, distort per channel) can force these filters on. zepto8 and fake-08 model them. A tracker exporting carts only needs them if it lets users poke those registers.

### SFX instruments (since 0.1.11)

From the manual (lines 690–715): when a note's instrument has the custom bit set, it plays SFX 0..7 as an instrument.

- Pitch is added relative to C2.
- Volume is multiplied.
- Effects are applied on top of the instrument's own.
- The instrument's filters apply.
- It retriggers only when the pitch changes or the previous note had volume 0. Effect 3 inverts this rule.

zepto8 runs a second `custom_sfx` cursor that always loops, scales its frequency by `freq / key_to_freq(24)`, and multiplies the volumes ([sfx.cpp#L327-L351](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L327)). Its retrigger test is "instrument or key changed, the main SFX looped, or the instrument ended and a new note began". It does not implement the effect-3 inversion, so treat that as unverified. pemsa has a similar second cursor.

### Waveform instruments (since 0.2.6): no reference implementation

- zep's 0.2.6 post ([BBS tid=140421](https://www.lexaloffle.com/bbs/?tid=140421), 2024-02-28) says that each one takes one SFX slot (0..7) and holds "64 8-bit samples of a short looping waveform". They can be played from regular SFX and from inside SFX instruments, and they observe the detune, reverb and dampen filters. A "bass" toggle drops them an octave. The manual (lines 717–721) says they "function the same way as SFX instruments".
- The byte layout comes from a community post written before release, not from zep ([BBS pid=140311](https://lexaloffle.com/bbs/?pid=140311), ridgek, 2024-01):
  - bytes 0–63: signed 8-bit samples;
  - byte 64: filters, as usual;
  - byte 65 bit 0: bass;
  - byte 66 bit 7: the waveform-instrument flag.
- zepto8, fake-08, nano9, pemsa, picolove and retro8 contain no waveform-instrument code (checked by searching each repo's audio code). The 0.2.7 changelog still lists waveform-instrument fixes.

### How Patterns advance

- From the manual (lines 663–683): a pattern holds 4 SFX indices. Bit 6 of an index mutes that channel. Bit 7 of channel bytes 0, 1 and 2 holds LOOP START, LOOP BACK and STOP. After a pattern, play goes to the next one, unless STOP is set, or LOOP BACK jumps back to the nearest LOOP START (or to pattern 0).
- **Pattern length.** zepto8's `set_music_pattern` ([sfx.cpp#L548-L592](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L548)) follows the manual's rule: "the pattern finishes playing when the left-most non-looping channel has finished playing". It scans channels 0→3, skipping muted ones.
  - The first channel without a loop (`loop_end <= loop_start`, or `loop_end == 0`) sets the length: `min(32, LEN) × speed` ticks. LEN is used when only the first loop value is set.
  - If every channel loops, the length is the longest `32 × speed`. The changelog matches this: 0.1.2 "music pattern length taken to be first non-looping channel's length"; 0.1.3 "pattern finished too early when all tracks set to looping".
  - If there are no channels, the music stops.
  - fake-08 copies this rule. pemsa (slowest channel × 32), picolove (fastest channel) and nano9 (longest channel) do not.
- **A LEN quirk.** zepto8 notes that LEN is not applied to the music SFX's own playback, only to the pattern length. "it's probably a bug" in PICO-8 ([sfx.cpp#L108-L113](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L108-L113)).
- The music clock advances `22050/183` ticks per second. A music SFX that is interrupted by `sfx()` resumes at the music offset divided by its speed.

## Licence and reuse

- **zepto8: WTFPL v2.** It allows anything, including relicensing a Rust port as MIT, Apache-2.0, GPL or proprietary, with no notice required (keeping a credit is still good practice). The licence is stated in each source file's header, not in a LICENSE file. fake-08's LICENSE.MD lists zepto8 as "WTFPL 2". Caveat: WTFPL is not OSI-approved, and some organisations will not accept it. That affects policy, not legality. Constants and formulas measured from PICO-8 exports are facts, and copying them is low risk under any licence.
- **fake-08: MIT.** Porting its fixes (REVERB level 2, vibrato) needs its copyright notice and the MIT permission notice in the tracker's source or third-party notices. This is compatible with every common licence.
- **nano9: MIT OR Apache-2.0.** Compatible, but there is little to take. Its audio is incomplete and tied to Bevy.
- **pemsa (MIT) and picolove (zlib).** Compatible, but less accurate. Nothing to gain.
- **retro8: GPL-3.0.** Only usable if picotracker is GPL-3. It is also inaccurate, so avoid it.
- None of these is PICO-8's code. Lexaloffle's synth is closed. Every reference is a clean-room reconstruction, fitted to PICO-8's output by ear and from WAV exports.

## Recommendation for the emulation

1. Port zepto8's synth, effects, filters, SFX-instrument and pattern logic to Rust. Take fake-08's REVERB-2 and vibrato fixes. Keep a credit to zepto8 (WTFPL) and include fake-08's MIT notice.
2. Build waveform instruments from zep's 0.2.6 description and the community byte layout. Treat them as unverified.
3. Test against WAV exports from real PICO-8 v0.2.7 (`export foo.wav` / `extcmd("audio_rec")`), not against another emulator. Even zepto8 lists open FIXMEs: music advance is driven by channel 0, and fades on channels above 0 are approximate ([sfx.cpp#L243-L250](https://github.com/samhocevar/zepto8/blob/b2929a4fe31e5765ef66d7794edd11086b9638b4/src/pico8/sfx.cpp#L243-L250)).

## Sources

- PICO-8 v0.2.7 manual, local copy, `docs/sources/pico-8_manual.txt` lines 565–721, 1347–1363, 1947–1985.
- PICO-8 changelog: https://www.lexaloffle.com/dl/docs/pico-8_changelog.txt
- zep, "PICO-8 0.2.6" (waveform instruments): https://www.lexaloffle.com/bbs/?tid=140421
- ridgek, waveform instrument byte layout (community, pre-release): https://lexaloffle.com/bbs/?pid=140311
- Forum report of the 183-sample tick: https://lexaloffle.com/bbs/?pid=91665
- zepto8 @ b2929a4: `src/synth.cpp`, `src/pico8/sfx.cpp`, `src/pico8/vm.h`, `src/filter.cpp`, `src/pico8/memory.h`
- fake-08 @ 814991a: `source/Audio.cpp`, `source/synth.cpp`, `LICENSE.MD`
- nano9 @ 723105f: `src/pico8/audio/{mod,music}.rs`, `Cargo.toml`, `compat.md`
- pemsa @ 6c13c58: `src/pemsa/audio/*.cpp`, `LICENSE`
- picolove @ 97075db: `main.lua`, `api.lua`, `LICENCE.md`
- retro8 @ ecee659: `src/vm/sound.cpp`, `LICENSE`
