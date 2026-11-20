use super::support::{TestNote, TestPattern, TestSfx};
use pikutracker_core::synth::{SAMPLES_PER_TICK, packed_filters};
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
const PHASER: u8 = 7;

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

#[derive(Clone, Debug)]
pub struct Case {
    pub name: &'static str,
    pub plays: Plays,
    pub ticks: u32,
    sfx: Vec<TestSfx>,
    patterns: Vec<TestPattern>,
}

impl Case {
    pub fn p8(&self) -> String {
        let start = match self.plays {
            Plays::Music => "music(0)".to_owned(),
            Plays::Sfx(slot) => format!("sfx({slot},0)"),
        };
        let frames = (self.ticks * SAMPLES_PER_TICK).div_ceil(SAMPLES_PER_FRAME) + SPARE_FRAMES;
        let mut text = format!(
            "{HEADER}__lua__\n\
             -- pikutracker reference case {name}: records {ticks} ticks\n\
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

const fn on_sfx_instrument(pitch: u8, instrument: u8, volume: u8, effect: u8) -> TestNote {
    TestNote {
        custom: true,
        ..note(pitch, instrument, volume, effect)
    }
}

const REST: TestNote = note(0, 0, 0, 0);

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

fn repeated(note: TestNote, count: usize) -> Vec<TestNote> {
    vec![note; count]
}

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

fn stepped_waveform() -> [u8; 64] {
    std::array::from_fn(|index| STEPPED_WAVEFORM[index / 8])
}

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

pub fn all() -> Vec<Case> {
    let mut cases = coverage();
    cases.extend(checked_behaviours());
    cases.extend(synth_fixes());
    cases.extend(waveform_instruments());
    cases.extend(last_measured_behaviours());
    cases.extend(custom_instruments_inside_a_dampened_sfx());
    cases.extend(every_pitch());
    cases.extend(phaser_second_wave());
    cases.extend(near_silence());
    cases
}

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

fn custom_instruments_inside_a_dampened_sfx() -> Vec<Case> {
    let played = |speed| sfx(&[on_sfx_instrument(C2, 0, 7, 0)], speed, 1, 0, dampen(2));
    let sfx_instrument = sfx(&repeated(note(C2, SAW, 5, 0), 32), 16, 0, 32, dampen(1));
    vec![
        CartBuilder::new()
            .instrument(
                0,
                waveform_instrument(stepped_waveform(), WAVEFORM_INSTRUMENT_SPEED, true, 0),
            )
            .sfx_case("waveform_instrument_inside_a_dampened_sfx", played(64), 84),
        CartBuilder::new().instrument(0, sfx_instrument).sfx_case(
            "sfx_instrument_ending_inside_a_dampened_sfx",
            played(4),
            12,
        ),
    ]
}

fn every_pitch() -> Vec<Case> {
    let rising = |from: u8| {
        let notes: Vec<TestNote> = (from..from + 32)
            .map(|pitch| note(pitch, TRIANGLE, 5, 0))
            .collect();
        plain(&notes, 16)
    };
    vec![
        sfx_case("every_pitch_from_0_to_31", rising(0), 520),
        sfx_case("every_pitch_from_32_to_63", rising(32), 520),
    ]
}

fn phaser_second_wave() -> Vec<Case> {
    let phaser = |pitch| note(pitch, PHASER, 5, 0);
    let quiet_noise = |pitch| note(pitch, NOISE, 1, 0);
    vec![sfx_case(
        "phaser_after_noise",
        plain(
            &[
                phaser(C3),
                phaser(G3),
                quiet_noise(C3),
                quiet_noise(C4),
                phaser(E3),
                phaser(E3),
                phaser(C3),
                phaser(C3),
            ],
            8,
        ),
        84,
    )]
}

fn near_silence() -> Vec<Case> {
    let echoed = repeated(note(C3, SQUARE, 5, 0), 8);
    let filters = packed_filters(false, false, 0, 2, 2);
    let silent_16 = with_len(&[], 8, 16);
    let triangle_then_saw: Vec<TestNote> = repeated(note(C2, TRIANGLE, 5, 0), 8)
        .into_iter()
        .chain(repeated(note(C2, SAW, 5, 0), 24))
        .collect();
    let around_silence: Vec<TestNote> = repeated(note(C3, SQUARE, 5, 0), 4)
        .into_iter()
        .chain(repeated(on_sfx_instrument(C3, TRIANGLE, 0, 0), 4))
        .chain(repeated(note(C3, SQUARE, 5, 0), 4))
        .collect();
    vec![
        CartBuilder::new().music_case(
            "shorter_channel_loops_on_a_silent_note_under_reverb_and_dampen",
            &[&[sfx(&echoed, 8, 8, 9, filters), silent_16]],
            136,
        ),
        CartBuilder::new().music_case(
            "shorter_channel_ends_at_len_under_reverb_and_dampen",
            &[&[silent_16, sfx(&echoed, 8, 8, 0, filters)]],
            136,
        ),
        CartBuilder::new()
            .instrument(TRIANGLE, sfx(&triangle_then_saw, 1, 0, 0, 0))
            .sfx_case(
                "silent_sfx_instrument_note_runs_the_waves_as_its_instrument_under_reverb",
                sfx(
                    &around_silence,
                    8,
                    12,
                    0,
                    packed_filters(false, false, 1, 1, 0),
                ),
                100,
            ),
    ]
}
