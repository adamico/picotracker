//! Synth: turns sound data into samples at 22,050 Hz.
//!
//! This module is a Rust port of the PICO-8 synth in zepto8 at commit `b2929a4` (`src/synth.cpp`,
//! `src/filter.cpp` and `src/pico8/sfx.cpp`), with the REVERB 2 fix from fake-08 at commit
//! `814991a` (`source/Audio.cpp`, lines 764 and 765). The notices of both projects follow.
//!
//! Where the PICO-8 reference set measures real PICO-8 differently, the synth follows PICO-8
//! instead of the port: the pitch of each note in whole phase units, the phase a voice starts from
//! and keeps, the crossfade between notes, the phase of saw, square and pulse, the level of the
//! whole output, the saw, noise and Waveform Instruments, BUZZ and DETUNE, the steps of vibrato
//! and slide, the retrigger of an SFX Instrument under drop, and silence on a silent note of an
//! SFX Instrument.
//!
//! zepto8, `src/synth.cpp`:
//!
//! ```text
//! ZEPTO-8 — Fantasy console emulator
//!
//! Copyright © 2017—2020 Sam Hocevar <sam@hocevar.net>
//!
//! This program is free software. It comes without any warranty, to
//! the extent permitted by applicable law. You can redistribute it
//! and/or modify it under the terms of the Do What the Fuck You Want
//! to Public License, Version 2, as published by the WTFPL Task Force.
//! See http://www.wtfpl.net/ for more details.
//! ```
//!
//! zepto8, `src/filter.cpp`:
//!
//! ```text
//! ZEPTO-8 — Fantasy console emulator
//!
//! Copyright © 2017–2024 Sam Hocevar <sam@hocevar.net>
//!
//! This program is free software. It comes without any warranty, to
//! the extent permitted by applicable law. You can redistribute it
//! and/or modify it under the terms of the Do What the Fuck You Want
//! to Public License, Version 2, as published by the WTFPL Task Force.
//! See http://www.wtfpl.net/ for more details.
//! ```
//!
//! zepto8, `src/pico8/sfx.cpp`:
//!
//! ```text
//! ZEPTO-8 — Fantasy console emulator
//!
//! Copyright © 2016–2024 Sam Hocevar <sam@hocevar.net>
//!
//! This program is free software. It comes without any warranty, to
//! the extent permitted by applicable law. You can redistribute it
//! and/or modify it under the terms of the Do What the Fuck You Want
//! to Public License, Version 2, as published by the WTFPL Task Force.
//! See http://www.wtfpl.net/ for more details.
//! ```
//!
//! FAKE-08, <https://github.com/jtothebell/fake-08>, MIT license, from its `LICENSE.MD`:
//!
//! ```text
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

pub const SAMPLE_RATE: u32 = 22_050;
pub const SAMPLES_PER_TICK: u32 = 183;
pub const NOTES: usize = 32;
pub const WAVEFORM_SAMPLES: usize = 64;

const RATE: f64 = SAMPLE_RATE as f64;
const NOIZ_AND_BUZZ: u8 = 0x6;
const WAVEFORM_FULL_SCALE: f32 = 192.0;
const WAVEFORM_FIRST_VOICE: f32 = 0.7;
const DETUNE_FIRST_VOICE: f32 = 5.0 / 6.0;
const DETUNE_SECOND_VOICE: f32 = 5.0 / 12.0;
const REVERB_1_SAMPLES: usize = 2 * SAMPLES_PER_TICK as usize;
const REVERB_2_SAMPLES: usize = 4 * SAMPLES_PER_TICK as usize;
const NOISE_SEED: u32 = 0x1234_5678;
const MIDDLE_KEY: i16 = 24;
const NO_INSTRUMENT: u8 = 0xff;
const NO_KEY: i16 = i16::MAX;
const CROSSFADE_SAMPLES: f32 = 64.0;
const TICK_FLOOR_TOLERANCE: f64 = 1e-9;
const VIBRATO_STEPS: f64 = 8.0;
const VIBRATO_TICKS_PER_STEP: f64 = 2.0;
const VIBRATO_SEMITONES: f32 = 0.27;
const SAW_LEVEL: f32 = 2.0 / 3.0;
const OUTPUT_GAIN: f32 = 0.9841;
const NOISE_LEVEL: f32 = 0.844;
const NOIZ_LEVEL: f32 = 0.25;
const NOIZ_HOLD_PITCH: i16 = 64;
const PHASE_UNITS_PER_CYCLE: f32 = 65_536.0;
const TABLE_OCTAVE: i32 = 3;
const MAX_OCTAVE_SHIFT: i32 = 32;
const OCTAVE_PHASE_UNITS: [u32; 12] = [
    1554, 1646, 1744, 1848, 1958, 2074, 2199, 2330, 2469, 2615, 2770, 2924,
];

const FX_SLIDE: u8 = 1;
const FX_VIBRATO: u8 = 2;
const FX_DROP: u8 = 3;
const FX_FADE_IN: u8 = 4;
const FX_FADE_OUT: u8 = 5;
const FX_ARP_FAST: u8 = 6;
const FX_ARP_SLOW: u8 = 7;

const INST_TRIANGLE: u8 = 0;
const INST_TILTED_SAW: u8 = 1;
const INST_SAW: u8 = 2;
const INST_SQUARE: u8 = 3;
const INST_PULSE: u8 = 4;
const INST_ORGAN: u8 = 5;
const INST_NOISE: u8 = 6;
const INST_PHASER: u8 = 7;

/// One note of a sound. The pitch is wider than PICO-8's 0–63, so that a transposed Step can play
/// at its true pitch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Note {
    pub pitch: i16,
    pub instrument: u8,
    pub custom: bool,
    pub volume: u8,
    pub effect: u8,
}

/// The sound data the synth plays, laid out like one PICO-8 SFX.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sound {
    pub notes: [Note; NOTES],
    pub speed: u8,
    pub filters: u8,
    pub loop_start: u8,
    pub loop_end: u8,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            notes: [Note::default(); NOTES],
            speed: 1,
            filters: 0,
            loop_start: 0,
            loop_end: 0,
        }
    }
}

impl Sound {
    /// The LEN a sound ends at: its loop start, when it has no loop end.
    fn len(&self) -> Option<u8> {
        (self.loop_end == 0 && self.loop_start > 0).then_some(self.loop_start)
    }
}

/// A Custom Instrument made of one looping wave of signed samples. Its filters are packed like
/// byte 64 of a PICO-8 SFX, and NOIZ and BUZZ have no effect on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waveform {
    pub samples: [i8; WAVEFORM_SAMPLES],
    pub bass: bool,
    pub filters: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instrument {
    Sfx(Sound),
    Waveform(Waveform),
}

/// The Custom Instruments a voice can play, by the instrument number their notes hold.
pub type Instruments = BTreeMap<u8, Instrument>;

/// Packs the filters the way byte 64 of a PICO-8 SFX holds them.
pub const fn packed_filters(noiz: bool, buzz: bool, detune: u8, reverb: u8, dampen: u8) -> u8 {
    (noiz as u8) * 2 + (buzz as u8) * 4 + detune * 8 + reverb * 24 + dampen * 72
}

/// How far through its cycle a pitch moves in one sample, as PICO-8 plays it. PICO-8 keeps a phase
/// in 65,536ths of a cycle and moves it by a whole number of them each sample, taken from one
/// octave of pitches and halved or doubled for each octave below or above it. Measured on PICO-8,
/// these lie a little below equal temperament, and B lies 7 cents below.
fn pitch_increment(pitch: i16) -> f32 {
    let octaves_up = i32::from(pitch.div_euclid(12)) - TABLE_OCTAVE;
    let phase_units = u64::from(OCTAVE_PHASE_UNITS[pitch.rem_euclid(12) as usize]);
    let phase_units = if octaves_up >= 0 {
        phase_units << octaves_up.min(MAX_OCTAVE_SHIFT)
    } else {
        phase_units >> (-octaves_up).min(MAX_OCTAVE_SHIFT)
    };
    phase_units as f32 / PHASE_UNITS_PER_CYCLE
}

/// The phase step of a pitch between two whole pitches. Measured on PICO-8, a slide moves through
/// pitches in a straight line, and between two whole pitches PICO-8 takes the whole number of phase
/// units nearest the straight line between their steps.
fn between_pitches_increment(pitch: f32) -> f32 {
    let whole = pitch.floor() as i16;
    let units = mix(
        pitch_increment(whole),
        pitch_increment(whole + 1),
        pitch - f32::from(whole),
    ) * PHASE_UNITS_PER_CYCLE;
    units.round() / PHASE_UNITS_PER_CYCLE
}

/// How far through their cycles a voice's waves are. The phaser and DETUNE mix in a second wave
/// that keeps a phase of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Phase {
    main: f64,
    second: f64,
}

impl Phase {
    const START: Self = Self {
        main: 0.0,
        second: 0.0,
    };

    fn at(cycles: f64) -> Self {
        Self {
            main: cycles,
            second: cycles,
        }
    }
}

impl std::ops::MulAssign<f64> for Phase {
    fn mul_assign(&mut self, factor: f64) {
        self.main *= factor;
        self.second *= factor;
    }
}

impl std::ops::RemAssign<f64> for Phase {
    fn rem_assign(&mut self, cycles: f64) {
        self.main %= cycles;
        self.second %= cycles;
    }
}

/// How far the second wave moves for each whole phase unit the main wave moves, as a fraction,
/// measured on PICO-8. Without DETUNE it keeps up with the main wave, except on the phaser. Under
/// DETUNE it stands still through noise, though PICO-8's noise moves it without DETUNE in a way
/// not yet matched. DETUNE 2 reads it twice over, except on the triangle and the phaser.
fn second_wave_ratio(instrument: u8, detune: u8) -> (u32, u32) {
    match (instrument, detune) {
        (INST_PHASER, 0) => (127, 128),
        (INST_PHASER, 1) => (125, 128),
        (INST_PHASER, _) => (254, 128),
        (_, 0) => (1, 1),
        (INST_NOISE, _) => (0, 1),
        (INST_TRIANGLE, 1) => (193, 256),
        (INST_TRIANGLE, _) => (3, 2),
        _ => (255, 256),
    }
}

/// A phase moved on by one sample of an increment, in PICO-8's whole phase units. The second wave
/// moves by its ratio of those units, rounded down to a whole unit.
fn advanced(phi: Phase, increment: f32, instrument: u8, filters: u8) -> Phase {
    let units = (increment * PHASE_UNITS_PER_CYCLE).floor();
    let (numerator, denominator) = second_wave_ratio(instrument, detune_level(filters));
    let second_units = (f64::from(units) * f64::from(numerator) / f64::from(denominator)).floor();
    let cycles = |units: f64| units / f64::from(PHASE_UNITS_PER_CYCLE);
    Phase {
        main: phi.main + cycles(f64::from(units)),
        second: phi.second + cycles(second_units),
    }
}

fn detune_level(filters: u8) -> u8 {
    (filters / 8) % 3
}

fn reverb_level(filters: u8) -> u8 {
    (filters / 24) % 3
}

fn dampen_level(filters: u8) -> u8 {
    (filters / 72) % 3
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The noise source: xorshift32 from a fixed seed, so that renders repeat exactly.
#[derive(Clone, Copy, Debug)]
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SynthParam {
    instrument: u8,
    custom: bool,
    filters: u8,
    key: i16,
    increment: f32,
    volume: f32,
    phi: Phase,
    last_advance: f64,
    last_sample: f32,
    held_samples: i16,
    wave: Option<[i8; WAVEFORM_SAMPLES]>,
}

/// One cycle of a Waveform Instrument, read between its samples by linear interpolation.
fn interpolated(wave: &[i8; WAVEFORM_SAMPLES], t: f32) -> f32 {
    let position = t * WAVEFORM_SAMPLES as f32;
    let index = (position as usize).min(WAVEFORM_SAMPLES - 1);
    let next = (index + 1) % WAVEFORM_SAMPLES;
    let between = mix(
        f32::from(wave[index]),
        f32::from(wave[next]),
        position - index as f32,
    );
    between / WAVEFORM_FULL_SCALE
}

/// A wave rising from -1 to 1 through the first `a` of the cycle and falling back after it.
fn tilted(a: f32, t: f32) -> f32 {
    if t < a {
        2.0 * t / a - 1.0
    } else {
        2.0 * (1.0 - t) / (1.0 - a) - 1.0
    }
}

/// The triangle from -1 to 1, which BUZZ mixes with a quarter of a tilted wave.
fn triangle(t: f32, buzz: bool) -> f32 {
    let shape = 1.0 - (4.0 * t - 2.0).abs();
    if buzz {
        shape * 0.75 + tilted(0.875, t) * 0.25
    } else {
        shape
    }
}

/// PICO-8's square and pulse: low through the cycle and high for its last `duty`.
fn high_for_the_last(duty: f32, t: f32) -> f32 {
    if t < 1.0 - duty { -0.25 } else { 0.25 }
}

fn waveform(p: &mut SynthParam, rng: &mut Rng) -> f32 {
    let advance = p.phi.main;
    let t = (advance % 1.0) as f32;
    if let Some(wave) = &p.wave {
        return interpolated(wave, t);
    }
    let noiz = p.filters & 0x2 != 0;
    let buzz = p.filters & 0x4 != 0;
    match p.instrument {
        INST_TRIANGLE => triangle(t, buzz) * 0.5,
        INST_TILTED_SAW => tilted(if buzz { 0.9375 } else { 0.875 }, t) * 0.5,
        INST_SAW => {
            // PICO-8's saw rises through the whole cycle and drops as it starts.
            let advance = advance + 0.5;
            let t = (advance % 1.0) as f32;
            let mut shape = if t < 0.5 { t } else { t - 1.0 };
            if buzz {
                // Measured on PICO-8: BUZZ lowers the saw by more on every other cycle, counted
                // from the cycle each Tick starts in.
                let first_of_two = p.phi.main.floor() % 2.0 == 0.0;
                shape = shape * 0.75 - if first_of_two { 1.0 / 8.0 } else { 1.0 / 24.0 };
            }
            SAW_LEVEL * shape
        }
        INST_SQUARE => high_for_the_last(if buzz { 13.0 / 32.0 } else { 0.5 }, t),
        INST_PULSE => high_for_the_last(if buzz { 7.0 / 32.0 } else { 0.3125 }, t),
        INST_ORGAN => {
            let mut shape = if t < 0.5 {
                3.0 - (24.0 * t - 6.0).abs()
            } else {
                1.0 - (16.0 * t - 12.0).abs()
            };
            if buzz {
                shape = shape * 2.0 / 3.0 + if t < 0.5 { -0.375 } else { 0.375 };
            }
            shape / 9.0
        }
        INST_NOISE if noiz => {
            // Measured on PICO-8: NOIZ holds each random level for 64 samples less the pitch.
            p.held_samples += 1;
            if p.held_samples >= (NOIZ_HOLD_PITCH - p.key).max(1) {
                p.held_samples = 0;
                p.last_sample = rng.next();
            }
            NOIZ_LEVEL * p.last_sample
        }
        INST_NOISE => {
            let scale = (advance - p.last_advance) as f32 * 8.858_923;
            let new_sample = (p.last_sample + scale * rng.next()) / (1.0 + scale);
            let factor = 1.0 - f32::from(p.key) / 63.0;
            let shape = NOISE_LEVEL * new_sample * 1.5 * (1.0 + factor * factor);
            p.last_advance = advance;
            p.last_sample = new_sample;
            shape
        }
        INST_PHASER => {
            (2.0 * triangle(t, buzz) + triangle((p.phi.second % 1.0) as f32, buzz)) / 6.0
        }
        _ => 0.0,
    }
}

/// How far DETUNE moves the second voice of a Waveform Instrument, whatever its instrument digit.
/// DETUNE 1 is measured on PICO-8, a little below the first voice; DETUNE 2 follows the zepto8
/// port.
fn waveform_detune_factor(detune: u8) -> f32 {
    if detune == 1 {
        199.0 / 200.0
    } else {
        400.0 / 199.0
    }
}

fn synth_sample(p: &mut SynthParam, rng: &mut Rng) -> f32 {
    let mut wave = waveform(p, rng);
    let detune = detune_level(p.filters);
    let plays_waveform = p.wave.is_some();
    // DETUNE on the phaser only moves its own second wave.
    let has_second_voice = plays_waveform || !matches!(p.instrument, INST_NOISE | INST_PHASER);
    if detune != 0 && has_second_voice {
        let mut second = *p;
        if plays_waveform {
            second.phi *= f64::from(waveform_detune_factor(detune));
        } else {
            let reads_twice_over = detune == 2 && p.instrument != INST_TRIANGLE;
            second.phi.main = p.phi.second * if reads_twice_over { 2.0 } else { 1.0 };
        }
        let detuned = waveform(&mut second, rng);
        // Measured on PICO-8: a Waveform Instrument mixes its detuned voice in at 0.3 and its
        // first voice at 0.7, so that DETUNE leaves its peak level as it was. A Built-in
        // Instrument mixes them at 5/6 and 5/12.
        wave = if plays_waveform {
            wave * WAVEFORM_FIRST_VOICE + detuned * (1.0 - WAVEFORM_FIRST_VOICE)
        } else {
            wave * DETUNE_FIRST_VOICE + detuned * DETUNE_SECOND_VOICE
        };
    }
    (wave * p.volume).clamp(-1.0, 1.0)
}

/// How much of the last output sample a DAMPEN level keeps. Measured on PICO-8: DAMPEN is a
/// one-pole low-pass, `out = keep * last_out + (1 - keep) * in`, that keeps 1/2 at DAMPEN 1 and
/// 3/4 at DAMPEN 2, and runs on DAMPEN 0 by keeping nothing.
fn dampen_keep(dampen: u8) -> f32 {
    match dampen {
        1 => 0.5,
        2 => 0.75,
        _ => 0.0,
    }
}

/// How far a sound has played.
#[derive(Clone, Copy, Debug)]
struct SoundState {
    playing: bool,
    offset: f64,
    time: f64,
    previous_key: i16,
    previous_volume: f32,
}

impl SoundState {
    const STOPPED: Self = Self {
        playing: false,
        offset: 0.0,
        time: 0.0,
        previous_key: MIDDLE_KEY,
        previous_volume: 0.0,
    };
}

/// One channel of sound: plays one [`Sound`] at a time.
#[derive(Clone, Debug)]
pub struct Voice {
    sound: Sound,
    state: SoundState,
    instruments: Arc<Instruments>,
    instrument_number: u8,
    instrument_state: SoundState,
    last_main_instrument: u8,
    last_main_key: i16,
    last_synth: SynthParam,
    fade_synth: SynthParam,
    fade: f32,
    reverb_1: Vec<f32>,
    reverb_2: Vec<f32>,
    reverb_index: usize,
    latched_dampen: u8,
    low_pass_output: f32,
    resting_filters: u8,
    rest_samples_left: Option<u64>,
    tick_sample: u32,
    rng: Rng,
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

impl Voice {
    /// A silent voice.
    pub fn new() -> Self {
        Self {
            sound: Sound::default(),
            state: SoundState::STOPPED,
            instruments: Arc::default(),
            instrument_number: 0,
            instrument_state: SoundState::STOPPED,
            last_main_instrument: NO_INSTRUMENT,
            last_main_key: NO_KEY,
            last_synth: SynthParam::default(),
            fade_synth: SynthParam::default(),
            fade: 0.0,
            reverb_1: vec![0.0; REVERB_1_SAMPLES],
            reverb_2: vec![0.0; REVERB_2_SAMPLES],
            reverb_index: 0,
            latched_dampen: 0,
            low_pass_output: 0.0,
            resting_filters: 0,
            rest_samples_left: None,
            tick_sample: 0,
            rng: Rng(NOISE_SEED),
        }
    }

    /// A silent voice whose wave is `phase_units` 65,536ths of the way through its cycle. A new
    /// voice starts at the start of its cycle, as every channel does when PICO-8 starts a song,
    /// and playing a sound keeps the phase the voice has.
    pub fn with_phase(phase_units: u16) -> Self {
        let mut voice = Self::new();
        voice.last_synth.phi = Phase::at(f64::from(phase_units) / f64::from(PHASE_UNITS_PER_CYCLE));
        voice
    }

    /// Sets the sound that notes marked `custom` play for an instrument slot from 0 to 7.
    pub fn set_sfx_instrument(&mut self, slot: usize, sound: Sound) {
        self.set_instrument(slot as u8, Instrument::Sfx(sound));
    }

    /// Sets what notes marked `custom` play for one instrument number.
    pub fn set_instrument(&mut self, number: u8, instrument: Instrument) {
        Arc::make_mut(&mut self.instruments).insert(number, instrument);
    }

    /// Replaces every instrument that notes marked `custom` play. A number with no instrument
    /// plays silence.
    pub fn set_instruments(&mut self, instruments: Arc<Instruments>) {
        self.instruments = instruments;
    }

    /// Starts a sound from one of its notes.
    pub fn play(&mut self, sound: Sound, from_note: usize) {
        self.resting_filters = 0;
        self.latched_dampen = dampen_level(sound.filters);
        self.tick_sample = 0;
        self.sound = sound;
        self.state = SoundState {
            playing: true,
            offset: from_note.min(NOTES - 1) as f64,
            ..SoundState::STOPPED
        };
        self.last_main_instrument = NO_INSTRUMENT;
        self.last_main_key = NO_KEY;
    }

    /// Starts a sound the way PICO-8's music starts a channel's SFX at each Pattern: as [`play`],
    /// except that a silent voice also starts its wave at the start of its cycle.
    ///
    /// [`play`]: Self::play
    pub fn play_restarting_if_silent(&mut self, sound: Sound, from_note: usize) {
        if self.last_synth.volume == 0.0 && self.fade <= 0.0 {
            self.last_synth.phi = Phase::START;
        }
        self.play(sound, from_note);
    }

    /// Moves on to one note of a sound as the next Step of what is already playing, so that a
    /// slide carries over from the note just played and nothing retriggers.
    pub fn continue_at(&mut self, sound: Sound, note: usize) {
        // A voice that has just crossed into its next note already holds the one it played.
        if self.state.offset.fract() != 0.0 {
            let played = self.sound.notes[(self.state.offset as usize).min(NOTES - 1)];
            self.state.previous_key = played.pitch;
            self.state.previous_volume = f32::from(played.volume) / 7.0;
        }
        self.sound = sound;
        self.state.playing = true;
        self.state.offset = note.min(NOTES - 1) as f64;
    }

    /// Falls silent the way a PICO-8 channel does when its SFX ends at LEN inside a Pattern: as
    /// playing a silent sound, except that the echo of the sound it ends rings on.
    pub fn rest(&mut self) {
        let filters = self.sound.filters;
        self.play(Sound::default(), 0);
        self.resting_filters = filters;
        self.rest_samples_left = None;
    }

    /// After a sound's last audible note, its echo rings on through the silent notes up to its LEN,
    /// or to its last note without one, and is then cut.
    fn ring_through_trailing_silence(&mut self) {
        let length = self
            .sound
            .len()
            .map_or(NOTES as f64, |len| f64::from(len).min(NOTES as f64));
        let samples_per_note = f64::from(SAMPLES_PER_TICK) * f64::from(self.sound.speed.max(1));
        let left = ((length - self.state.time) * samples_per_note)
            .round()
            .max(0.0);
        self.resting_filters = self.sound.filters;
        self.rest_samples_left = Some(left as u64);
    }

    /// Cuts the sound at once, along with any echo and crossfade still ringing.
    pub fn stop(&mut self) {
        let instruments = std::mem::take(&mut self.instruments);
        *self = Self::new();
        self.instruments = instruments;
    }

    /// Whether a sound is still playing.
    pub fn is_playing(&self) -> bool {
        self.state.playing
    }

    /// The next sample at 22,050 Hz.
    pub fn next_sample(&mut self) -> f32 {
        let last_synth = self.last_synth;
        let mut new_synth = SynthParam {
            phi: last_synth.phi,
            last_advance: last_synth.last_advance,
            last_sample: last_synth.last_sample,
            held_samples: last_synth.held_samples,
            ..SynthParam::default()
        };
        let mut value = 0.0;

        let base_offset = self.state.offset;
        let plays_a_note = self.state.playing;
        update_sound(&self.sound, &mut self.state, &mut new_synth, 0, 1.0);
        if plays_a_note && !self.state.playing {
            self.ring_through_trailing_silence();
        }
        // Measured on PICO-8: a silent note sets the wave back to the start of its cycle, so the
        // next note starts there.
        if plays_a_note && new_synth.volume == 0.0 {
            new_synth.phi = Phase::START;
        }

        let plays_a_custom_note = new_synth.custom;
        // PICO-8 retriggers an SFX Instrument when the pitch changes or after a silent note. Drop
        // inverts the pitch rule: the same pitch retriggers at the start of its note, and a new
        // pitch carries on.
        let drops = self.sound.notes[(base_offset as usize).min(NOTES - 1)].effect == FX_DROP;
        let starts_a_note = base_offset.fract() == 0.0;
        let same_key = new_synth.key == self.last_main_key;
        let mut restart_instrument = new_synth.instrument != self.last_main_instrument
            || self.last_main_key == NO_KEY
            || if drops {
                same_key && starts_a_note
            } else {
                !same_key
            };
        self.last_main_instrument = new_synth.instrument;
        self.last_main_key = if new_synth.volume > 0.0 {
            new_synth.key
        } else {
            NO_KEY
        };

        if new_synth.volume > 0.0 {
            if plays_a_custom_note {
                if self.state.offset < base_offset {
                    restart_instrument = true;
                }
                if !self.instrument_state.playing
                    && base_offset.floor() != self.state.offset.floor()
                {
                    restart_instrument = true;
                }
                if restart_instrument {
                    self.instrument_number = new_synth.instrument;
                    self.instrument_state = SoundState {
                        playing: true,
                        ..SoundState::STOPPED
                    };
                }
                match self.instruments.get(&self.instrument_number) {
                    Some(Instrument::Waveform(wave)) => {
                        if wave.bass {
                            new_synth.increment *= 0.5;
                        }
                        new_synth.filters = wave.filters & !NOIZ_AND_BUZZ;
                        new_synth.phi = advanced(
                            last_synth.phi,
                            new_synth.increment,
                            new_synth.instrument,
                            new_synth.filters,
                        );
                        new_synth.wave = Some(wave.samples);
                    }
                    sfx => {
                        let silent = Sound::default();
                        let sound = match sfx {
                            Some(Instrument::Sfx(sound)) => sound,
                            _ => &silent,
                        };
                        new_synth.phi = last_synth.phi;
                        // Measured on PICO-8: the played pitch adds to the instrument's own
                        // pitches before they become phase steps, and the played note's own effect
                        // scales what they give.
                        let pitch_shift = new_synth.key - MIDDLE_KEY;
                        let increment_factor = new_synth.increment / pitch_increment(new_synth.key);
                        let main_volume = new_synth.volume;
                        // A silent note of the SFX Instrument silences the voice.
                        new_synth.volume = 0.0;
                        update_sound(
                            sound,
                            &mut self.instrument_state,
                            &mut new_synth,
                            pitch_shift,
                            increment_factor,
                        );
                        new_synth.volume *= main_volume;
                    }
                }
            }
            value = synth_sample(&mut new_synth, &mut self.rng);
        }

        // Crossfade over abrupt changes, so that they do not click.
        let increment_threshold = new_synth.increment.min(last_synth.increment) * 0.01;
        if new_synth.volume != last_synth.volume
            || (new_synth.increment - last_synth.increment).abs() > increment_threshold
            || new_synth.instrument != last_synth.instrument
        {
            if self.fade <= 0.0 {
                self.fade_synth = last_synth;
            }
            self.fade = 1.0;
            new_synth.phi %= 1.0;
        }
        self.last_synth = new_synth;

        // Measured on PICO-8: the echo rings through silent notes, through those after a sound's
        // last audible note up to its LEN or last note, and on after an SFX ends at LEN inside a
        // Pattern. A sound played on its own cuts it there.
        if !self.state.playing {
            match &mut self.rest_samples_left {
                Some(0) => self.resting_filters = 0,
                Some(left) => *left -= 1,
                None => {}
            }
        }
        let echo_filters = if new_synth.volume > 0.0 {
            new_synth.filters
        } else if self.state.playing {
            self.sound.filters
        } else {
            self.resting_filters
        };
        let reverb = reverb_level(echo_filters);
        let level = |on: bool| if on { 1.0_f32 } else { 0.0 };
        let mut reverb_1 = level(reverb == 1);
        let mut reverb_2 = level(reverb == 2);

        if self.fade > 0.0 {
            // PICO-8 is already one sample into the crossfade on the first sample that changes.
            self.fade -= 1.0 / CROSSFADE_SAMPLES;
            self.fade_synth.phi = advanced(
                self.fade_synth.phi,
                self.fade_synth.increment,
                self.fade_synth.instrument,
                self.fade_synth.filters,
            );
            let faded = synth_sample(&mut self.fade_synth, &mut self.rng);
            value = mix(value, faded, self.fade);
            let fade_reverb = reverb_level(self.fade_synth.filters);
            reverb_1 = mix(reverb_1, level(fade_reverb == 1), self.fade);
            reverb_2 = mix(reverb_2, level(fade_reverb == 2), self.fade);
        }

        let index_1 = self.reverb_index % REVERB_1_SAMPLES;
        let index_2 = self.reverb_index % REVERB_2_SAMPLES;
        if reverb_1 > 0.0 {
            value += reverb_1 * self.reverb_1[index_1] * 0.5;
        }
        if reverb_2 > 0.0 {
            // fake-08's fix: zepto8 scales this echo by the REVERB 1 level.
            value += reverb_2 * self.reverb_2[index_2] * 0.5;
        }
        self.reverb_1[index_1] = value;
        self.reverb_2[index_2] = value;
        self.reverb_index = self.reverb_index.wrapping_add(1);

        // Measured on PICO-8: DAMPEN takes the level a sound starts with, and changes only on the
        // last sample of each Tick, to the level playing then, so that a Custom Instrument's own
        // DAMPEN comes in at the end of its first Tick. A Custom Instrument plays at the higher
        // of its own level and the sound's. It changes at once, with no crossfade, and a silent
        // note keeps the sound's own level. A sound that has ended keeps only the level of a
        // Custom Instrument it ended on.
        self.tick_sample += 1;
        if self.tick_sample == SAMPLES_PER_TICK {
            self.tick_sample = 0;
            self.latched_dampen = if !self.state.playing {
                if plays_a_custom_note {
                    dampen_level(new_synth.filters)
                } else {
                    0
                }
            } else if new_synth.volume > 0.0 {
                dampen_level(new_synth.filters).max(dampen_level(self.sound.filters))
            } else {
                dampen_level(self.sound.filters)
            };
        }
        self.low_pass_output = mix(
            value,
            self.low_pass_output,
            dampen_keep(self.latched_dampen),
        );
        value = self.low_pass_output;

        f32::from((32767.99 * (OUTPUT_GAIN * value).clamp(-0.99, 0.99)) as i16) / 32768.0
    }
}

/// Advances a sound by one sample and writes what it plays into `synth`, with its pitches moved up
/// by `pitch_shift` and its phase steps scaled by `increment_factor`.
fn update_sound(
    sound: &Sound,
    state: &mut SoundState,
    synth: &mut SynthParam,
    pitch_shift: i16,
    increment_factor: f32,
) {
    if !state.playing {
        return;
    }
    let inverse_rate = 1.0 / RATE;
    let speed = i32::from(sound.speed).max(1);
    let offset = state.offset;
    let offset_per_second = RATE / (f64::from(SAMPLES_PER_TICK) * f64::from(speed));
    let offset_per_sample = offset_per_second * inverse_rate;
    // A sound moves on by whole samples, so its position stays on that grid rather than drifting
    // by rounding and crossing a note a sample late.
    let samples_per_note = f64::from(SAMPLES_PER_TICK) * f64::from(speed);
    let on_sample_grid = |position: f64| (position * samples_per_note).round() / samples_per_note;
    let mut next_offset = on_sample_grid(offset + offset_per_sample);
    let next_time = on_sample_grid(state.time + offset_per_sample);

    let loop_start = f64::from(sound.loop_start);
    let loop_end = f64::from(sound.loop_end);
    let loop_range = loop_end - loop_start;
    if loop_range > 0.0 && next_offset >= loop_end {
        next_offset = (next_offset - loop_start) % loop_range + loop_start;
    }

    let mut has_end = false;
    let mut end_time = NOTES as f64;
    if let Some(len) = sound.len() {
        has_end = true;
        end_time = end_time.min(f64::from(len));
    }
    if loop_range <= 0.0 {
        has_end = true;
        let last_note = sound
            .notes
            .iter()
            .rposition(|note| note.volume > 0)
            .map_or(0, |index| index + 1);
        end_time = end_time.min(last_note as f64);
    }

    if offset < NOTES as f64 {
        let note_index = offset.floor() as usize;
        let next_note_index = next_offset.floor() as usize;
        let note = sound.notes[note_index];
        let mut volume = f32::from(note.volume) / 7.0;
        let mut increment = pitch_increment(note.pitch + pitch_shift) * increment_factor;
        // PICO-8 moves an effect on once per Tick: by the note's last Tick it has gone
        // (speed - 1) / speed of the way.
        let ticks_played = (offset * f64::from(speed) + TICK_FLOOR_TOLERANCE).floor();
        let next_ticks = next_offset * f64::from(speed);
        let ends_a_tick =
            next_ticks - (next_ticks + TICK_FLOOR_TOLERANCE).floor() < TICK_FLOOR_TOLERANCE;
        let progress = ((ticks_played % f64::from(speed)) / f64::from(speed)) as f32;

        if volume > 0.0 {
            match note.effect {
                FX_SLIDE => {
                    let from = f32::from(state.previous_key + pitch_shift);
                    let to = f32::from(note.pitch + pitch_shift);
                    increment =
                        between_pitches_increment(mix(from, to, progress)) * increment_factor;
                    if state.previous_volume > 0.0 {
                        volume = mix(state.previous_volume, volume, progress);
                    }
                }
                FX_VIBRATO => {
                    // A triangle in pitch held for two Ticks at a time: up from the note's pitch,
                    // down through it and back, over eight steps.
                    let ticks = offset * f64::from(speed);
                    let step = (ticks / VIBRATO_TICKS_PER_STEP).floor() % VIBRATO_STEPS;
                    let t = 1.0 - (((step + 2.0) % VIBRATO_STEPS) as f32 / 2.0 - 2.0).abs();
                    increment *= (t * VIBRATO_SEMITONES / 12.0).exp2();
                }
                FX_DROP if !note.custom => increment *= 1.0 - progress,
                FX_FADE_IN => volume *= progress,
                FX_FADE_OUT => volume *= 1.0 - progress,
                FX_ARP_FAST | FX_ARP_SLOW => {
                    // Each pitch of the arpeggio lasts whole Ticks: 2 up to speed 8 and 4 above,
                    // twice as long for the slow arpeggio.
                    let fast_ticks_per_pitch = if speed <= 8 { 2 } else { 4 };
                    let ticks_per_pitch = if note.effect == FX_ARP_FAST {
                        fast_ticks_per_pitch
                    } else {
                        2 * fast_ticks_per_pitch
                    };
                    let pitch_index = ticks_played as usize / ticks_per_pitch;
                    let arpeggio_note = (note_index & !3) | (pitch_index & 3);
                    increment = pitch_increment(sound.notes[arpeggio_note].pitch + pitch_shift)
                        * increment_factor;
                }
                _ => {}
            }
            synth.key = note.pitch;
            synth.increment = increment;
            synth.instrument = note.instrument;
            synth.custom = note.custom;
            synth.filters = sound.filters;
            synth.volume = volume;
            synth.phi = advanced(synth.phi, increment, note.instrument, sound.filters);
            // Measured on PICO-8: the last sample of each Tick takes its waves back to the cycle
            // they are in.
            if ends_a_tick {
                synth.last_advance -= synth.phi.main.floor();
                synth.phi %= 1.0;
            }
        }

        if next_note_index != note_index {
            state.previous_key = note.pitch;
            state.previous_volume = f32::from(note.volume) / 7.0;
        }
    }

    state.offset = next_offset;
    state.time = next_time;
    if has_end && next_time >= end_time {
        state.playing = false;
    }
}
