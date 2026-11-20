//! Carts generated in test code. None of these were saved by PICO-8: they follow the formats the
//! spec describes, and only stand in until Carts re-saved through real PICO-8 exist.
#![allow(dead_code)]

use pikutracker_core::app::{Action, App};
use pikutracker_core::cart::Cart;
use pikutracker_core::player::{AudioRequest, Player, TrackMix};
use pikutracker_core::project::Project;
use pikutracker_core::synth::{
    Instrument, Note, SAMPLES_PER_TICK, Sound, Voice, Waveform, packed_filters,
};
use std::fmt::Write;
use std::sync::Arc;

pub const HEADER: &str = "pico-8 cartridge // http://www.pico-8.com\nversion 43\n";

pub const PATTERNS_AT: usize = 0x3100;
pub const SFX_AT: usize = 0x3200;
const CART_BYTES: usize = 0x8020;
const PNG_WIDTH: u32 = 160;
const PNG_HEIGHT: u32 = 205;

/// Applies these actions in turn.
pub fn apply(app: &mut App, actions: &[Action]) {
    for action in actions {
        app.apply(*action);
    }
}

/// The bottom row of the character grid.
pub fn status_line(app: &App) -> String {
    app.grid().text_dump().lines().last().unwrap().to_owned()
}

/// One note written the way a `__sfx__` line holds it.
#[derive(Clone, Copy, Debug)]
pub struct TestNote {
    pub pitch: u8,
    pub instrument: u8,
    pub volume: u8,
    pub effect: u8,
    pub custom: bool,
}

/// An audible note on the triangle at a moderate volume.
pub const fn note(pitch: u8) -> TestNote {
    TestNote {
        pitch,
        instrument: 0,
        volume: 3,
        effect: 0,
        custom: false,
    }
}

const REST: TestNote = TestNote {
    pitch: 0,
    instrument: 0,
    volume: 0,
    effect: 0,
    custom: false,
};

/// One SFX of a generated Cart.
#[derive(Clone, Copy, Debug)]
pub struct TestSfx {
    pub editor_and_filters: u8,
    pub speed: u8,
    pub loop_start: u8,
    pub loop_end: u8,
    pub notes: [TestNote; 32],
}

impl TestSfx {
    /// An SFX at this Speed whose first notes are these pitches, and silent after them.
    pub fn with_pitches(speed: u8, pitches: &[u8]) -> Self {
        let mut notes = [REST; 32];
        for (slot, pitch) in notes.iter_mut().zip(pitches) {
            *slot = note(*pitch);
        }
        Self {
            editor_and_filters: 0,
            speed,
            loop_start: 0,
            loop_end: 0,
            notes,
        }
    }

    /// The SFX's 168-digit `__sfx__` line.
    pub fn line(&self) -> String {
        let mut line = format!(
            "{:02x}{:02x}{:02x}{:02x}",
            self.editor_and_filters, self.speed, self.loop_start, self.loop_end
        );
        for note in &self.notes {
            let instrument = note.instrument + if note.custom { 8 } else { 0 };
            write!(
                line,
                "{:02x}{:x}{:x}{:x}",
                note.pitch, instrument, note.volume, note.effect
            )
            .unwrap();
        }
        line
    }

    /// The SFX's 68 bytes in Cart memory.
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(68);
        for note in &self.notes {
            let word = u16::from(note.pitch)
                | u16::from(note.instrument) << 6
                | u16::from(note.volume) << 9
                | u16::from(note.effect) << 12
                | u16::from(note.custom) << 15;
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend([
            self.editor_and_filters,
            self.speed,
            self.loop_start,
            self.loop_end,
        ]);
        bytes
    }
}

/// One Pattern of a generated Cart.
#[derive(Clone, Copy, Debug, Default)]
pub struct TestPattern {
    pub channels: [Option<u8>; 4],
    pub loop_start: bool,
    pub loop_back: bool,
    pub stop: bool,
}

impl TestPattern {
    pub const fn playing(channels: [Option<u8>; 4]) -> Self {
        Self {
            channels,
            loop_start: false,
            loop_back: false,
            stop: false,
        }
    }

    /// The Pattern's four bytes in Cart memory.
    pub fn bytes(&self) -> [u8; 4] {
        let flags = [self.loop_start, self.loop_back, self.stop, false];
        let mut bytes = [0; 4];
        for (channel, byte) in bytes.iter_mut().enumerate() {
            let sfx = self.channels[channel].unwrap_or(0x41 + channel as u8);
            *byte = sfx | u8::from(flags[channel]) << 7;
        }
        bytes
    }

    /// The Pattern's `__music__` line.
    pub fn line(&self) -> String {
        let flags =
            u8::from(self.loop_start) | u8::from(self.loop_back) << 1 | u8::from(self.stop) << 2;
        let [a, b, c, d] = self.bytes().map(|byte| byte & 0x7f);
        format!("{flags:02x} {a:02x}{b:02x}{c:02x}{d:02x}")
    }
}

/// A generated Cart: SFX and Patterns from slot 0, with the rest left out.
#[derive(Clone, Debug, Default)]
pub struct TestCart {
    pub sfx: Vec<TestSfx>,
    pub patterns: Vec<TestPattern>,
}

impl TestCart {
    /// The Cart as `.p8` text, with the trailing default rows left out.
    pub fn p8(&self) -> Vec<u8> {
        let mut text = format!("{HEADER}__lua__\nprint(\"hi\")\n");
        if !self.sfx.is_empty() {
            text.push_str("__sfx__\n");
            for sfx in &self.sfx {
                text.push_str(&sfx.line());
                text.push('\n');
            }
        }
        if !self.patterns.is_empty() {
            text.push_str("__music__\n");
            for pattern in &self.patterns {
                text.push_str(&pattern.line());
                text.push('\n');
            }
        }
        text.into_bytes()
    }

    /// The Cart's whole memory, with default rows where nothing was written.
    pub fn memory(&self) -> Vec<u8> {
        let mut memory = vec![0; CART_BYTES];
        let blank = TestSfx::with_pitches(16, &[]);
        for slot in 0..64 {
            let sfx = self.sfx.get(slot).unwrap_or(&blank);
            memory[SFX_AT + slot * 68..SFX_AT + (slot + 1) * 68].copy_from_slice(&sfx.bytes());
            let pattern = self.patterns.get(slot).copied().unwrap_or_default();
            memory[PATTERNS_AT + slot * 4..PATTERNS_AT + (slot + 1) * 4]
                .copy_from_slice(&pattern.bytes());
        }
        memory
    }

    /// The Cart as a `.p8.png` image.
    pub fn png(&self) -> Vec<u8> {
        png_of(&self.memory(), PNG_WIDTH, PNG_HEIGHT)
    }
}

/// What the synth plays for one SFX of a Cart.
pub fn sound_of(cart: &Cart, slot: u8) -> Sound {
    let sfx = cart.sfx(slot);
    let mut notes = [Note::default(); 32];
    for (index, note) in notes.iter_mut().enumerate() {
        let stored = sfx.note(index);
        *note = Note {
            pitch: i16::from(stored.pitch),
            instrument: stored.instrument,
            custom: stored.custom,
            volume: stored.volume,
            effect: stored.effect,
        };
    }
    let byte = sfx.filters();
    Sound {
        notes,
        speed: sfx.speed(),
        filters: packed_filters(
            byte & 2 != 0,
            byte & 4 != 0,
            (byte / 8) % 3,
            (byte / 24) % 3,
            (byte / 72) % 3,
        ),
        loop_start: 0,
        loop_end: 32,
    }
}

/// Plays these stored notes of an SFX in turn, one Step each, the way a channel moves through them.
pub fn render_notes(sound: Sound, notes: &[usize]) -> Vec<f32> {
    render_sounds(&[(sound, notes)])
}

/// Several sounds one after another on one voice, as a channel plays them Pattern by Pattern:
/// each starts afresh from its first listed note as PICO-8 starts a Pattern, while the voice
/// carries its phase, crossfade and echo over.
pub fn render_sounds(sounds: &[(Sound, &[usize])]) -> Vec<f32> {
    let mut voice = Voice::new();
    let mut samples = Vec::new();
    for (sound, notes) in sounds {
        let step_samples = usize::from(sound.speed.max(1)) * SAMPLES_PER_TICK as usize;
        for (step, note) in notes.iter().enumerate() {
            if step == 0 {
                voice.play_restarting_if_silent(*sound, *note);
            } else {
                voice.continue_at(*sound, *note);
            }
            samples.extend((0..step_samples).map(|_| voice.next_sample()));
        }
    }
    samples
}

/// The Song of a Project through the Player, from its first row.
pub fn render_song(project: Project, samples: usize) -> Vec<f32> {
    let mut player = Player::new();
    player.handle(AudioRequest::PlaySong {
        project: Arc::new(project),
        from_row: 0,
    });
    player.render(samples)
}

/// The Song of a Project through the Player, from its first row, with only its Export Tracks heard.
pub fn render_export_tracks(project: Project, samples: usize) -> Vec<f32> {
    let mut mix = TrackMix::default();
    for track in project.export_track_indices().into_iter().flatten() {
        mix.toggle_solo(track);
    }
    let mut player = Player::new();
    player.handle(AudioRequest::SetTrackMix(mix));
    player.handle(AudioRequest::PlaySong {
        project: Arc::new(project),
        from_row: 0,
    });
    player.render(samples)
}

/// Fails on the first sample that differs, so that a failure says where the sound changed.
pub fn assert_same_samples(rendered: &[f32], expected: &[f32]) {
    assert_eq!(rendered.len(), expected.len());
    assert!(expected.iter().any(|sample| sample.abs() > 0.01));
    let first_difference = expected
        .iter()
        .zip(rendered)
        .position(|(expected, rendered)| expected != rendered);
    assert_eq!(first_difference, None);
}

/// Packs bytes into the low 2 bits of each pixel's channels, over a mid-grey picture.
pub fn png_of(memory: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for index in 0..(width * height) as usize {
        let byte = memory.get(index).copied().unwrap_or(0);
        pixels.extend([
            0x80 | (byte >> 4 & 3),
            0x40 | (byte >> 2 & 3),
            0xc0 | (byte & 3),
            0xfc | (byte >> 6),
        ]);
    }
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    file
}

/// The text of a `state.ron` remembering these Project files, the most recent first, written by
/// hand as the spec describes the file: RON with an integer format version.
pub fn state_ron(recent_projects: &[&str]) -> String {
    let paths: Vec<String> = recent_projects
        .iter()
        .map(|path| format!("\"{path}\""))
        .collect();
    format!("(version: 1, recent_projects: [{}])", paths.join(", "))
}

const SFX_NOTES: usize = 32;
const EDITOR_MODE_BIT: u8 = 0x01;
const WAVEFORM_INSTRUMENT_BIT: u8 = 0x80;
const BASS_BIT: u8 = 0x01;
const INSTRUMENT_SLOTS: u8 = 8;
const SLOT_COUNT: u8 = 64;

/// One SFX of a Cart exactly as its bytes hold it, loop bytes included.
pub fn stored_sound(cart: &Cart, slot: u8) -> Sound {
    let sfx = cart.sfx(slot);
    Sound {
        notes: std::array::from_fn(|index| {
            let stored = sfx.note(index);
            Note {
                pitch: i16::from(stored.pitch),
                instrument: stored.instrument,
                custom: stored.custom,
                volume: stored.volume,
                effect: stored.effect,
            }
        }),
        speed: sfx.speed(),
        filters: sfx.filters() & !EDITOR_MODE_BIT,
        loop_start: sfx.loop_start(),
        loop_end: sfx.loop_end(),
    }
}

/// What a note marked `custom` plays from one SFX 0–7: a Waveform Instrument when bit 7 of byte 66
/// is set, with Bass in bit 0 of byte 65, or else the SFX itself.
pub fn stored_instrument(cart: &Cart, slot: u8) -> Instrument {
    let sfx = cart.sfx(slot);
    if sfx.loop_start() & WAVEFORM_INSTRUMENT_BIT == 0 {
        return Instrument::Sfx(stored_sound(cart, slot));
    }
    let bytes = cart.sfx_bytes(slot);
    Instrument::Waveform(Waveform {
        samples: std::array::from_fn(|index| bytes[index] as i8),
        bass: sfx.speed() & BASS_BIT != 0,
        filters: sfx.filters() & !EDITOR_MODE_BIT,
    })
}

const SATURATION_KNEE: f32 = 0.75;
const SATURATION_SLOPE: f32 = 1.0 / 25.0;

/// The 4 channels mixed as PICO-8 0.2.7 outputs them (ADR 0011): unchanged up to 0.75 of full
/// scale, and past it rising at a twenty-fifth of the rate.
fn pico8_mix(voices: &mut [Voice]) -> f32 {
    let mix: f32 = voices.iter_mut().map(Voice::next_sample).sum();
    let past_knee = mix.abs() - SATURATION_KNEE;
    if past_knee > 0.0 {
        mix.signum() * (SATURATION_KNEE + past_knee * SATURATION_SLOPE)
    } else {
        mix
    }
}

/// One channel of a Pattern as PICO-8's music plays it.
struct MusicChannel {
    sound: Sound,
    note: usize,
    ended: bool,
}

impl MusicChannel {
    /// What the voice plays: the SFX looping over all its notes, so that the channel, not the
    /// voice, decides when it ends. A voice that ends a sound by itself cuts its echo, as `sfx()`
    /// does, while a channel that ends inside a Pattern lets it ring on.
    fn voice_sound(&self) -> Sound {
        if self.loops() {
            self.sound
        } else {
            Sound {
                loop_start: 0,
                loop_end: SFX_NOTES as u8,
                ..self.sound
            }
        }
    }

    fn loops(&self) -> bool {
        self.sound.loop_end > self.sound.loop_start
    }

    /// The notes it plays before ending: its LEN, or all 32.
    fn length(&self) -> usize {
        match (self.sound.loop_start, self.sound.loop_end) {
            (start, 0) if start > 0 => usize::from(start),
            _ => SFX_NOTES,
        }
    }
}

/// How long PICO-8 plays a Pattern, in Ticks: as long as its left-most channel that does not loop,
/// or 32 notes at the slowest Speed when every channel loops.
fn pattern_ticks(channels: &[Option<MusicChannel>]) -> usize {
    let playing = || channels.iter().flatten();
    playing().find(|channel| !channel.loops()).map_or_else(
        || {
            playing()
                .map(|channel| SFX_NOTES * usize::from(channel.sound.speed))
                .max()
                .unwrap_or(0)
        },
        |channel| channel.length() * usize::from(channel.sound.speed),
    )
}

/// The Pattern that `music()` plays after one, if the music goes on.
fn next_pattern(cart: &Cart, slot: u8) -> Option<u8> {
    let pattern = cart.pattern(slot);
    if pattern.stop() || slot + 1 == SLOT_COUNT {
        None
    } else if pattern.loop_back() {
        let loop_start = (0..=slot)
            .rev()
            .find(|&slot| cart.pattern(slot).loop_start());
        Some(loop_start.unwrap_or(0))
    } else {
        Some(slot + 1)
    }
}

/// The Channels of a Pattern, or none if it plays nothing, which ends the music.
fn music_channels(cart: &Cart, slot: u8) -> Option<Vec<Option<MusicChannel>>> {
    let pattern = cart.pattern(slot);
    let channels: Vec<Option<MusicChannel>> = (0..4)
        .map(|channel| {
            pattern.channel(channel).map(|sfx| MusicChannel {
                sound: stored_sound(cart, sfx),
                note: 0,
                ended: false,
            })
        })
        .collect();
    channels.iter().any(Option::is_some).then_some(channels)
}

/// How many samples `music()` plays from one Pattern before it stops, for music that does.
pub fn music_samples(cart: &Cart, first_pattern: u8) -> usize {
    let mut ticks = 0;
    let mut next = Some(first_pattern);
    while let Some(slot) = next {
        let Some(channels) = music_channels(cart, slot) else {
            break;
        };
        ticks += pattern_ticks(&channels);
        next = next_pattern(cart, slot);
    }
    ticks * SAMPLES_PER_TICK as usize
}

/// Plays a Cart's Patterns from one of them the way PICO-8's `music()` does, through our synth:
/// each channel steps through its SFX's notes and loop, the left-most channel that does not loop
/// ends the Pattern, and the flags choose the next one. A Pattern that plays nothing ends the
/// music. Once the music stops, the voices fall
/// silent and keep running.
pub fn render_music(cart: &Cart, first_pattern: u8, samples: usize) -> Vec<f32> {
    let mut voices = vec![Voice::new(); 4];
    for voice in &mut voices {
        for slot in 0..INSTRUMENT_SLOTS {
            voice.set_instrument(slot, stored_instrument(cart, slot));
        }
    }
    let mut output = Vec::with_capacity(samples);
    let mut next = Some(first_pattern);
    while output.len() < samples {
        let Some(mut channels) = next.and_then(|slot| music_channels(cart, slot)) else {
            for voice in &mut voices {
                voice.play(Sound::default(), 0);
            }
            while output.len() < samples {
                output.push(pico8_mix(&mut voices));
            }
            break;
        };
        for (voice, channel) in voices.iter_mut().zip(&channels) {
            let sound = channel
                .as_ref()
                .map_or_else(Sound::default, |channel| channel.sound);
            voice.play_restarting_if_silent(sound, 0);
        }
        let ticks = pattern_ticks(&channels);
        for tick in 1..=ticks {
            for _ in 0..SAMPLES_PER_TICK {
                output.push(pico8_mix(&mut voices));
            }
            if tick == ticks {
                break;
            }
            for (voice, channel) in voices.iter_mut().zip(&mut channels) {
                let Some(channel) = channel.as_mut().filter(|channel| !channel.ended) else {
                    continue;
                };
                if tick % usize::from(channel.sound.speed) != 0 {
                    continue;
                }
                channel.note += 1;
                if channel.loops() && channel.note >= usize::from(channel.sound.loop_end) {
                    channel.note = usize::from(channel.sound.loop_start);
                }
                if !channel.loops() && channel.note >= channel.length() {
                    channel.ended = true;
                    voice.rest();
                } else if channel.note >= SFX_NOTES {
                    // A loop past the last note, such as 32 to 33, plays silence in the SFX.
                    let silence = Sound {
                        notes: [Note::default(); SFX_NOTES],
                        ..channel.sound
                    };
                    voice.continue_at(silence, 0);
                } else {
                    voice.continue_at(channel.voice_sound(), channel.note);
                }
            }
        }
        next = next.and_then(|slot| next_pattern(cart, slot));
    }
    output.truncate(samples);
    output
}
