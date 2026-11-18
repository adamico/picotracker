//! Player: turns a Project and a play request into the Playhead and samples. The Window shell
//! owns the audio stream that carries them to a device.

use crate::project::{CHAIN_ENTRIES, ChainEntry, MAX_STEPS, Phrase, PhraseId, Project, TRACKS};
use crate::synth::{Note, SAMPLE_RATE, SAMPLES_PER_TICK, Sound, Voice, packed_filters};
use std::sync::Arc;

/// What the App core asks the Player to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioRequest {
    /// Loops one Phrase over its LEN, starting from one of its Steps.
    PlayPhrase {
        project: Arc<Project>,
        phrase: PhraseId,
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
    /// Swaps in the edited Project. What is playing carries on and hears each change the next time
    /// the Playhead reaches it.
    UpdateProject(Arc<Project>),
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
    /// One Phrase alone, round and round its LEN.
    Phrase { phrase: PhraseId, transpose: i8 },
}

/// How far a Track has got through its entry of the current Chain row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LaneState {
    /// No entry, or one that was cleared while it played.
    #[default]
    Idle,
    Playing,
    /// Played to its LEN, and silent until the next Chain row.
    Finished,
}

/// One Track's place in the current Chain row. Every Track keeps time, heard or not.
#[derive(Clone, Copy, Debug, Default)]
struct Lane {
    state: LaneState,
    fill: bool,
    /// Whether a Fill entry has played its Phrase through at least once.
    played_through: bool,
    step: usize,
    samples_left: u64,
}

impl Lane {
    fn starting(entry: Option<ChainEntry>, project: &Project, from_step: usize) -> Self {
        let Some(entry) = entry else {
            return Self::default();
        };
        let phrase = project.phrase(entry.phrase());
        Self {
            state: LaneState::Playing,
            fill: entry.fill(),
            played_through: false,
            step: from_step.min(phrase.length() - 1),
            samples_left: step_samples(phrase),
        }
    }

    /// Moves on from a finished Step, reading the entry as it stands now.
    fn finish_step(&mut self, entry: Option<ChainEntry>, project: &Project) {
        let Some(entry) = entry else {
            self.state = LaneState::Idle;
            return;
        };
        let phrase = project.phrase(entry.phrase());
        self.fill = entry.fill();
        if self.step + 1 < phrase.length() {
            self.step += 1;
        } else if entry.fill() {
            self.step = 0;
            self.played_through = true;
        } else {
            self.state = LaneState::Finished;
            return;
        }
        self.samples_left = step_samples(phrase);
    }
}

/// How long one Step of a Phrase lasts.
fn step_samples(phrase: &Phrase) -> u64 {
    u64::from(phrase.speed()) * u64::from(SAMPLES_PER_TICK)
}

/// The place in a Project that is playing, advanced one Step at a time.
#[derive(Clone, Debug)]
struct Sequence {
    project: Arc<Project>,
    route: Route,
    row: usize,
    k: usize,
    lanes: [Lane; TRACKS],
    /// A looping Chain with nothing left to play waits here for an edit to give it notes again.
    waiting: bool,
}

impl Sequence {
    /// The Chain row that plays after this one, if the Song has not ended. A Song row emptied
    /// while it played acts like reaching an empty row.
    fn next_chain_row(&self) -> Option<(usize, usize)> {
        let project = &self.project;
        let later_in_row = first_chain_row(project, self.row, self.k + 1);
        match self.route {
            Route::Phrase { .. } => None,
            Route::Chain { .. } => later_in_row
                .or_else(|| first_chain_row(project, self.row, 0))
                .map(|k| (self.row, k)),
            Route::Song => {
                let emptied = project.song_row_is_empty(self.row);
                if let Some(k) = later_in_row.filter(|_| !emptied) {
                    return Some((self.row, k));
                }
                let next_row = self.row + 1;
                let row = if emptied || project.song_row_is_empty(next_row) {
                    project.song_loop_row()?
                } else {
                    next_row
                };
                Some((row, first_chain_row(project, row, 0)?))
            }
        }
    }

    /// The entry that keeps time on a Track, as the Project stands now.
    fn entry(&self, track: usize) -> Option<ChainEntry> {
        match self.route {
            Route::Phrase { phrase, transpose } => (track == 0).then(|| {
                let mut entry = ChainEntry::new(phrase);
                entry.set_transpose(i32::from(transpose));
                entry.set_fill(true);
                entry
            }),
            _ => self.project.chain_row_entry(self.row, track, self.k),
        }
    }

    /// Whether a Track is heard on this route.
    fn sounds(&self, track: usize) -> bool {
        match self.route {
            Route::Song => true,
            Route::Chain { track: alone } => track == alone,
            Route::Phrase { .. } => track == 0,
        }
    }

    /// The sound of a Track's entry, if the Track is heard.
    fn sound(&self, track: usize) -> Option<Sound> {
        let entry = self.entry(track).filter(|_| self.sounds(track))?;
        Some(entry_sound(self.project.phrase(entry.phrase()), entry))
    }

    /// Whether every entry that sets the Chain row's length has played out: the entries not marked
    /// Fill, or the longest entry when all are marked Fill.
    fn chain_row_is_over(&self) -> bool {
        if matches!(self.route, Route::Phrase { .. }) {
            return false;
        }
        let mut plain = self
            .lanes
            .iter()
            .filter(|lane| lane.state != LaneState::Idle && !lane.fill)
            .peekable();
        if plain.peek().is_some() {
            return plain.all(|lane| lane.state == LaneState::Finished);
        }
        self.lanes
            .iter()
            .filter(|lane| lane.state == LaneState::Playing)
            .all(|lane| lane.played_through)
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
        match request {
            AudioRequest::PlayPhrase {
                project,
                phrase,
                transpose,
                from_step,
            } => {
                let route = Route::Phrase { phrase, transpose };
                self.start(project, route, 0, Some(0), from_step);
            }
            AudioRequest::PlayChain {
                project,
                song_row,
                track,
                from_entry,
            } => {
                let k = first_chain_row(&project, song_row, from_entry)
                    .or_else(|| first_chain_row(&project, song_row, 0));
                self.start(project, Route::Chain { track }, song_row, k, 0);
            }
            AudioRequest::PlaySong { project, from_row } => {
                let k = first_chain_row(&project, from_row, 0);
                self.start(project, Route::Song, from_row, k, 0);
            }
            AudioRequest::UpdateProject(project) => self.update_project(project),
            AudioRequest::Stop => self.cut(),
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

    fn update_project(&mut self, project: Arc<Project>) {
        let Some(sequence) = self.sequence.as_mut() else {
            return;
        };
        sequence.project = project;
        if sequence.waiting {
            self.leave_chain_row();
        }
    }

    fn start(
        &mut self,
        project: Arc<Project>,
        route: Route,
        row: usize,
        k: Option<usize>,
        from_step: usize,
    ) {
        self.cut();
        self.plays += 1;
        let Some(k) = k else {
            self.finished_play = self.plays;
            return;
        };
        let sequence = Sequence {
            project,
            route,
            row,
            k,
            lanes: [Lane::default(); TRACKS],
            waiting: false,
        };
        self.enter_chain_row(sequence, from_step);
    }

    /// Starts every Track on a Chain row. A Track with no entry lets its sound die away.
    fn enter_chain_row(&mut self, mut sequence: Sequence, from_step: usize) {
        for (track, voice) in self.voices.iter_mut().enumerate() {
            let lane = Lane::starting(sequence.entry(track), &sequence.project, from_step);
            sequence.lanes[track] = lane;
            voice.play(sequence.sound(track).unwrap_or_default(), lane.step);
        }
        self.sequence = Some(sequence);
    }

    /// Goes on to the next Chain row. With none to go to, a looping Chain waits for an edit and
    /// anything else ends the play.
    fn leave_chain_row(&mut self) {
        let Some(mut sequence) = self.sequence.take() else {
            return;
        };
        if let Some((row, k)) = sequence.next_chain_row() {
            (sequence.row, sequence.k) = (row, k);
            sequence.waiting = false;
            self.enter_chain_row(sequence, 0);
            return;
        }
        for voice in &mut self.voices {
            voice.play(Sound::default(), 0);
        }
        if matches!(sequence.route, Route::Chain { .. }) {
            sequence.lanes = [Lane::default(); TRACKS];
            sequence.waiting = true;
            self.sequence = Some(sequence);
        } else {
            self.finished_play = self.plays;
        }
    }

    /// Moves every Track that has finished a Step on to its next one, reading the Project as it
    /// stands now. A Step already sounding is never touched.
    fn advance(&mut self) {
        let Some(sequence) = self.sequence.as_mut() else {
            return;
        };
        let mut moved = [false; TRACKS];
        for (track, moved) in moved.iter_mut().enumerate() {
            let lane = sequence.lanes[track];
            if lane.state == LaneState::Playing && lane.samples_left == 0 {
                let entry = sequence.entry(track);
                sequence.lanes[track].finish_step(entry, &sequence.project);
                *moved = true;
            }
        }
        if moved.contains(&true) {
            if sequence.chain_row_is_over() {
                self.leave_chain_row();
            } else {
                self.start_next_steps(moved);
            }
        }
        if let Some(sequence) = self.sequence.as_mut() {
            for lane in &mut sequence.lanes {
                if lane.state == LaneState::Playing {
                    lane.samples_left -= 1;
                }
            }
        }
    }

    /// Gives each Track that has just moved the Step it moved to, or silence if its entry is over.
    fn start_next_steps(&mut self, moved: [bool; TRACKS]) {
        let Some(sequence) = self.sequence.as_ref() else {
            return;
        };
        for track in (0..TRACKS).filter(|&track| moved[track]) {
            let lane = sequence.lanes[track];
            let sound = sequence
                .sound(track)
                .filter(|_| lane.state == LaneState::Playing);
            match sound {
                Some(sound) => self.voices[track].continue_at(sound, lane.step),
                None => self.voices[track].play(Sound::default(), 0),
            }
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

/// The sound of a Chain entry. It loops over LEN, because the Player decides when each Step and
/// the entry itself end.
fn entry_sound(phrase: &Phrase, entry: ChainEntry) -> Sound {
    sound(phrase, entry.transpose(), 0, phrase.length() as u8)
}

/// The sound of a Phrase, with each Step at its true pitch after the transpose.
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
