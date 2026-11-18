//! Project model: the Song, Chains, Phrases, Steps, Instruments, Export Tracks, Target Cart and
//! Claim, plus validated edits.

/// The most Steps a Phrase can hold.
pub const MAX_STEPS: usize = 32;
/// The vertical lanes of the Song.
pub const TRACKS: usize = 8;
/// The entries of every Chain.
pub const CHAIN_ENTRIES: usize = 16;
/// The rows of the Song, as many as a two-digit hexadecimal row label can name.
pub const SONG_ROWS: usize = 256;

/// The Chains and the Phrases a Project can name with two hexadecimal digits.
const ID_COUNT: usize = 256;
const MIN_TRANSPOSE: i32 = i8::MIN as i32;
const MAX_TRANSPOSE: i32 = i8::MAX as i32;

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
        self.steps[..self.length]
            .iter()
            .all(|step| step.is_silent())
    }
}

/// Names one Phrase of a Project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PhraseId(u8);

impl PhraseId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    /// The number shown for this Phrase.
    pub const fn number(self) -> u8 {
        self.0
    }
}

/// Names one Chain of a Project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChainId(u8);

impl ChainId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    /// The number shown for this Chain.
    pub const fn number(self) -> u8 {
        self.0
    }
}

/// One filled entry of a Chain: a Phrase, its transpose and its Fill mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainEntry {
    phrase: PhraseId,
    transpose: i8,
    fill: bool,
}

impl ChainEntry {
    /// An entry that plays a Phrase as written.
    pub const fn new(phrase: PhraseId) -> Self {
        Self {
            phrase,
            transpose: 0,
            fill: false,
        }
    }

    pub const fn phrase(self) -> PhraseId {
        self.phrase
    }

    /// The semitones added to every Step of the Phrase.
    pub const fn transpose(self) -> i8 {
        self.transpose
    }

    /// Whether the Phrase repeats until the Chain row ends.
    pub const fn fill(self) -> bool {
        self.fill
    }

    pub fn set_phrase(&mut self, phrase: PhraseId) {
        self.phrase = phrase;
    }

    /// Sets the transpose, stopping at -128 and 127.
    pub fn set_transpose(&mut self, semitones: i32) {
        self.transpose = semitones.clamp(MIN_TRANSPOSE, MAX_TRANSPOSE) as i8;
    }

    pub fn set_fill(&mut self, fill: bool) {
        self.fill = fill;
    }
}

/// An ordered list of entries, each holding a Phrase or empty.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chain {
    entries: [Option<ChainEntry>; CHAIN_ENTRIES],
}

impl Chain {
    /// The entry at an index from 0 to 15.
    ///
    /// # Panics
    ///
    /// Panics if the index is 16 or more.
    pub fn entry(&self, index: usize) -> Option<ChainEntry> {
        self.entries[index]
    }

    /// The entry at an index from 0 to 15, for editing.
    ///
    /// # Panics
    ///
    /// Panics if the index is 16 or more.
    pub fn entry_mut(&mut self, index: usize) -> &mut Option<ChainEntry> {
        &mut self.entries[index]
    }
}

/// The tracker's saved work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    phrases: Vec<Phrase>,
    chains: Vec<Chain>,
    song: Vec<[Option<ChainId>; TRACKS]>,
    song_loop_row: Option<usize>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            phrases: vec![Phrase::default(); ID_COUNT],
            chains: vec![Chain::default(); ID_COUNT],
            song: vec![[None; TRACKS]; SONG_ROWS],
            song_loop_row: Some(0),
        }
    }
}

impl Project {
    /// A new Project with an empty Song.
    pub fn new() -> Self {
        Self::default()
    }

    /// The Phrase with this id.
    pub fn phrase(&self, id: PhraseId) -> &Phrase {
        &self.phrases[usize::from(id.0)]
    }

    /// The Phrase with this id, for editing.
    pub fn phrase_mut(&mut self, id: PhraseId) -> &mut Phrase {
        &mut self.phrases[usize::from(id.0)]
    }

    /// The Chain with this id.
    pub fn chain(&self, id: ChainId) -> &Chain {
        &self.chains[usize::from(id.0)]
    }

    /// The Chain with this id, for editing.
    pub fn chain_mut(&mut self, id: ChainId) -> &mut Chain {
        &mut self.chains[usize::from(id.0)]
    }

    /// The Chain a Track holds on a Song row. Rows past the end of the Song hold none.
    pub fn song_cell(&self, row: usize, track: usize) -> Option<ChainId> {
        self.song.get(row)?[track]
    }

    /// Places a Chain on a Song row, or takes it away.
    ///
    /// # Panics
    ///
    /// Panics if the row is 256 or more, or the Track is 8 or more.
    pub fn set_song_cell(&mut self, row: usize, track: usize, chain: Option<ChainId>) {
        self.song[row][track] = chain;
    }

    /// The row the Song jumps to when it ends, if it loops at all.
    pub const fn song_loop_row(&self) -> Option<usize> {
        self.song_loop_row
    }

    pub fn set_song_loop_row(&mut self, row: Option<usize>) {
        self.song_loop_row = row;
    }
}
