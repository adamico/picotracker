// PROTOTYPE — throwaway. A near line-by-line Rust port of zepto8's PICO-8 synth
// (src/synth.cpp, src/filter.cpp, src/pico8/sfx.cpp @ b2929a4), which is
// WTFPL v2 (c) Sam Hocevar, with fake-08's REVERB-2 fix (MIT, (c) Jon Bell).
// Only what playback needs: no sfx() API, no hardware registers, no pause.

use crate::cart::{Cart, Sfx};

pub const RATE: f64 = 22050.0;

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

pub fn key_to_freq(key: f32) -> f32 {
    440.0 * ((key - 33.0) / 12.0).exp2()
}

fn fmod(a: f32, b: f32) -> f32 {
    a % b // C fmod semantics (sign of a), same as Rust's %
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// ---------------------------------------------------------------- rng

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        // xorshift32, mapped to [-1, 1)
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

// ---------------------------------------------------------------- synth param

#[derive(Clone, Copy, Default)]
pub struct SynthParam {
    pub instrument: u8,
    pub custom: bool,
    pub filters: u8,
    pub key: u8,
    pub freq: f32,
    pub volume: f32,
    pub phi: f32,
    pub last_advance: f32,
    pub last_sample: f32,
    pub is_music: bool,
}

fn waveform(p: &mut SynthParam, rng: &mut Rng) -> f32 {
    let advance = p.phi;
    let t = fmod(advance, 1.0);
    let noiz = p.filters & 0x2 != 0;
    let buzz = p.filters & 0x4 != 0;
    match p.instrument {
        INST_TRIANGLE => {
            let mut ret = 1.0 - (4.0 * t - 2.0).abs();
            if buzz {
                let a = 0.875;
                let bret = if t < a { 2.0 * t / a - 1.0 } else { 2.0 * (1.0 - t) / (1.0 - a) - 1.0 };
                ret = ret * 0.75 + bret * 0.25;
            }
            ret * 0.5
        }
        INST_TILTED_SAW => {
            let a = if buzz { 0.975 } else { 0.875 };
            let ret = if t < a { 2.0 * t / a - 1.0 } else { 2.0 * (1.0 - t) / (1.0 - a) - 1.0 };
            ret * 0.5
        }
        INST_SAW => {
            let mut ret = if t < 0.5 { t } else { t - 1.0 };
            if buzz {
                ret = ret * 0.83 - if (fmod(advance, 2.0) - 1.0).abs() < 0.5 { 0.085 } else { 0.0 };
            }
            0.653 * ret
        }
        INST_SQUARE => if t < (if buzz { 0.4 } else { 0.5 }) { 0.25 } else { -0.25 },
        INST_PULSE => if t < (if buzz { 0.255 } else { 0.316 }) { 0.25 } else { -0.25 },
        INST_ORGAN => {
            let mut ret = if t < 0.5 { 3.0 - (24.0 * t - 6.0).abs() } else { 1.0 - (16.0 * t - 12.0).abs() };
            if buzz {
                ret = if t < 0.5 { ret * 2.0 + 3.0 } else { ret };
                ret = if t < 0.5 && ret > -1.875 { ret * 0.2 - 1.0 } else { ret + 0.5 };
            }
            ret / 9.0
        }
        INST_NOISE => {
            let tscale = 8.858923;
            let scale = (advance - p.last_advance) * tscale;
            let new_sample = (p.last_sample + scale * rng.next()) / (1.0 + scale);
            let factor = 1.0 - p.key as f32 / 63.0;
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
            ret += 1.0 - (4.0 * fmod(advance * 109.0 / 110.0, 1.0) - 2.0).abs();
            if buzz {
                ret += 0.25 - (fmod(advance * 2.0 + 0.5, 1.0) - 0.5).abs();
                ret += 0.125 - (0.5 * fmod(advance * 4.0, 1.0) - 0.25).abs();
            }
            ret / 6.0
        }
        _ => 0.0,
    }
}

fn synth_sample(p: &mut SynthParam, rng: &mut Rng) -> f32 {
    let mut w = waveform(p, rng);
    let detune = (p.filters / 8) % 3;
    if detune != 0 && p.instrument != INST_NOISE {
        let factor = match p.instrument {
            INST_TRIANGLE => if detune == 1 { 3.0 / 4.0 } else { 3.0 / 2.0 },
            INST_ORGAN => if detune == 1 { 200.0 / 199.0 } else { 800.0 / 199.0 },
            INST_PHASER => if detune == 1 { 49.0 / 50.0 } else { 400.0 / 199.0 },
            _ => if detune == 1 { 200.0 / 199.0 } else { 400.0 / 199.0 },
        };
        let mut second = *p;
        second.phi *= factor;
        if detune == 2 && p.instrument == INST_ORGAN {
            second.instrument = INST_TRIANGLE;
        }
        w += waveform(&mut second, rng) * 0.5;
    }
    // music/sfx master volumes and music fade are all 1.0 in the spike
    (w * p.volume).clamp(-1.0, 1.0)
}

// ---------------------------------------------------------------- biquad

#[derive(Clone, Copy)]
struct Biquad { c1: f32, c2: f32, c3: f32, c4: f32, c5: f32, li: f32, lli: f32, lo: f32, llo: f32 }

impl Biquad {
    /// RBJ high shelf, as zepto8's DAMPEN.
    fn highshelf(freq: f32, q: f32, gain: f32) -> Biquad {
        let w0 = std::f32::consts::TAU * freq / 22050.0;
        let cosw0 = w0.cos();
        let a = 10f32.powf(gain / 40.0);
        let alpha = (w0.sin() / 2.0) * ((a + 1.0 / a) * (1.0 / q - 1.0) + 2.0).sqrt();
        let sqra = 2.0 * a.sqrt() * alpha;
        let a0 = (a + 1.0) - (a - 1.0) * cosw0 + sqra;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cosw0);
        let a2 = (a + 1.0) - (a - 1.0) * cosw0 - sqra;
        let b0 = a * ((a + 1.0) + (a - 1.0) * cosw0 + sqra);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cosw0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cosw0 - sqra);
        Biquad { c1: b0 / a0, c2: b1 / a0, c3: b2 / a0, c4: a1 / a0, c5: a2 / a0, li: 0.0, lli: 0.0, lo: 0.0, llo: 0.0 }
    }
    fn run(&mut self, x: f32) -> f32 {
        let y = self.c1 * x + self.c2 * self.li + self.c3 * self.lli - self.c4 * self.lo - self.c5 * self.llo;
        self.lli = self.li;
        self.li = x;
        self.llo = self.lo;
        self.lo = y;
        y
    }
}

// ---------------------------------------------------------------- state

#[derive(Clone, Copy)]
pub struct SfxState {
    pub sfx: i32, // -1 = none
    pub offset: f64,
    pub time: f64,
    prev_key: f32,
    prev_vol: f32,
}

impl Default for SfxState {
    fn default() -> Self {
        SfxState { sfx: -1, offset: 0.0, time: 0.0, prev_key: 24.0, prev_vol: 0.0 }
    }
}

pub struct Channel {
    pub main_sfx: SfxState,
    custom_sfx: SfxState,
    length: f32,
    can_loop: bool,
    is_music: bool,
    last_main_instrument: u8,
    last_main_key: u8,
    pub last_synth: SynthParam,
    fade_synth: SynthParam,
    fade: f32,
    reverb_2: Box<[f32; 366]>,
    reverb_4: Box<[f32; 732]>,
    reverb_index: usize,
    damp1: Biquad,
    damp2: Biquad,
}

impl Channel {
    fn new() -> Channel {
        Channel {
            main_sfx: SfxState::default(),
            custom_sfx: SfxState::default(),
            length: 0.0,
            can_loop: true,
            is_music: false,
            last_main_instrument: 0xff,
            last_main_key: 0xff,
            last_synth: SynthParam::default(),
            fade_synth: SynthParam::default(),
            fade: 0.0,
            reverb_2: Box::new([0.0; 366]),
            reverb_4: Box::new([0.0; 732]),
            reverb_index: 0,
            damp1: Biquad::highshelf(2400.0, 1.0, -6.0),
            damp2: Biquad::highshelf(1000.0, 1.0, -12.0),
        }
    }
}

pub struct Music {
    pub pattern: i32,
    pub count: i32,
    pub offset: f64,
    pub length: f64,
}

pub struct Mixer {
    pub cart: Cart,
    pub channels: [Channel; 4],
    pub music: Music,
    rng: Rng,
    /// Per-channel mute (spike-only listening aid; not a PICO-8 feature).
    pub mute: [bool; 4],
}

impl Mixer {
    pub fn new(cart: Cart) -> Mixer {
        Mixer {
            cart,
            channels: [Channel::new(), Channel::new(), Channel::new(), Channel::new()],
            music: Music { pattern: -1, count: -1, offset: -1.0, length: 0.0 },
            rng: Rng(0x1234_5678),
            mute: [false; 4],
        }
    }

    pub fn is_silent(&self) -> bool {
        self.music.pattern == -1 && self.channels.iter().all(|c| c.main_sfx.sfx == -1)
    }

    pub fn launch_sfx(&mut self, sfx: i32, chan: usize, offset: f32, length: f32, is_music: bool) {
        let c = &mut self.channels[chan];
        c.main_sfx.sfx = sfx;
        c.main_sfx.offset = offset.max(0.0) as f64;
        c.main_sfx.time = 0.0;
        c.length = length.max(0.0);
        c.can_loop = true;
        c.is_music = is_music;
        c.last_main_instrument = 0xff;
        c.last_main_key = 0xff;
        c.main_sfx.prev_key = 24.0;
        c.main_sfx.prev_vol = 0.0;
    }

    pub fn play_music(&mut self, pattern: i32) {
        self.music.count = 0;
        self.set_music_pattern(pattern);
    }

    fn set_music_pattern(&mut self, pattern: i32) {
        for c in self.channels.iter_mut() {
            if c.is_music {
                c.main_sfx.sfx = -1;
            }
        }
        let stop = |m: &mut Music| {
            m.pattern = -1;
            m.count = -1;
            m.offset = -1.0;
            m.length = 0.0;
        };
        if !(0..=63).contains(&pattern) {
            stop(&mut self.music);
            return;
        }
        let pat = self.cart.song[pattern as usize];
        let mut duration_looping: i32 = -1;
        let mut duration_no_loop: i32 = -1;
        for i in 0..4 {
            let n = pat.sfx(i);
            if n & 0x40 != 0 {
                continue;
            }
            let s = &self.cart.sfx[(n & 0x3f) as usize];
            let has_loop = s.loop_end > 0 && s.loop_end > s.loop_start;
            if has_loop {
                duration_looping = duration_looping.max(32 * s.speed as i32);
            } else {
                let mut end_time = 32;
                if s.loop_end == 0 && s.loop_start > 0 {
                    end_time = end_time.min(s.loop_start as i32);
                }
                duration_no_loop = end_time * s.speed as i32;
                break;
            }
        }
        let duration = if duration_no_loop > 0 { duration_no_loop } else { duration_looping };
        if duration <= 0 {
            stop(&mut self.music);
            return;
        }
        self.music.pattern = pattern;
        self.music.offset = 0.0;
        self.music.length = duration as f64;
        for i in 0..4 {
            let n = pat.sfx(i);
            if n & 0x40 != 0 {
                continue;
            }
            self.launch_sfx(n as i32, i, 0.0, 0.0, true);
        }
    }

    /// One 22050 Hz output sample, in [-1, 1].
    pub fn sample(&mut self) -> f32 {
        let inv = 1.0 / RATE;
        let mut channel_mix = 0.0f32;
        for chan in 0..4 {
            if chan == 0 && self.music.pattern != -1 {
                self.music.offset += (22050.0 / 183.0) * inv;
                if self.music.offset >= self.music.length {
                    let cur = self.cart.song[self.music.pattern as usize];
                    let mut next = self.music.pattern + 1;
                    let mut count = self.music.count + 1;
                    if cur.stop() {
                        next = -1;
                        count = -1;
                    } else if cur.loop_back() {
                        loop {
                            next -= 1;
                            if !(next > 0 && !self.cart.song[next as usize].start()) {
                                break;
                            }
                        }
                    }
                    self.music.count = count;
                    self.set_music_pattern(next);
                }
            }

            let rng = &mut self.rng;
            let cart = &self.cart;
            let ch = &mut self.channels[chan];
            let last_synth = ch.last_synth;
            let mut new_synth = SynthParam {
                phi: last_synth.phi,
                last_advance: last_synth.last_advance,
                last_sample: last_synth.last_sample,
                ..Default::default()
            };
            let mut value = 0.0f32;

            let base_offset = ch.main_sfx.offset;
            update_sfx_state(cart, &mut ch.main_sfx, &mut new_synth, 1.0, ch.length, ch.is_music, ch.can_loop, inv);

            let mut restart_custom = new_synth.instrument != ch.last_main_instrument || new_synth.key != ch.last_main_key;
            ch.last_main_instrument = new_synth.instrument;
            ch.last_main_key = new_synth.key;

            if new_synth.volume > 0.0 {
                if new_synth.custom {
                    if ch.main_sfx.offset < base_offset {
                        restart_custom = true;
                    }
                    if ch.custom_sfx.sfx == -1 && base_offset.floor() != ch.main_sfx.offset.floor() {
                        restart_custom = true;
                    }
                    if restart_custom {
                        ch.custom_sfx.sfx = new_synth.instrument as i32;
                        ch.custom_sfx.offset = 0.0;
                        ch.custom_sfx.time = 0.0;
                    }
                    new_synth.phi = last_synth.phi;
                    let freq_factor = new_synth.freq / key_to_freq(24.0);
                    let main_vol = new_synth.volume;
                    update_sfx_state(cart, &mut ch.custom_sfx, &mut new_synth, freq_factor, 0.0, false, true, inv);
                    new_synth.volume *= main_vol;
                }
                value = synth_sample(&mut new_synth, rng);
            }

            // anti-click crossfade on abrupt changes
            let freq_threshold = new_synth.freq.min(last_synth.freq) * 0.01;
            if (new_synth.volume - last_synth.volume).abs() > 0.1
                || (new_synth.freq - last_synth.freq).abs() > freq_threshold
                || new_synth.instrument != last_synth.instrument
            {
                if ch.fade <= 0.0 {
                    ch.fade_synth = last_synth;
                }
                ch.fade = 1.0;
                new_synth.phi = fmod(new_synth.phi, 1.0);
            }
            ch.last_synth = new_synth;

            let reverb = (new_synth.filters / 24) % 3;
            let dampen = (new_synth.filters / 72) % 3;
            let f = |b: bool| if b { 1.0f32 } else { 0.0 };
            let (mut r1, mut r2, mut d1, mut d2) = (f(reverb == 1), f(reverb == 2), f(dampen == 1), f(dampen == 2));

            if ch.fade > 0.0 {
                ch.fade_synth.phi += (ch.fade_synth.freq as f64 * inv) as f32;
                let vf = synth_sample(&mut ch.fade_synth, rng);
                value = mix(value, vf, ch.fade);
                let fr = (ch.fade_synth.filters / 24) % 3;
                let fd = (ch.fade_synth.filters / 72) % 3;
                r1 = mix(r1, f(fr == 1), ch.fade);
                r2 = mix(r2, f(fr == 2), ch.fade);
                d1 = mix(d1, f(fd == 1), ch.fade);
                d2 = mix(d2, f(fd == 2), ch.fade);
                ch.fade -= (130.0 * inv) as f32;
            }

            if r1 > 0.0 {
                value += r1 * ch.reverb_2[ch.reverb_index % 366] * 0.5;
            }
            if r2 > 0.0 {
                // fake-08 fix: zepto8 multiplies by r1 here, silencing REVERB 2
                value += r2 * ch.reverb_4[ch.reverb_index % 732] * 0.5;
            }
            ch.reverb_2[ch.reverb_index % 366] = value;
            ch.reverb_4[ch.reverb_index % 732] = value;
            ch.reverb_index += 1;

            let v1 = ch.damp1.run(value);
            if d1 > 0.0 {
                value = mix(value, v1, d1);
            }
            let v2 = ch.damp2.run(value);
            if d2 > 0.0 {
                value = mix(value, v2, d2);
            }

            let s = (32767.99 * value.clamp(-0.99, 0.99)) as i16;
            if !self.mute[chan] {
                channel_mix += s as f32;
            }
        }
        channel_mix.clamp(-32767.9, 32767.9) / 32768.0
    }
}

#[allow(clippy::too_many_arguments)]
fn update_sfx_state(
    cart: &Cart,
    cur: &mut SfxState,
    ns: &mut SynthParam,
    freq_factor: f32,
    length: f32,
    is_music: bool,
    can_loop: bool,
    inv: f64,
) {
    if cur.sfx == -1 {
        return;
    }
    let sfx: &Sfx = &cart.sfx[cur.sfx as usize];
    let speed = (sfx.speed as i32).max(1);
    let offset = cur.offset;
    let time = cur.time;
    let offset_per_second = 22050.0 / (183.0 * speed as f64);
    let offset_per_frame = offset_per_second * inv;
    let mut next_offset = offset + offset_per_frame;
    let next_time = time + offset_per_frame;

    let loop_range = sfx.loop_end as f32 - sfx.loop_start as f32;
    if loop_range > 0.0 && next_offset >= sfx.loop_end as f64 && can_loop {
        next_offset = (next_offset - sfx.loop_start as f64) % loop_range as f64 + sfx.loop_start as f64;
    }

    let mut has_end = false;
    let mut end_time = 32.0f32;
    if length > 0.0 {
        has_end = true;
        end_time = length;
    }
    // PICO-8 quirk (per zepto8): LEN is not applied to music SFX playback
    if !is_music && sfx.loop_end == 0 && sfx.loop_start > 0 {
        has_end = true;
        end_time = end_time.min(sfx.loop_start as f32);
    }
    if loop_range <= 0.0 {
        has_end = true;
        if !is_music {
            let mut last_note = 0;
            for n in 0..32 {
                if sfx.notes[n].volume > 0 {
                    last_note = (n + 1).min(32);
                }
            }
            end_time = end_time.min(last_note as f32);
        }
    }

    if offset < 32.0 {
        let note_id = offset.floor() as usize;
        let next_note_id = next_offset.floor() as usize;
        let note = sfx.notes[note_id];
        let key = note.key;
        let mut volume = note.volume as f32 / 7.0;
        let mut freq = key_to_freq(key as f32) * freq_factor;
        let fo = (offset % 1.0) as f32;

        if volume > 0.0 {
            match note.effect {
                FX_SLIDE => {
                    freq = mix(key_to_freq(cur.prev_key), freq, fo);
                    if cur.prev_vol > 0.0 {
                        volume = mix(cur.prev_vol, volume, fo);
                    }
                }
                FX_VIBRATO => {
                    let t = (((7.5 * offset / offset_per_second) % 1.0) as f32 - 0.5).abs() - 0.25;
                    freq = mix(freq, freq * 1.059463094359, t);
                }
                FX_DROP => freq *= 1.0 - fo,
                FX_FADE_IN => volume *= fo,
                FX_FADE_OUT => volume *= 1.0 - fo,
                FX_ARP_FAST | FX_ARP_SLOW => {
                    let m = (if speed <= 8 { 32 } else { 16 }) / (if note.effect == FX_ARP_FAST { 4 } else { 8 });
                    let n = (m as f64 * 7.5 * offset / offset_per_second) as usize;
                    let arp = (note_id & !3) | (n & 3);
                    // NB: zepto8 drops freq_factor here, so arpeggios inside SFX instruments ignore the played pitch
                    freq = key_to_freq(sfx.notes[arp].key as f32);
                }
                _ => {}
            }
            ns.key = key;
            ns.freq = freq;
            ns.instrument = note.instrument;
            ns.custom = note.custom;
            ns.filters = sfx.filters;
            ns.volume = volume;
            ns.is_music = is_music;
            ns.phi += (freq as f64 * inv) as f32;
        }

        if next_note_id != note_id {
            cur.prev_key = key as f32;
            cur.prev_vol = note.volume as f32 / 7.0;
        }
    }

    cur.offset = next_offset;
    cur.time = next_time;
    if has_end && next_time >= end_time as f64 {
        cur.sfx = -1;
    }
}
