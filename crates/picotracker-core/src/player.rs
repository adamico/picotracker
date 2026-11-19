//! Player: turns a Project and a play request into the Playhead and samples. The Window shell
//! owns the audio stream that carries them to a device.

use crate::project::{
    CHAIN_ENTRIES, ChainEntry, FIRST_CUSTOM_INSTRUMENT, Filters, InstrumentSound, MAX_STEPS,
    Phrase, PhraseId, Project, SfxInstrument, Step, TRACKS, WaveformInstrument,
};
use crate::synth::{
    Instrument, Instruments, Note, SAMPLE_RATE, SAMPLES_PER_TICK, Sound, Voice, Waveform,
    packed_filters,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioRequest {
    PlayPhrase {
        project: Arc<Project>,
        phrase: PhraseId,
        transpose: i8,
        from_step: usize,
    },
    PlayChain {
        project: Arc<Project>,
        song_row: usize,
        track: usize,
        from_entry: usize,
    },
    PlaySong {
        project: Arc<Project>,
        from_row: usize,
    },
    UpdateProject(Arc<Project>),
    SetTrackMix(TrackMix),
    PreviewStep {
        project: Arc<Project>,
        phrase: Phrase,
        step: usize,
        transpose: i8,
    },
    EndPreview,
    Stop,
}

/// The note being previewed.
#[derive(Clone, Copy, Debug)]
struct Preview {
    samples_left: u64,
    released: bool,
}

/// Holds the mix inside the output range. The gain drops at once to just what the loudest sample
/// needs, so a mix that would not clip passes through unchanged, and then recovers.
#[derive(Clone, Copy, Debug)]
struct Limiter {
    gain: f32,
}

impl Limiter {
    const RELEASE_PER_SAMPLE: f32 = 1.0 / (0.05 * SAMPLE_RATE as f32);
    const FULL_GAIN: f32 = 0.9999;

    const fn new() -> Self {
        Self { gain: 1.0 }
    }

    fn run(&mut self, mix: f32) -> f32 {
        if mix.abs() * self.gain > 1.0 {
            self.gain = 1.0 / mix.abs();
        }
        let limited = (mix * self.gain).clamp(-1.0, 1.0);
        self.gain += (1.0 - self.gain) * Self::RELEASE_PER_SAMPLE;
        if self.gain > Self::FULL_GAIN {
            self.gain = 1.0;
        }
        limited
    }
}

pub const SCOPE_COLUMNS: usize = 23;
pub const SCOPE_RANGE: i8 = 100;
const SAMPLES_PER_SCOPE_COLUMN: usize = 8;
const SCOPE_SAMPLES: usize = SCOPE_COLUMNS * SAMPLES_PER_SCOPE_COLUMN;

/// The latest samples of the mix being heard.
#[derive(Clone, Copy, Debug)]
struct ScopeTrace {
    samples: [f32; SCOPE_SAMPLES],
    next: usize,
}

impl ScopeTrace {
    const fn new() -> Self {
        Self {
            samples: [0.0; SCOPE_SAMPLES],
            next: 0,
        }
    }

    fn push(&mut self, sample: f32) {
        self.samples[self.next] = sample;
        self.next = (self.next + 1) % SCOPE_SAMPLES;
    }

    /// The loudest sample of each column, oldest first.
    fn columns(&self) -> [i8; SCOPE_COLUMNS] {
        let mut columns = [0; SCOPE_COLUMNS];
        for (column, value) in columns.iter_mut().enumerate() {
            let start = self.next + column * SAMPLES_PER_SCOPE_COLUMN;
            let loudest = (start..start + SAMPLES_PER_SCOPE_COLUMN)
                .map(|index| self.samples[index % SCOPE_SAMPLES])
                .fold(0.0_f32, |loudest, sample| {
                    if sample.abs() > loudest.abs() {
                        sample
                    } else {
                        loudest
                    }
                });
            *value = (loudest * f32::from(SCOPE_RANGE)).round() as i8;
        }
        columns
    }
}

/// Which Tracks are muted and soloed. Neither changes any timing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackMix {
    muted: [bool; TRACKS],
    soloed: [bool; TRACKS],
}

impl TrackMix {
    /// Mutes a Track, counted from 0, or brings it back.
    pub fn toggle_mute(&mut self, track: usize) {
        self.muted[track] = !self.muted[track];
    }

    /// Solos a Track, counted from 0, or takes the Solo away.
    pub fn toggle_solo(&mut self, track: usize) {
        self.soloed[track] = !self.soloed[track];
    }

    /// Whether a Track is marked as muted.
    pub fn is_muted(&self, track: usize) -> bool {
        self.muted[track]
    }

    /// Whether a Track is marked as soloed.
    pub fn is_soloed(&self, track: usize) -> bool {
        self.soloed[track]
    }

    /// Whether a Track is heard: Solo overrides Mute.
    pub fn is_heard(&self, track: usize) -> bool {
        if self.soloed.contains(&true) {
            self.soloed[track]
        } else {
            !self.muted[track]
        }
    }
}

/// The place being played, as the Player last reached it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Playhead {
    pub chain_row: Option<(usize, usize)>,
    pub tracks: [Option<(PhraseId, usize)>; TRACKS],
}

impl Playhead {
    /// The Step being played in a Phrase, looking at this Track first.
    pub fn step_in(&self, phrase: PhraseId, track: usize) -> Option<usize> {
        let step_on = |place: &Option<(PhraseId, usize)>| {
            place.filter(|(id, _)| *id == phrase).map(|(_, step)| step)
        };
        step_on(&self.tracks[track]).or_else(|| self.tracks.iter().find_map(step_on))
    }
}

/// What the Player tells the App core about playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackReport {
    pub playhead: Option<Playhead>,
    pub scope: [i8; SCOPE_COLUMNS],
    pub finished_play: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    Song,
    Chain { track: usize },
    Phrase { phrase: PhraseId, transpose: i8 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LaneState {
    #[default]
    Idle,
    Playing,
    Finished,
}

/// One Track's place in the current Chain row. Every Track keeps time, heard or not.
#[derive(Clone, Copy, Debug, Default)]
struct Lane {
    state: LaneState,
    fill: bool,
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
struct Playback {
    project: Arc<Project>,
    route: Route,
    row: usize,
    k: usize,
    lanes: [Lane; TRACKS],
    waiting_for_edit: bool,
}

impl Playback {
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

    /// The place being played.
    fn playhead(&self) -> Playhead {
        let mut tracks = [None; TRACKS];
        for (track, place) in tracks.iter_mut().enumerate() {
            let lane = self.lanes[track];
            if lane.state == LaneState::Playing && self.sounds(track) {
                *place = self.entry(track).map(|entry| (entry.phrase(), lane.step));
            }
        }
        let chain_row = match self.route {
            Route::Phrase { .. } => None,
            _ => Some((self.row, self.k)),
        };
        Playhead { chain_row, tracks }
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
    playback: Option<Playback>,
    track_mix: TrackMix,
    preview: Option<Preview>,
    limiter: Limiter,
    scope: ScopeTrace,
    plays: u64,
    finished_play: u64,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            voices: vec![Voice::new(); TRACKS],
            playback: None,
            track_mix: TrackMix::default(),
            preview: None,
            limiter: Limiter::new(),
            scope: ScopeTrace::new(),
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
            AudioRequest::SetTrackMix(track_mix) => self.track_mix = track_mix,
            AudioRequest::PreviewStep {
                project,
                phrase,
                step,
                transpose,
            } => {
                self.cut();
                self.hear_instruments_of(&project);
                self.start_preview(&phrase, step, transpose);
            }
            AudioRequest::EndPreview => {
                if let Some(preview) = self.preview.as_mut() {
                    preview.released = true;
                }
            }
            AudioRequest::Stop => self.cut(),
        }
    }

    /// The number of the latest play that ended by itself, counting every play request from 1.
    /// It is 0 until one has.
    pub fn finished_play(&self) -> u64 {
        self.finished_play
    }

    /// Where playback has got to, for the App core to show.
    pub fn report(&self) -> PlaybackReport {
        PlaybackReport {
            playhead: self.playback.as_ref().map(Playback::playhead),
            scope: self.scope.columns(),
            finished_play: self.finished_play,
        }
    }

    fn cut(&mut self) {
        self.playback = None;
        self.preview = None;
        self.limiter = Limiter::new();
        self.scope = ScopeTrace::new();
        for voice in &mut self.voices {
            voice.stop();
        }
    }

    /// Gives every voice the Custom Instruments of a Project.
    fn hear_instruments_of(&mut self, project: &Project) {
        let instruments = Arc::new(instruments(project));
        for voice in &mut self.voices {
            voice.set_instruments(Arc::clone(&instruments));
        }
    }

    /// Holds one Step on the first voice, repeating it until the preview ends.
    fn start_preview(&mut self, phrase: &Phrase, step: usize, transpose: i8) {
        let mut held = sound(phrase, transpose, 0, 1);
        held.notes[0] = held.notes[step];
        self.voices[0].play(held, 0);
        self.preview = Some(Preview {
            samples_left: step_samples(phrase),
            released: false,
        });
    }

    /// Ends a released preview once it has sounded for a whole Step.
    fn advance_preview(&mut self) {
        let Some(preview) = self.preview.as_mut() else {
            return;
        };
        preview.samples_left = preview.samples_left.saturating_sub(1);
        if preview.released && preview.samples_left == 0 {
            self.preview = None;
            self.voices[0].play(Sound::default(), 0);
        }
    }

    fn update_project(&mut self, project: Arc<Project>) {
        self.hear_instruments_of(&project);
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        playback.project = project;
        if playback.waiting_for_edit {
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
        self.hear_instruments_of(&project);
        self.plays += 1;
        let Some(k) = k else {
            self.finished_play = self.plays;
            return;
        };
        let playback = Playback {
            project,
            route,
            row,
            k,
            lanes: [Lane::default(); TRACKS],
            waiting_for_edit: false,
        };
        self.enter_chain_row(playback, from_step);
    }

    /// Starts every Track on a Chain row. A Track with no entry lets its sound die away.
    fn enter_chain_row(&mut self, mut playback: Playback, from_step: usize) {
        for (track, voice) in self.voices.iter_mut().enumerate() {
            let lane = Lane::starting(playback.entry(track), &playback.project, from_step);
            playback.lanes[track] = lane;
            voice.play(playback.sound(track).unwrap_or_default(), lane.step);
        }
        self.playback = Some(playback);
    }

    /// Goes on to the next Chain row. With none to go to, a looping Chain waits for an edit and
    /// anything else ends the play.
    fn leave_chain_row(&mut self) {
        let Some(mut playback) = self.playback.take() else {
            return;
        };
        if let Some((row, k)) = playback.next_chain_row() {
            (playback.row, playback.k) = (row, k);
            playback.waiting_for_edit = false;
            self.enter_chain_row(playback, 0);
            return;
        }
        for voice in &mut self.voices {
            voice.play(Sound::default(), 0);
        }
        if matches!(playback.route, Route::Chain { .. }) {
            playback.lanes = [Lane::default(); TRACKS];
            playback.waiting_for_edit = true;
            self.playback = Some(playback);
        } else {
            self.finished_play = self.plays;
        }
    }

    /// Moves every Track that has finished a Step on to its next one, reading the Project as it
    /// stands now. A Step already sounding is never touched.
    fn advance(&mut self) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        let mut moved = [false; TRACKS];
        for (track, moved) in moved.iter_mut().enumerate() {
            let lane = playback.lanes[track];
            if lane.state == LaneState::Playing && lane.samples_left == 0 {
                let entry = playback.entry(track);
                playback.lanes[track].finish_step(entry, &playback.project);
                *moved = true;
            }
        }
        if moved.contains(&true) {
            if playback.chain_row_is_over() {
                self.leave_chain_row();
            } else {
                self.start_next_steps(moved);
            }
        }
        if let Some(playback) = self.playback.as_mut() {
            for lane in &mut playback.lanes {
                if lane.state == LaneState::Playing {
                    lane.samples_left -= 1;
                }
            }
        }
    }

    /// Gives each Track that has just moved the Step it moved to, or silence if its entry is over.
    fn start_next_steps(&mut self, moved: [bool; TRACKS]) {
        let Some(playback) = self.playback.as_ref() else {
            return;
        };
        for track in (0..TRACKS).filter(|&track| moved[track]) {
            let lane = playback.lanes[track];
            let sound = playback
                .sound(track)
                .filter(|_| lane.state == LaneState::Playing);
            match sound {
                Some(sound) => self.voices[track].continue_at(sound, lane.step),
                None => self.voices[track].play(Sound::default(), 0),
            }
        }
    }

    /// The next sample at 22,050 Hz: the Tracks mixed at PICO-8's per-channel level, limited only
    /// when the mix would clip.
    pub fn next_sample(&mut self) -> f32 {
        self.advance();
        self.advance_preview();
        let respects_mix = self
            .playback
            .as_ref()
            .is_some_and(|playback| playback.route == Route::Song);
        let mut mix = 0.0;
        for (track, voice) in self.voices.iter_mut().enumerate() {
            // A silenced voice keeps running, so that bringing it back finds it in time.
            let sample = voice.next_sample();
            if !respects_mix || self.track_mix.is_heard(track) {
                mix += sample;
            }
        }
        let limited = self.limiter.run(mix);
        self.scope.push(limited);
        limited
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
            custom: step.instrument() >= FIRST_CUSTOM_INSTRUMENT,
            volume: step.volume(),
            effect: step.effect(),
        };
    }
    Sound {
        notes,
        speed: phrase.speed(),
        filters: packed(phrase.filters()),
        loop_start,
        loop_end,
    }
}

fn packed(filters: Filters) -> u8 {
    packed_filters(
        filters.noiz(),
        filters.buzz(),
        filters.detune(),
        filters.reverb(),
        filters.dampen(),
    )
}

/// What the synth plays for each of a Project's Custom Instruments.
fn instruments(project: &Project) -> Instruments {
    project
        .custom_instruments()
        .map(|(id, instrument)| {
            let played = match &instrument.sound {
                InstrumentSound::Sfx(sfx) => Instrument::Sfx(sfx_instrument_sound(sfx)),
                InstrumentSound::Waveform(wave) => Instrument::Waveform(waveform(wave)),
            };
            (id, played)
        })
        .collect()
}

/// The sound of an SFX Instrument. Its Steps keep their instrument digits as Built-in
/// Instruments, and the loop plays only when it starts before it ends.
fn sfx_instrument_sound(sfx: &SfxInstrument) -> Sound {
    let mut notes = [Note::default(); MAX_STEPS];
    for (note, step) in notes.iter_mut().zip(&sfx.steps) {
        *note = built_in_note(*step);
    }
    let (loop_start, loop_end) = if sfx.loop_start < sfx.loop_end {
        (sfx.loop_start, sfx.loop_end)
    } else {
        (0, 0)
    };
    Sound {
        notes,
        speed: sfx.speed,
        filters: packed(sfx.filters),
        loop_start,
        loop_end,
    }
}

fn built_in_note(step: Step) -> Note {
    Note {
        pitch: i16::from(step.pitch()),
        instrument: step.instrument(),
        custom: false,
        volume: step.volume(),
        effect: step.effect(),
    }
}

fn waveform(wave: &WaveformInstrument) -> Waveform {
    Waveform {
        samples: wave.samples,
        bass: wave.bass,
        filters: packed_filters(false, false, wave.detune, wave.reverb, wave.dampen),
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
