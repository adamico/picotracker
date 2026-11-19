//! The cases of the reference set. Each case is one driver Cart: it holds only the SFX and Patterns
//! the case plays, and its Lua records what PICO-8 plays to `recording.wav`, prints `ALLDONE` and
//! waits to be closed. PICO-8 crashes on long recording names, so the name is the same for every
//! case and the recording command renames the file.

use super::support::{TestNote, TestPattern, TestSfx};
use picotracker_core::synth::{SAMPLES_PER_TICK, packed_filters};
use std::fmt::Write;

pub const RECORDING: &str = "recording";
const HEADER: &str = "pico-8 cartridge // http://www.pico-8.com\nversion 43\n";
const SAMPLES_PER_FRAME: u32 = 735;
const SPARE_FRAMES: u32 = 15;
const FIRST_PLAYED_SLOT: u8 = 8;
const MARKER_TICKS: u32 = 8;

const C2: u8 = 24;
const E2: u8 = 28;
const G2: u8 = 31;
const C3: u8 = 36;
const E3: u8 = 40;
const G3: u8 = 43;
const C4: u8 = 48;

const TRIANGLE: u8 = 0;
const TILTED_SAW: u8 = 1;
const SAW: u8 = 2;
const SQUARE: u8 = 3;
const PULSE: u8 = 4;
const ORGAN: u8 = 5;
const NOISE: u8 = 6;

const SLIDE: u8 = 1;
const DROP: u8 = 3;
const ARPEGGIO_FAST: u8 = 6;
const ARPEGGIO_SLOW: u8 = 7;

const BIT_7: u8 = 0x80;
const WAVEFORM_INSTRUMENT_SPEED: u8 = 16;
const WAVEFORM_INSTRUMENT_BASS_SPEED: u8 = 17;
const STEPPED_WAVEFORM: [u8; 8] = [0x00, 0x20, 0x40, 0x7f, 0x80, 0xa0, 0xc0, 0xff];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plays {
    Music,
    Sfx(u8),
}

/// One case: a driver Cart, what it plays once recording begins and for how many Ticks the
/// recording is measured.
#[derive(Clone, Debug)]
pub struct Case {
    pub name: &'static str,
    pub plays: Plays,
    pub ticks: u32,
    sfx: Vec<TestSfx>,
    patterns: Vec<TestPattern>,
}

impl Case {
    /// The driver Cart as `.p8` text.
    pub fn p8(&self) -> String {
        let start = match self.plays {
            Plays::Music => "music(0)".to_owned(),
            Plays::Sfx(slot) => format!("sfx({slot},0)"),
        };
        let frames = (self.ticks * SAMPLES_PER_TICK).div_ceil(SAMPLES_PER_FRAME) + SPARE_FRAMES;
        let mut text = format!(
            "{HEADER}__lua__\n\
             -- picotracker reference case {name}: records {ticks} ticks\n\
             printh(\"VERSION \"..stat(5))\n\
             extcmd(\"set_filename\",\"{RECORDING}\")\n\
             extcmd(\"audio_rec\")\n\
             {start}\n\
             for i=1,{frames} do flip() end\n\
             extcmd(\"audio_end\",1)\n\
             printh(\"ALLDONE\")\n\
             while true do flip() end\n",
            name = self.name,
            ticks = self.ticks,
        );
        text.push_str("__sfx__\n");
        for sfx in &self.sfx {
            writeln!(text, "{}", sfx.line()).unwrap();
        }
        if !self.patterns.is_empty() {
            text.push_str("__music__\n");
            for pattern in &self.patterns {
                writeln!(text, "{}", pattern.line()).unwrap();
            }
        }
        text
    }
}

const fn note(pitch: u8, instrument: u8, volume: u8, effect: u8) -> TestNote {
    TestNote {
        pitch,
        instrument,
        volume,
        effect,
        custom: false,
    }
}

/// A note that plays the SFX Instrument in slot `instrument`.
const fn on_sfx_instrument(pitch: u8, instrument: u8, volume: u8, effect: u8) -> TestNote {
    TestNote {
        custom: true,
        ..note(pitch, instrument, volume, effect)
    }
}

const REST: TestNote = note(0, 0, 0, 0);

/// An SFX whose filter byte holds the editor bit and these filters.
fn sfx(notes: &[TestNote], speed: u8, loop_start: u8, loop_end: u8, filters: u8) -> TestSfx {
    let mut all = [REST; 32];
    all[..notes.len()].copy_from_slice(notes);
    TestSfx {
        editor_and_filters: 1 | filters,
        speed,
        loop_start,
        loop_end,
        notes: all,
    }
}

fn plain(notes: &[TestNote], speed: u8) -> TestSfx {
    sfx(notes, speed, 0, 0, 0)
}

fn with_len(notes: &[TestNote], speed: u8, len: u8) -> TestSfx {
    sfx(notes, speed, len, 0, 0)
}

fn looping(notes: &[TestNote], speed: u8, start: u8, end: u8) -> TestSfx {
    sfx(notes, speed, start, end, 0)
}

fn filtered(notes: &[TestNote], speed: u8, filters: u8) -> TestSfx {
    sfx(notes, speed, 0, 0, filters)
}

const fn noiz() -> u8 {
    packed_filters(true, false, 0, 0, 0)
}

const fn buzz() -> u8 {
    packed_filters(false, true, 0, 0, 0)
}

const fn detune(level: u8) -> u8 {
    packed_filters(false, false, level, 0, 0)
}

const fn reverb(level: u8) -> u8 {
    packed_filters(false, false, 0, level, 0)
}

const fn dampen(level: u8) -> u8 {
    packed_filters(false, false, 0, 0, level)
}

/// The same note `count` times.
fn repeated(note: TestNote, count: usize) -> Vec<TestNote> {
    vec![note; count]
}

/// A Waveform Instrument: 64 signed samples in the note bytes, flagged by bit 7 of byte 66.
fn waveform_instrument(samples: [u8; 64], speed: u8, flagged: bool, filters: u8) -> TestSfx {
    let mut notes = [REST; 32];
    for (index, note) in notes.iter_mut().enumerate() {
        let word = u16::from_le_bytes([samples[index * 2], samples[index * 2 + 1]]);
        *note = TestNote {
            pitch: (word & 0x3f) as u8,
            instrument: (word >> 6 & 7) as u8,
            volume: (word >> 9 & 7) as u8,
            effect: (word >> 12 & 7) as u8,
            custom: word & 0x8000 != 0,
        };
    }
    TestSfx {
        editor_and_filters: 1 | filters,
        speed,
        loop_start: if flagged { BIT_7 } else { 0 },
        loop_end: 0,
        notes,
    }
}

/// Eight steps of eight samples each, from silence up to the highest and lowest values.
fn stepped_waveform() -> [u8; 64] {
    std::array::from_fn(|index| STEPPED_WAVEFORM[index / 8])
}

/// Builds one case's Cart: instruments in slots 0 to 7, played SFX from slot 8.
struct CartBuilder {
    sfx: Vec<TestSfx>,
    patterns: Vec<TestPattern>,
}

impl CartBuilder {
    fn new() -> Self {
        Self {
            sfx: vec![plain(&[], 16); usize::from(FIRST_PLAYED_SLOT)],
            patterns: Vec::new(),
        }
    }

    fn instrument(mut self, slot: u8, instrument: TestSfx) -> Self {
        self.sfx[usize::from(slot)] = instrument;
        self
    }

    /// Adds an SFX after the instruments, or finds an identical one, and gives its slot.
    fn add(&mut self, sfx: TestSfx) -> u8 {
        let known = self.sfx[usize::from(FIRST_PLAYED_SLOT)..]
            .iter()
            .position(|known| known.line() == sfx.line());
        let index = known.map_or_else(
            || {
                self.sfx.push(sfx);
                self.sfx.len() - 1
            },
            |index| index + usize::from(FIRST_PLAYED_SLOT),
        );
        index as u8
    }

    /// Plays one SFX straight through `sfx()`.
    fn sfx_case(mut self, name: &'static str, played: TestSfx, ticks: u32) -> Case {
        let slot = self.add(played);
        Case {
            name,
            plays: Plays::Sfx(slot),
            ticks,
            sfx: self.sfx,
            patterns: self.patterns,
        }
    }

    /// Plays these Patterns through `music(0)`, with STOP on the last one.
    fn music_case(mut self, name: &'static str, patterns: &[&[TestSfx]], ticks: u32) -> Case {
        for channels in patterns {
            let mut pattern = TestPattern::default();
            for (channel, played) in channels.iter().enumerate() {
                pattern.channels[channel] = Some(self.add(*played));
            }
            self.patterns.push(pattern);
        }
        self.patterns.last_mut().unwrap().stop = true;
        Case {
            name,
            plays: Plays::Music,
            ticks,
            sfx: self.sfx,
            patterns: self.patterns,
        }
    }

    /// Plays one Pattern and then a short audible marker, so that the recording shows when the
    /// Pattern ended.
    fn length_case(self, name: &'static str, channels: &[TestSfx], pattern_ticks: u32) -> Case {
        let marker = with_len(&[note(C4, SQUARE, 5, 0), note(C4, SQUARE, 5, 0)], 4, 2);
        self.music_case(
            name,
            &[channels, &[marker]],
            pattern_ticks + MARKER_TICKS + 24,
        )
    }
}

fn sfx_case(name: &'static str, played: TestSfx, ticks: u32) -> Case {
    CartBuilder::new().sfx_case(name, played, ticks)
}

fn length_case(name: &'static str, channels: &[TestSfx], pattern_ticks: u32) -> Case {
    CartBuilder::new().length_case(name, channels, pattern_ticks)
}

/// Every case, in the order they are recorded.
pub fn all() -> Vec<Case> {
    let mut cases = coverage();
    cases.extend(checked_behaviours());
    cases.extend(synth_fixes());
    cases.extend(waveform_instruments());
    cases.extend(last_measured_behaviours());
    cases
}

/// The playback spike's coverage Cart: 23 SFX and 5 Patterns, one case each.
fn coverage() -> Vec<Case> {
    let effects: [(u8, [u8; 4]); 8] = [
        (0, [C3; 4]),
        (1, [G3; 4]),
        (2, [C3; 4]),
        (3, [C4; 4]),
        (4, [E3; 4]),
        (5, [E3; 4]),
        (6, [C3, E3, G3, C4]),
        (7, [C3, E3, G3, C4]),
    ];
    let through_effects = |waveform: u8| {
        let notes: Vec<TestNote> = effects
            .iter()
            .flat_map(|(effect, pitches)| pitches.map(|pitch| note(pitch, waveform, 5, *effect)))
            .collect();
        plain(&notes, 10)
    };
    let melody = [C3, E3, G3, C4, G3, E3, C3, 0];
    let saw_melody: Vec<TestNote> = melody
        .iter()
        .cycle()
        .take(32)
        .map(|&pitch| note(pitch, SAW, if pitch == 0 { 0 } else { 5 }, 0))
        .collect();
    let every_waveform: Vec<TestNote> = melody
        .iter()
        .cycle()
        .take(32)
        .enumerate()
        .map(|(index, &pitch)| note(pitch, (index / 4 % 8) as u8, 5, 0))
        .collect();
    let noise_octaves: Vec<TestNote> = [12, 24, 36, 48]
        .iter()
        .cycle()
        .take(32)
        .map(|&pitch| note(pitch, NOISE, 5, 0))
        .collect();
    let instrument_source = looping(
        &[
            note(C3, SQUARE, 6, 0),
            note(C3 + 12, SQUARE, 4, 0),
            note(C3, ORGAN, 5, 0),
            note(C3 + 7, ORGAN, 3, 0),
        ],
        4,
        0,
        4,
    );
    let on_instrument = |pitch, effect| on_sfx_instrument(pitch, 0, 5, effect);
    let mut instrument_notes = repeated(on_instrument(C3, 0), 4);
    instrument_notes.push(REST);
    instrument_notes.extend(repeated(on_instrument(C3, 0), 3));
    instrument_notes.extend([on_instrument(E3, 0), on_instrument(G3, 0)]);
    instrument_notes.extend(repeated(on_instrument(C4, SLIDE), 2));
    instrument_notes.extend(repeated(on_instrument(C3, DROP), 4));
    let arpeggio: Vec<TestNote> = [C3, E3, G3, C4]
        .iter()
        .cycle()
        .take(16)
        .map(|&pitch| note(pitch, PULSE, 5, ARPEGGIO_FAST))
        .collect();
    let rising: Vec<TestNote> = (0..32)
        .map(|index| note(C3 + index % 12, TILTED_SAW, 4, 0))
        .collect();
    let sparse: Vec<TestNote> = (0..32)
        .map(|index| {
            if index % 4 == 0 {
                note(G3, SQUARE, 4, 0)
            } else {
                REST
            }
        })
        .collect();

    let sfx: Vec<(&'static str, TestSfx, u32)> = vec![
        ("coverage_sfx08", through_effects(TRIANGLE), 340),
        ("coverage_sfx09", through_effects(TILTED_SAW), 340),
        ("coverage_sfx10", through_effects(SAW), 340),
        ("coverage_sfx11", through_effects(SQUARE), 340),
        ("coverage_sfx12", through_effects(PULSE), 340),
        ("coverage_sfx13", through_effects(ORGAN), 340),
        ("coverage_sfx14", through_effects(NOISE), 340),
        ("coverage_sfx15", through_effects(7), 340),
        ("coverage_sfx16", filtered(&noise_octaves, 8, noiz()), 276),
        ("coverage_sfx17", filtered(&noise_octaves, 8, buzz()), 276),
        ("coverage_sfx18", filtered(&every_waveform, 8, buzz()), 276),
        (
            "coverage_sfx19",
            filtered(&every_waveform, 8, detune(1)),
            276,
        ),
        (
            "coverage_sfx20",
            filtered(&every_waveform, 8, detune(2)),
            276,
        ),
        ("coverage_sfx21", filtered(&saw_melody, 8, reverb(1)), 276),
        ("coverage_sfx22", filtered(&saw_melody, 8, reverb(2)), 276),
        ("coverage_sfx23", filtered(&saw_melody, 8, dampen(1)), 276),
        ("coverage_sfx24", filtered(&saw_melody, 8, dampen(2)), 276),
        ("coverage_sfx25", plain(&instrument_notes, 12), 236),
        ("coverage_sfx26", plain(&arpeggio, 8), 148),
        ("coverage_sfx27", plain(&arpeggio, 9), 164),
        ("coverage_sfx28", looping(&rising, 12, 0, 8), 480),
        ("coverage_sfx29", with_len(&sparse, 12, 12), 164),
        (
            "coverage_sfx30",
            looping(&repeated(note(C4, TRIANGLE, 4, 0), 32), 6, 0, 16),
            480,
        ),
    ];
    let with_instrument = || CartBuilder::new().instrument(0, instrument_source);
    let mut cases: Vec<Case> = sfx
        .into_iter()
        .map(|(name, played, ticks)| with_instrument().sfx_case(name, played, ticks))
        .collect();

    let rising_loop = looping(&rising, 12, 0, 8);
    let looping_triangle = looping(&repeated(note(C4, TRIANGLE, 4, 0), 32), 6, 0, 16);
    let music: Vec<(&'static str, Vec<TestSfx>, u32)> = vec![
        (
            "coverage_music0",
            (0..4).map(through_effects).collect(),
            340,
        ),
        (
            "coverage_music1",
            (4..8).map(through_effects).collect(),
            340,
        ),
        (
            "coverage_music2",
            vec![rising_loop, with_len(&sparse, 12, 12)],
            164,
        ),
        ("coverage_music3", vec![rising_loop, looping_triangle], 404),
        (
            "coverage_music4",
            vec![
                plain(&instrument_notes, 12),
                filtered(&saw_melody, 8, reverb(1)),
                plain(&arpeggio, 8),
            ],
            420,
        ),
    ];
    cases.extend(
        music
            .into_iter()
            .map(|(name, channels, ticks)| with_instrument().music_case(name, &[&channels], ticks)),
    );
    cases
}

/// The behaviours checked in "Check open PICO-8 behaviours against real PICO-8".
fn checked_behaviours() -> Vec<Case> {
    let audible = |pitch, count| repeated(note(pitch, SQUARE, 5, 0), count);
    let silent = |count| repeated(note(C3, SQUARE, 0, 0), count);
    let silent_loop = |count, speed| {
        let mut notes = audible(C3, count);
        notes.push(REST);
        looping(&notes, speed, count as u8, count as u8 + 1)
    };
    let silent_16 = with_len(&silent(16), 8, 16);
    let silent_32 = plain(&silent(32), 8);
    let repeated_triangle = with_len(&repeated(note(C3, TRIANGLE, 5, 0), 8), 8, 8);
    let spd_0_ramp: Vec<TestNote> = (0..32)
        .map(|index| note(24 + index, SQUARE, 5, 0))
        .collect();
    vec![
        sfx_case("repeated_note_does_not_click", repeated_triangle, 84),
        CartBuilder::new().music_case(
            "repeated_note_does_not_click_across_patterns",
            &[&[repeated_triangle], &[repeated_triangle]],
            148,
        ),
        length_case(
            "silent_looping_step_after_1_step",
            &[silent_loop(1, 8), silent_16],
            128,
        ),
        length_case(
            "silent_looping_step_after_8_steps",
            &[silent_loop(8, 8), silent_16],
            128,
        ),
        length_case(
            "silent_looping_step_after_31_steps",
            &[silent_loop(31, 8), silent_32],
            256,
        ),
        length_case(
            "silent_looping_step_at_a_faster_speed",
            &[silent_loop(8, 4), silent_16],
            128,
        ),
        length_case(
            "silent_looping_steps_on_two_channels",
            &[
                silent_loop(8, 8),
                silent_loop(4, 8),
                with_len(&audible(G3, 16), 8, 16),
            ],
            128,
        ),
        length_case(
            "loop_past_note_31_does_not_set_the_length",
            &[looping(&audible(C3, 32), 4, 32, 33), silent_32],
            256,
        ),
        length_case(
            "len_40_sets_the_pattern_length",
            &[with_len(&audible(C3, 32), 8, 40)],
            320,
        ),
        length_case(
            "len_64_sets_the_pattern_length",
            &[with_len(&audible(C3, 32), 8, 64)],
            512,
        ),
        length_case(
            "len_on_another_channel_keeps_playing",
            &[silent_32, with_len(&audible(G3, 32), 8, 8)],
            256,
        ),
        sfx_case("sfx_stops_at_len", with_len(&audible(G3, 32), 8, 8), 84),
        length_case(
            "loop_start_past_loop_end_plays_every_note",
            &[looping(&audible(C3, 32), 8, 8, 4)],
            256,
        ),
        length_case(
            "bit_7_sfx_beside_another_channel",
            &[
                sfx(&repeated(note(E3, SAW, 5, 0), 16), 8, BIT_7, 0, 0),
                with_len(&audible(C3, 16), 8, 16),
            ],
            128,
        ),
        sfx_case("spd_0_plays_at_speed_1", plain(&spd_0_ramp, 0), 52),
        length_case(
            "spd_0_in_a_pattern_plays_at_speed_1",
            &[plain(&spd_0_ramp, 0)],
            32,
        ),
    ]
}

/// One case for each of the four synth fixes, plus the one-level nesting that already matches.
fn synth_fixes() -> Vec<Case> {
    let stepped = looping(
        &[
            note(C2, TILTED_SAW, 6, 0),
            note(E2, TILTED_SAW, 6, 0),
            note(G2, TILTED_SAW, 6, 0),
            note(C3, TILTED_SAW, 6, 0),
            note(C2, TILTED_SAW, 6, 0),
            note(E2, TILTED_SAW, 6, 0),
            note(G2, TILTED_SAW, 6, 0),
            note(C3, TILTED_SAW, 6, 0),
        ],
        6,
        0,
        8,
    );
    let steady = looping(&[note(C2, TRIANGLE, 6, 0)], 8, 0, 1);
    let inner_arpeggio = |effect| {
        let notes: Vec<TestNote> = [C2, E2, G2, C3]
            .iter()
            .cycle()
            .take(8)
            .map(|&pitch| note(pitch, TILTED_SAW, 6, effect))
            .collect();
        looping(&notes, 16, 0, 8)
    };
    let nesting = looping(
        &[
            on_sfx_instrument(C2, 1, 6, 0),
            on_sfx_instrument(G2, 1, 6, 0),
            note(C3, TILTED_SAW, 6, 0),
            note(E3, TILTED_SAW, 6, 0),
        ],
        16,
        0,
        4,
    );
    let instruments = || {
        CartBuilder::new()
            .instrument(0, stepped)
            .instrument(1, steady)
            .instrument(2, inner_arpeggio(ARPEGGIO_FAST))
            .instrument(3, nesting)
            .instrument(4, inner_arpeggio(ARPEGGIO_SLOW))
    };
    let on = |pitch, instrument, effect| on_sfx_instrument(pitch, instrument, 5, effect);
    let rising = [C3, E3, G3, C4];
    let saw_rising: Vec<TestNote> = rising.iter().map(|&pitch| note(pitch, SAW, 5, 0)).collect();
    let saw_then_silence = {
        let mut notes = saw_rising.clone();
        notes.extend(repeated(REST, 12));
        notes
    };
    let silent_16 = with_len(&repeated(note(C3, SQUARE, 0, 0), 16), 16, 16);
    vec![
        instruments().sfx_case(
            "drop_on_an_sfx_instrument_keeps_the_pitch",
            with_len(&rising.map(|pitch| on(pitch, 0, DROP)), 32, 4),
            148,
        ),
        instruments().sfx_case(
            "drop_on_an_sfx_instrument_retriggers_the_same_pitch",
            with_len(&repeated(on(C3, 0, DROP), 6), 32, 6),
            212,
        ),
        sfx_case(
            "slide_is_linear_in_pitch",
            with_len(
                &[
                    note(C3, TRIANGLE, 5, 0),
                    note(G3, TRIANGLE, 5, SLIDE),
                    note(C3, TRIANGLE, 5, SLIDE),
                    note(C4, TRIANGLE, 5, SLIDE),
                ],
                32,
                4,
            ),
            148,
        ),
        sfx_case(
            "reverb_1_rings_through_silent_steps",
            sfx(&saw_then_silence, 16, 16, 0, reverb(1)),
            276,
        ),
        sfx_case(
            "reverb_2_rings_through_silent_steps",
            sfx(&saw_then_silence, 16, 16, 0, reverb(2)),
            276,
        ),
        length_case(
            "reverb_2_rings_after_len_inside_a_pattern",
            &[silent_16, sfx(&saw_rising, 16, 4, 0, reverb(2))],
            256,
        ),
        sfx_case(
            "reverb_2_is_cut_when_sfx_ends_at_len",
            sfx(&saw_rising, 16, 4, 0, reverb(2)),
            84,
        ),
        instruments().sfx_case(
            "arpeggio_inside_an_sfx_instrument_follows_the_played_pitch",
            with_len(&[on(C3, 2, 0)], 96, 1),
            116,
        ),
        instruments().sfx_case(
            "slow_arpeggio_inside_an_sfx_instrument_follows_the_played_pitch",
            with_len(&[on(G3, 4, 0)], 96, 1),
            116,
        ),
        instruments().sfx_case(
            "sfx_instrument_nested_one_level",
            with_len(&[on(C3, 3, 0)], 96, 1),
            116,
        ),
    ]
}

/// Waveform Instruments, as "Check open PICO-8 behaviours against real PICO-8" found them.
fn waveform_instruments() -> Vec<Case> {
    let case = |name, instrument: TestSfx, pitch| {
        CartBuilder::new().instrument(0, instrument).sfx_case(
            name,
            with_len(&[on_sfx_instrument(pitch, 0, 7, 0)], 64, 1),
            84,
        )
    };
    let waveform =
        |speed, flagged, filters| waveform_instrument(stepped_waveform(), speed, flagged, filters);
    vec![
        case(
            "waveform_instrument_at_c2",
            waveform(WAVEFORM_INSTRUMENT_SPEED, true, 0),
            C2,
        ),
        case(
            "waveform_instrument_at_c3",
            waveform(WAVEFORM_INSTRUMENT_SPEED, true, 0),
            C3,
        ),
        case(
            "waveform_instrument_with_bass",
            waveform(WAVEFORM_INSTRUMENT_BASS_SPEED, true, 0),
            C3,
        ),
        case(
            "waveform_instrument_bytes_without_the_flag",
            waveform(WAVEFORM_INSTRUMENT_SPEED, false, 0),
            C2,
        ),
        case(
            "waveform_instrument_with_detune",
            waveform(WAVEFORM_INSTRUMENT_SPEED, true, detune(1)),
            C2,
        ),
        case(
            "waveform_instrument_with_reverb",
            waveform(WAVEFORM_INSTRUMENT_SPEED, true, reverb(1)),
            C2,
        ),
        case(
            "waveform_instrument_with_dampen",
            waveform(WAVEFORM_INSTRUMENT_SPEED, true, dampen(2)),
            C2,
        ),
        case(
            "waveform_instrument_ignores_its_own_noiz_and_buzz",
            waveform(WAVEFORM_INSTRUMENT_SPEED, true, noiz() | buzz()),
            C2,
        ),
    ]
}

/// The behaviours measured in "Measure the last unchecked behaviours on real PICO-8".
fn last_measured_behaviours() -> Vec<Case> {
    let pitches = [C2, E2, G2, C3, E3, G3, C4, C3];
    let on_waveform: Vec<TestNote> = pitches
        .iter()
        .flat_map(|&pitch| repeated(on_sfx_instrument(pitch, 0, 5, 0), 2))
        .collect();
    let audible = |pitch, count| repeated(note(pitch, SQUARE, 5, 0), count);
    let bit_7 = |speed| sfx(&audible(C3, 32), speed, BIT_7, 0, 0);
    let looping_tilted = |speed| looping(&repeated(note(G3, TILTED_SAW, 4, 0), 32), speed, 0, 8);
    vec![
        CartBuilder::new()
            .instrument(
                0,
                waveform_instrument(stepped_waveform(), WAVEFORM_INSTRUMENT_SPEED, true, 0),
            )
            .sfx_case(
                "waveform_instrument_ignores_noiz_and_buzz_on_the_playing_sfx",
                sfx(&on_waveform, 8, 16, 0, noiz() | buzz()),
                148,
            ),
        length_case(
            "bit_7_sfx_is_silent_after_note_31",
            &[
                with_len(&repeated(note(C3, SQUARE, 0, 0), 32), 8, 64),
                bit_7(8),
            ],
            512,
        ),
        length_case(
            "bit_7_sfx_does_not_set_the_length_beside_looping_channels",
            &[bit_7(4), looping_tilted(16)],
            512,
        ),
        length_case(
            "looping_channels_play_32_times_the_slowest_speed",
            &[looping_tilted(16), looping_tilted(4)],
            512,
        ),
    ]
}
