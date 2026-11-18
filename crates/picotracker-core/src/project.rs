//! Project model: the Song, Chains, Phrases, Steps, Instruments, Export Tracks, Target Cart and
//! Claim, plus validated edits.

/// The most Steps a Phrase can hold.
pub const MAX_STEPS: usize = 32;

const DEFAULT_LENGTH: usize = 16;
const DEFAULT_SPEED: u8 = 16;
const DEFAULT_PITCH: u8 = 24;
const MAX_PITCH: i32 = 63;
const MAX_BUILT_IN_INSTRUMENT: i32 = 7;
const MAX_VOLUME: i32 = 7;
const MAX_EFFECT: i32 = 7;
const MAX_FILTER_LEVEL: i32 = 2;
const MIN_SPEED: i32 = 1;
const MAX_SPEED: i32 = 255;
/// BPM is about this number divided by the Speed, assuming 4 Steps per beat.
const BPM_AT_SPEED_1: f32 = 1807.0;

/// One PICO-8 note. A Step at volume 0 is silent and keeps its pitch and Instrument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pitch: u8,
    instrument: u8,
    volume: u8,
    effect: u8,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            pitch: DEFAULT_PITCH,
            instrument: 0,
            volume: 0,
            effect: 0,
        }
    }
}

impl Step {
    pub const fn pitch(self) -> u8 {
        self.pitch
    }

    pub const fn instrument(self) -> u8 {
        self.instrument
    }

    pub const fn volume(self) -> u8 {
        self.volume
    }

    pub const fn effect(self) -> u8 {
        self.effect
    }

    /// Whether the Step makes no sound.
    pub const fn is_silent(self) -> bool {
        self.volume == 0
    }

    /// Sets the pitch, stopping at 0 and 63.
    pub fn set_pitch(&mut self, pitch: i32) {
        self.pitch = pitch.clamp(0, MAX_PITCH) as u8;
    }

    /// Sets the Instrument, stopping at the Built-in Instruments 00 and 07.
    pub fn set_instrument(&mut self, instrument: i32) {
        self.instrument = instrument.clamp(0, MAX_BUILT_IN_INSTRUMENT) as u8;
    }

    /// Sets the volume, stopping at 0 and 7.
    pub fn set_volume(&mut self, volume: i32) {
        self.volume = volume.clamp(0, MAX_VOLUME) as u8;
    }

    /// Sets the effect, stopping at 0 and 7.
    pub fn set_effect(&mut self, effect: i32) {
        self.effect = effect.clamp(0, MAX_EFFECT) as u8;
    }
}

/// A Phrase's own sound controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    noiz: bool,
    buzz: bool,
    detune: u8,
    reverb: u8,
    dampen: u8,
}

impl Filters {
    pub const fn noiz(self) -> bool {
        self.noiz
    }

    pub const fn buzz(self) -> bool {
        self.buzz
    }

    pub const fn detune(self) -> u8 {
        self.detune
    }

    pub const fn reverb(self) -> u8 {
        self.reverb
    }

    pub const fn dampen(self) -> u8 {
        self.dampen
    }

    pub fn set_noiz(&mut self, on: bool) {
        self.noiz = on;
    }

    pub fn set_buzz(&mut self, on: bool) {
        self.buzz = on;
    }

    /// Sets DETUNE, stopping at 0 and 2.
    pub fn set_detune(&mut self, level: i32) {
        self.detune = level.clamp(0, MAX_FILTER_LEVEL) as u8;
    }

    /// Sets REVERB, stopping at 0 and 2.
    pub fn set_reverb(&mut self, level: i32) {
        self.reverb = level.clamp(0, MAX_FILTER_LEVEL) as u8;
    }

    /// Sets DAMPEN, stopping at 0 and 2.
    pub fn set_dampen(&mut self, level: i32) {
        self.dampen = level.clamp(0, MAX_FILTER_LEVEL) as u8;
    }
}

/// A run of 1 to 32 Steps with its own Speed and filters.
///
/// All 32 Steps are always stored, so shortening LEN and lengthening it again loses nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phrase {
    steps: [Step; MAX_STEPS],
    length: usize,
    speed: u8,
    filters: Filters,
}

impl Default for Phrase {
    fn default() -> Self {
        Self {
            steps: [Step::default(); MAX_STEPS],
            length: DEFAULT_LENGTH,
            speed: DEFAULT_SPEED,
            filters: Filters::default(),
        }
    }
}

impl Phrase {
    /// The Step at an index from 0 to 31.
    ///
    /// # Panics
    ///
    /// Panics if the index is 32 or more.
    pub fn step(&self, index: usize) -> Step {
        self.steps[index]
    }

    /// The Step at an index from 0 to 31, for editing.
    ///
    /// # Panics
    ///
    /// Panics if the index is 32 or more.
    pub fn step_mut(&mut self, index: usize) -> &mut Step {
        &mut self.steps[index]
    }

    /// The number of Steps that play: LEN.
    pub const fn length(&self) -> usize {
        self.length
    }

    /// Sets LEN, stopping at 1 and 32.
    pub fn set_length(&mut self, length: i32) {
        self.length = length.clamp(1, MAX_STEPS as i32) as usize;
    }

    /// How many Ticks each Step lasts.
    pub const fn speed(&self) -> u8 {
        self.speed
    }

    /// Sets the Speed, stopping at 1 and 255.
    pub fn set_speed(&mut self, speed: i32) {
        self.speed = speed.clamp(MIN_SPEED, MAX_SPEED) as u8;
    }

    /// The approximate tempo, assuming 4 Steps per beat. It is only a readout of the Speed.
    pub fn bpm(&self) -> u16 {
        (BPM_AT_SPEED_1 / f32::from(self.speed)).round() as u16
    }

    pub const fn filters(&self) -> Filters {
        self.filters
    }

    pub fn filters_mut(&mut self) -> &mut Filters {
        &mut self.filters
    }

    /// Whether every Step inside LEN is silent.
    pub fn is_silent(&self) -> bool {
        self.steps[..self.length].iter().all(|step| step.is_silent())
    }
}

/// Names one Phrase of a Project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PhraseId(usize);

/// The Phrase a new Project starts with.
pub const FIRST_PHRASE: PhraseId = PhraseId(0);

/// The tracker's saved work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    phrases: Vec<Phrase>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            phrases: vec![Phrase::default()],
        }
    }
}

impl Project {
    /// A new Project holding one empty Phrase.
    pub fn new() -> Self {
        Self::default()
    }

    /// The Phrase with this id.
    pub fn phrase(&self, id: PhraseId) -> &Phrase {
        &self.phrases[id.0]
    }

    /// The Phrase with this id, for editing.
    pub fn phrase_mut(&mut self, id: PhraseId) -> &mut Phrase {
        &mut self.phrases[id.0]
    }
}
