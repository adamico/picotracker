//! Player: turns a Project and a play request into the Playhead and samples. The Window shell
//! owns the audio stream that carries them to a device.

use crate::project::{CHAIN_ENTRIES, ChainEntry, MAX_STEPS, Phrase, Project, TRACKS};
use crate::synth::{Note, SAMPLE_RATE, SAMPLES_PER_TICK, Sound, Voice, packed_filters};
use std::sync::Arc;

/// What the App core asks the Player to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioRequest {
    /// Loops one Phrase over its LEN, starting from one of its Steps.
    PlayPhrase {
        phrase: Phrase,
        transpose: i8,
        from_step: usize,
    },
    /// Loops the Chain a Track holds on a Song row, alone on that Track, starting from one of its
    /// entries. Each entry lasts as long as its Chain row.
    PlayChain {
        project: Arc<Project>,
        song_row: usize,
        track: usize,
        from_entry: usize,
    },
    /// Plays the Song from one of its rows.
    PlaySong {
        project: Arc<Project>,
        from_row: usize,
    },
    /// Cuts the sound at once.
    Stop,
}

/// Which Chain rows play, and in what order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    /// Every Track, row by row, to the end of the Song.
    Song,
    /// One Track, round and round the Chain rows of one Song row.
    Chain { track: usize },
}

/// The place in a Project that is playing, advanced one Chain row at a time.
#[derive(Clone, Debug)]
struct Sequence {
    project: Arc<Project>,
    route: Route,
    row: usize,
    k: usize,
    samples_left: u64,
}

impl Sequence {
    /// The Chain row that plays after this one, if the Song has not ended.
    fn next_chain_row(&self) -> Option<(usize, usize)> {
        let project = &self.project;
        let later_in_row = first_chain_row(project, self.row, self.k + 1);
        match self.route {
            Route::Chain { .. } => later_in_row
                .or_else(|| first_chain_row(project, self.row, 0))
                .map(|k| (self.row, k)),
            Route::Song => later_in_row.map(|k| (self.row, k)).or_else(|| {
                let next_row = self.row + 1;
                let row = if project.song_row_is_empty(next_row) {
                    project.song_loop_row()?
                } else {
                    next_row
                };
                Some((row, first_chain_row(project, row, 0)?))
            }),
        }
    }

    /// The entry a Track plays in the current Chain row.
    fn entry(&self, track: usize) -> Option<ChainEntry> {
        let sounds = match self.route {
            Route::Song => true,
            Route::Chain { track: alone } => track == alone,
        };
        sounds
            .then(|| self.project.chain_row_entry(self.row, track, self.k))
            .flatten()
    }
}

/// The first Chain row at or after `from` on a Song row that lasts any time.
fn first_chain_row(project: &Project, row: usize, from: usize) -> Option<usize> {
    (from..CHAIN_ENTRIES).find(|&k| project.chain_row_ticks(row, k) > 0)
}

/// Renders what the App core asks for, one sample at a time.
#[derive(Clone, Debug)]
pub struct Player {
    voices: Vec<Voice>,
    sequence: Option<Sequence>,
    plays: u64,
    finished_play: u64,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            voices: vec![Voice::new(); TRACKS],
            sequence: None,
            plays: 0,
            finished_play: 0,
        }
    }
}

impl Player {
    /// A silent Player.
    pub fn new() -> Self {
        Self::default()
    }

    /// Carries out one request.
    pub fn handle(&mut self, request: AudioRequest) {
        self.cut();
        match request {
            AudioRequest::PlayPhrase {
                phrase,
                transpose,
                from_step,
            } => {
                self.plays += 1;
                let sound = sound(&phrase, transpose, 0, phrase.length() as u8);
                self.voices[0].play(sound, from_step);
            }
            AudioRequest::PlayChain {
                project,
                song_row,
                track,
                from_entry,
            } => {
                let k = first_chain_row(&project, song_row, from_entry)
                    .or_else(|| first_chain_row(&project, song_row, 0));
                self.start(project, Route::Chain { track }, song_row, k);
            }
            AudioRequest::PlaySong { project, from_row } => {
                let k = first_chain_row(&project, from_row, 0);
                self.start(project, Route::Song, from_row, k);
            }
            AudioRequest::Stop => {}
        }
    }

    /// The number of the latest play that ended by itself, counting every play request from 1.
    /// It is 0 until one has.
    pub fn finished_play(&self) -> u64 {
        self.finished_play
    }

    fn cut(&mut self) {
        self.sequence = None;
        for voice in &mut self.voices {
            voice.stop();
        }
    }

    fn start(&mut self, project: Arc<Project>, route: Route, row: usize, k: Option<usize>) {
        self.plays += 1;
        let Some(k) = k else {
            self.finished_play = self.plays;
            return;
        };
        self.enter_chain_row(Sequence {
            project,
            route,
            row,
            k,
            samples_left: 0,
        });
    }

    /// Starts every Track on a Chain row. A Track with no entry lets its sound die away.
    fn enter_chain_row(&mut self, mut sequence: Sequence) {
        for (track, voice) in self.voices.iter_mut().enumerate() {
            let sound = sequence.entry(track).map_or_else(Sound::default, |entry| {
                entry_sound(sequence.project.phrase(entry.phrase()), entry)
            });
            voice.play(sound, 0);
        }
        let ticks = sequence.project.chain_row_ticks(sequence.row, sequence.k);
        sequence.samples_left = u64::from(ticks) * u64::from(SAMPLES_PER_TICK);
        self.sequence = Some(sequence);
    }

    /// Moves to the next Chain row when the current one has run out.
    fn advance(&mut self) {
        let Some(sequence) = self.sequence.as_mut() else {
            return;
        };
        if sequence.samples_left == 0 {
            let mut sequence = self.sequence.take().expect("a sequence is playing");
            match sequence.next_chain_row() {
                Some((row, k)) => {
                    (sequence.row, sequence.k) = (row, k);
                    self.enter_chain_row(sequence);
                }
                None => {
                    self.finished_play = self.plays;
                    for voice in &mut self.voices {
                        voice.play(Sound::default(), 0);
                    }
                    return;
                }
            }
        }
        if let Some(sequence) = self.sequence.as_mut() {
            sequence.samples_left -= 1;
        }
    }

    /// The next sample at 22,050 Hz: the Tracks mixed at PICO-8's per-channel level.
    pub fn next_sample(&mut self) -> f32 {
        self.advance();
        let mix: f32 = self.voices.iter_mut().map(Voice::next_sample).sum();
        mix.clamp(-1.0, 1.0)
    }

    /// Renders a run of samples at 22,050 Hz.
    pub fn render(&mut self, samples: usize) -> Vec<f32> {
        (0..samples).map(|_| self.next_sample()).collect()
    }
}

/// The sound of a Chain entry: Fill loops the Phrase, and otherwise it plays once over its LEN.
fn entry_sound(phrase: &Phrase, entry: ChainEntry) -> Sound {
    let length = phrase.length() as u8;
    if entry.fill() {
        sound(phrase, entry.transpose(), 0, length)
    } else {
        sound(phrase, entry.transpose(), length, 0)
    }
}

/// The sound of a Phrase, with each Step at its true pitch after the transpose. A loop start with
/// a loop end of 0 is PICO-8's LEN.
fn sound(phrase: &Phrase, transpose: i8, loop_start: u8, loop_end: u8) -> Sound {
    let mut notes = [Note::default(); MAX_STEPS];
    for (index, note) in notes.iter_mut().enumerate() {
        let step = phrase.step(index);
        *note = Note {
            pitch: i16::from(step.pitch()) + i16::from(transpose),
            instrument: step.instrument(),
            custom: false,
            volume: step.volume(),
            effect: step.effect(),
        };
    }
    let filters = phrase.filters();
    Sound {
        notes,
        speed: phrase.speed(),
        filters: packed_filters(
            filters.noiz(),
            filters.buzz(),
            filters.detune(),
            filters.reverb(),
            filters.dampen(),
        ),
        loop_start,
        loop_end,
    }
}

/// Converts samples at 22,050 Hz to the device's rate by linear interpolation.
#[derive(Clone, Copy, Debug)]
pub struct Resampler {
    step: f64,
    position: f64,
    previous: f32,
    next: f32,
}

impl Resampler {
    /// A resampler for a device running at this many frames per second.
    pub fn new(device_rate: u32) -> Self {
        Self {
            step: f64::from(SAMPLE_RATE) / f64::from(device_rate),
            position: 1.0,
            previous: 0.0,
            next: 0.0,
        }
    }

    /// The next frame at the device's rate, pulling samples from the source as needed.
    pub fn next_frame(&mut self, mut source: impl FnMut() -> f32) -> f32 {
        self.position += self.step;
        while self.position >= 1.0 {
            self.position -= 1.0;
            self.previous = self.next;
            self.next = source();
        }
        self.previous + (self.next - self.previous) * self.position as f32
    }
}
