//! Synth: turns sound data into samples at 22,050 Hz.
//!
//! This module is a Rust port of the PICO-8 synth in zepto8 at commit `b2929a4` (`src/synth.cpp`,
//! `src/filter.cpp` and `src/pico8/sfx.cpp`), with the REVERB 2 fix from fake-08 at commit
//! `814991a` (`source/Audio.cpp`, lines 764 and 765). The notices of both projects follow.
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

pub const SAMPLE_RATE: u32 = 22_050;
pub const SAMPLES_PER_TICK: u32 = 183;
pub const NOTES: usize = 32;

const RATE: f64 = SAMPLE_RATE as f64;
const SFX_INSTRUMENTS: usize = 8;
const REVERB_1_SAMPLES: usize = 2 * SAMPLES_PER_TICK as usize;
const REVERB_2_SAMPLES: usize = 4 * SAMPLES_PER_TICK as usize;
const NOISE_SEED: u32 = 0x1234_5678;
const MIDDLE_KEY: i16 = 24;
const NO_INSTRUMENT: u8 = 0xff;
const NO_KEY: i16 = i16::MAX;

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

/// Packs the filters the way byte 64 of a PICO-8 SFX holds them.
pub const fn packed_filters(noiz: bool, buzz: bool, detune: u8, reverb: u8, dampen: u8) -> u8 {
    (noiz as u8) * 2 + (buzz as u8) * 4 + detune * 8 + reverb * 24 + dampen * 72
}

fn key_to_freq(key: f32) -> f32 {
    440.0 * ((key - 33.0) / 12.0).exp2()
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
    freq: f32,
    volume: f32,
    phi: f32,
    last_advance: f32,
    last_sample: f32,
}

fn waveform(p: &mut SynthParam, rng: &mut Rng) -> f32 {
    let advance = p.phi;
    let t = advance % 1.0;
    let noiz = p.filters & 0x2 != 0;
    let buzz = p.filters & 0x4 != 0;
    match p.instrument {
        INST_TRIANGLE => {
            let mut ret = 1.0 - (4.0 * t - 2.0).abs();
            if buzz {
                let a = 0.875;
                let tilted = if t < a {
                    2.0 * t / a - 1.0
                } else {
                    2.0 * (1.0 - t) / (1.0 - a) - 1.0
                };
                ret = ret * 0.75 + tilted * 0.25;
            }
            ret * 0.5
        }
        INST_TILTED_SAW => {
            let a = if buzz { 0.975 } else { 0.875 };
            let ret = if t < a {
                2.0 * t / a - 1.0
            } else {
                2.0 * (1.0 - t) / (1.0 - a) - 1.0
            };
            ret * 0.5
        }
        INST_SAW => {
            let mut ret = if t < 0.5 { t } else { t - 1.0 };
            if buzz {
                let offset = if ((advance % 2.0) - 1.0).abs() < 0.5 {
                    0.085
                } else {
                    0.0
                };
                ret = ret * 0.83 - offset;
            }
            0.653 * ret
        }
        INST_SQUARE => {
            if t < (if buzz { 0.4 } else { 0.5 }) {
                0.25
            } else {
                -0.25
            }
        }
        INST_PULSE => {
            if t < (if buzz { 0.255 } else { 0.316 }) {
                0.25
            } else {
                -0.25
            }
        }
        INST_ORGAN => {
            let mut ret = if t < 0.5 {
                3.0 - (24.0 * t - 6.0).abs()
            } else {
                1.0 - (16.0 * t - 12.0).abs()
            };
            if buzz {
                ret = if t < 0.5 { ret * 2.0 + 3.0 } else { ret };
                ret = if t < 0.5 && ret > -1.875 {
                    ret * 0.2 - 1.0
                } else {
                    ret + 0.5
                };
            }
            ret / 9.0
        }
        INST_NOISE => {
            let scale = (advance - p.last_advance) * 8.858_923;
            let new_sample = (p.last_sample + scale * rng.next()) / (1.0 + scale);
            let factor = 1.0 - f32::from(p.key) / 63.0;
            let mut ret = new_sample * 1.5 * (1.0 + factor * factor);
            if noiz {
                ret *= 2.0 * (if t < 0.5 { t } else { t - 1.0 });
            }
            p.last_advance = advance;
            p.last_sample = new_sample;
            ret
        }
        INST_PHASER => {
            let mut ret = 2.0 - (8.0 * t - 4.0).abs();
            ret += 1.0 - (4.0 * ((advance * 109.0 / 110.0) % 1.0) - 2.0).abs();
            if buzz {
                ret += 0.25 - (((advance * 2.0 + 0.5) % 1.0) - 0.5).abs();
                ret += 0.125 - (0.5 * ((advance * 4.0) % 1.0) - 0.25).abs();
            }
            ret / 6.0
        }
        _ => 0.0,
    }
}

fn detune_factor(instrument: u8, detune: u8) -> f32 {
    let first = detune == 1;
    match instrument {
        INST_TRIANGLE => {
            if first {
                3.0 / 4.0
            } else {
                3.0 / 2.0
            }
        }
        INST_ORGAN => {
            if first {
                200.0 / 199.0
            } else {
                800.0 / 199.0
            }
        }
        INST_PHASER => {
            if first {
                49.0 / 50.0
            } else {
                400.0 / 199.0
            }
        }
        _ => {
            if first {
                200.0 / 199.0
            } else {
                400.0 / 199.0
            }
        }
    }
}

fn synth_sample(p: &mut SynthParam, rng: &mut Rng) -> f32 {
    let mut wave = waveform(p, rng);
    let detune = (p.filters / 8) % 3;
    if detune != 0 && p.instrument != INST_NOISE {
        let mut second = *p;
        second.phi *= detune_factor(p.instrument, detune);
        if detune == 2 && p.instrument == INST_ORGAN {
            second.instrument = INST_TRIANGLE;
        }
        wave += waveform(&mut second, rng) * 0.5;
    }
    (wave * p.volume).clamp(-1.0, 1.0)
}

/// The biquad filter behind DAMPEN.
#[derive(Clone, Copy, Debug)]
struct Biquad {
    c1: f32,
    c2: f32,
    c3: f32,
    c4: f32,
    c5: f32,
    last_in: f32,
    older_in: f32,
    last_out: f32,
    older_out: f32,
}

impl Biquad {
    fn high_shelf(freq: f32, q: f32, gain: f32) -> Self {
        let w0 = std::f32::consts::TAU * freq / SAMPLE_RATE as f32;
        let cos_w0 = w0.cos();
        let a = 10f32.powf(gain / 40.0);
        let alpha = (w0.sin() / 2.0) * ((a + 1.0 / a) * (1.0 / q - 1.0) + 2.0).sqrt();
        let root = 2.0 * a.sqrt() * alpha;
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + root;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - root;
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + root);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - root);
        Self {
            c1: b0 / a0,
            c2: b1 / a0,
            c3: b2 / a0,
            c4: a1 / a0,
            c5: a2 / a0,
            last_in: 0.0,
            older_in: 0.0,
            last_out: 0.0,
            older_out: 0.0,
        }
    }

    fn run(&mut self, input: f32) -> f32 {
        let output = self.c1 * input + self.c2 * self.last_in + self.c3 * self.older_in
            - self.c4 * self.last_out
            - self.c5 * self.older_out;
        self.older_in = self.last_in;
        self.last_in = input;
        self.older_out = self.last_out;
        self.last_out = output;
        output
    }
}

/// How far a sound has played.
#[derive(Clone, Copy, Debug)]
struct SoundState {
    playing: bool,
    offset: f64,
    time: f64,
    previous_key: f32,
    previous_volume: f32,
}

impl SoundState {
    const STOPPED: Self = Self {
        playing: false,
        offset: 0.0,
        time: 0.0,
        previous_key: MIDDLE_KEY as f32,
        previous_volume: 0.0,
    };
}

/// One channel of sound: plays one [`Sound`] at a time.
#[derive(Clone, Debug)]
pub struct Voice {
    sound: Sound,
    state: SoundState,
    sfx_instruments: [Sound; SFX_INSTRUMENTS],
    instrument_slot: usize,
    instrument_state: SoundState,
    last_main_instrument: u8,
    last_main_key: i16,
    last_synth: SynthParam,
    fade_synth: SynthParam,
    fade: f32,
    reverb_1: Vec<f32>,
    reverb_2: Vec<f32>,
    reverb_index: usize,
    dampen_1: Biquad,
    dampen_2: Biquad,
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
            sfx_instruments: [Sound::default(); SFX_INSTRUMENTS],
            instrument_slot: 0,
            instrument_state: SoundState::STOPPED,
            last_main_instrument: NO_INSTRUMENT,
            last_main_key: NO_KEY,
            last_synth: SynthParam::default(),
            fade_synth: SynthParam::default(),
            fade: 0.0,
            reverb_1: vec![0.0; REVERB_1_SAMPLES],
            reverb_2: vec![0.0; REVERB_2_SAMPLES],
            reverb_index: 0,
            dampen_1: Biquad::high_shelf(2400.0, 1.0, -6.0),
            dampen_2: Biquad::high_shelf(1000.0, 1.0, -12.0),
            rng: Rng(NOISE_SEED),
        }
    }

    /// Sets the sound that notes marked `custom` play for an instrument slot from 0 to 7.
    pub fn set_sfx_instrument(&mut self, slot: usize, sound: Sound) {
        self.sfx_instruments[slot % SFX_INSTRUMENTS] = sound;
    }

    /// Starts a sound from one of its notes.
    pub fn play(&mut self, sound: Sound, from_note: usize) {
        self.sound = sound;
        self.state = SoundState {
            playing: true,
            offset: from_note.min(NOTES - 1) as f64,
            ..SoundState::STOPPED
        };
        self.last_main_instrument = NO_INSTRUMENT;
        self.last_main_key = NO_KEY;
    }

    /// Moves on to one note of a sound as the next Step of what is already playing, so that a
    /// slide carries over from the note just played and nothing retriggers.
    pub fn continue_at(&mut self, sound: Sound, note: usize) {
        let played = self.sound.notes[(self.state.offset as usize).min(NOTES - 1)];
        self.sound = sound;
        self.state.playing = true;
        self.state.offset = note.min(NOTES - 1) as f64;
        self.state.previous_key = f32::from(played.pitch);
        self.state.previous_volume = f32::from(played.volume) / 7.0;
    }

    /// Cuts the sound at once, along with any echo and crossfade still ringing.
    pub fn stop(&mut self) {
        let sfx_instruments = self.sfx_instruments;
        *self = Self::new();
        self.sfx_instruments = sfx_instruments;
    }

    /// Whether a sound is still playing.
    pub fn is_playing(&self) -> bool {
        self.state.playing
    }

    /// The next sample at 22,050 Hz.
    pub fn next_sample(&mut self) -> f32 {
        let inverse_rate = 1.0 / RATE;
        let last_synth = self.last_synth;
        let mut new_synth = SynthParam {
            phi: last_synth.phi,
            last_advance: last_synth.last_advance,
            last_sample: last_synth.last_sample,
            ..SynthParam::default()
        };
        let mut value = 0.0;

        let base_offset = self.state.offset;
        update_sound(&self.sound, &mut self.state, &mut new_synth, 1.0);

        let mut restart_instrument = new_synth.instrument != self.last_main_instrument
            || new_synth.key != self.last_main_key;
        self.last_main_instrument = new_synth.instrument;
        self.last_main_key = new_synth.key;

        if new_synth.volume > 0.0 {
            if new_synth.custom {
                if self.state.offset < base_offset {
                    restart_instrument = true;
                }
                if !self.instrument_state.playing
                    && base_offset.floor() != self.state.offset.floor()
                {
                    restart_instrument = true;
                }
                if restart_instrument {
                    self.instrument_slot = usize::from(new_synth.instrument) % SFX_INSTRUMENTS;
                    self.instrument_state = SoundState {
                        playing: true,
                        ..SoundState::STOPPED
                    };
                }
                new_synth.phi = last_synth.phi;
                let freq_factor = new_synth.freq / key_to_freq(f32::from(MIDDLE_KEY));
                let main_volume = new_synth.volume;
                update_sound(
                    &self.sfx_instruments[self.instrument_slot],
                    &mut self.instrument_state,
                    &mut new_synth,
                    freq_factor,
                );
                new_synth.volume *= main_volume;
            }
            value = synth_sample(&mut new_synth, &mut self.rng);
        }

        // Crossfade over abrupt changes, so that they do not click.
        let freq_threshold = new_synth.freq.min(last_synth.freq) * 0.01;
        if (new_synth.volume - last_synth.volume).abs() > 0.1
            || (new_synth.freq - last_synth.freq).abs() > freq_threshold
            || new_synth.instrument != last_synth.instrument
        {
            if self.fade <= 0.0 {
                self.fade_synth = last_synth;
            }
            self.fade = 1.0;
            new_synth.phi %= 1.0;
        }
        self.last_synth = new_synth;

        let reverb = (new_synth.filters / 24) % 3;
        let dampen = (new_synth.filters / 72) % 3;
        let level = |on: bool| if on { 1.0_f32 } else { 0.0 };
        let mut reverb_1 = level(reverb == 1);
        let mut reverb_2 = level(reverb == 2);
        let mut dampen_1 = level(dampen == 1);
        let mut dampen_2 = level(dampen == 2);

        if self.fade > 0.0 {
            self.fade_synth.phi += (f64::from(self.fade_synth.freq) * inverse_rate) as f32;
            let faded = synth_sample(&mut self.fade_synth, &mut self.rng);
            value = mix(value, faded, self.fade);
            let fade_reverb = (self.fade_synth.filters / 24) % 3;
            let fade_dampen = (self.fade_synth.filters / 72) % 3;
            reverb_1 = mix(reverb_1, level(fade_reverb == 1), self.fade);
            reverb_2 = mix(reverb_2, level(fade_reverb == 2), self.fade);
            dampen_1 = mix(dampen_1, level(fade_dampen == 1), self.fade);
            dampen_2 = mix(dampen_2, level(fade_dampen == 2), self.fade);
            self.fade -= (130.0 * inverse_rate) as f32;
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

        let damped = self.dampen_1.run(value);
        if dampen_1 > 0.0 {
            value = mix(value, damped, dampen_1);
        }
        let damped = self.dampen_2.run(value);
        if dampen_2 > 0.0 {
            value = mix(value, damped, dampen_2);
        }

        f32::from((32767.99 * value.clamp(-0.99, 0.99)) as i16) / 32768.0
    }
}

/// Advances a sound by one sample and writes what it plays into `synth`.
fn update_sound(sound: &Sound, state: &mut SoundState, synth: &mut SynthParam, freq_factor: f32) {
    if !state.playing {
        return;
    }
    let inverse_rate = 1.0 / RATE;
    let speed = i32::from(sound.speed).max(1);
    let offset = state.offset;
    let offset_per_second = RATE / (f64::from(SAMPLES_PER_TICK) * f64::from(speed));
    let offset_per_sample = offset_per_second * inverse_rate;
    let mut next_offset = offset + offset_per_sample;
    let next_time = state.time + offset_per_sample;

    let loop_start = f64::from(sound.loop_start);
    let loop_end = f64::from(sound.loop_end);
    let loop_range = loop_end - loop_start;
    if loop_range > 0.0 && next_offset >= loop_end {
        next_offset = (next_offset - loop_start) % loop_range + loop_start;
    }

    let mut has_end = false;
    let mut end_time = NOTES as f64;
    if sound.loop_end == 0 && sound.loop_start > 0 {
        has_end = true;
        end_time = end_time.min(loop_start);
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
        let mut freq = key_to_freq(f32::from(note.pitch)) * freq_factor;
        let progress = (offset % 1.0) as f32;

        if volume > 0.0 {
            match note.effect {
                FX_SLIDE => {
                    freq = mix(key_to_freq(state.previous_key), freq, progress);
                    if state.previous_volume > 0.0 {
                        volume = mix(state.previous_volume, volume, progress);
                    }
                }
                FX_VIBRATO => {
                    let t = (((7.5 * offset / offset_per_second) % 1.0) as f32 - 0.5).abs() - 0.25;
                    freq = mix(freq, freq * 1.059_463_1, t);
                }
                FX_DROP => freq *= 1.0 - progress,
                FX_FADE_IN => volume *= progress,
                FX_FADE_OUT => volume *= 1.0 - progress,
                FX_ARP_FAST | FX_ARP_SLOW => {
                    let rate = if speed <= 8 { 32 } else { 16 };
                    let divisor = if note.effect == FX_ARP_FAST { 4 } else { 8 };
                    let position = f64::from(rate / divisor) * 7.5 * offset / offset_per_second;
                    let arpeggio_note = (note_index & !3) | (position as usize & 3);
                    freq = key_to_freq(f32::from(sound.notes[arpeggio_note].pitch));
                }
                _ => {}
            }
            synth.key = note.pitch;
            synth.freq = freq;
            synth.instrument = note.instrument;
            synth.custom = note.custom;
            synth.filters = sound.filters;
            synth.volume = volume;
            synth.phi += (f64::from(freq) * inverse_rate) as f32;
        }

        if next_note_index != note_index {
            state.previous_key = f32::from(note.pitch);
            state.previous_volume = f32::from(note.volume) / 7.0;
        }
    }

    state.offset = next_offset;
    state.time = next_time;
    if has_end && next_time >= end_time {
        state.playing = false;
    }
}
