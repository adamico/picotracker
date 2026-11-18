//! The App core: named actions in; the character grid out.

mod action;
mod focus;
mod layout;
mod panes;
mod status_line;

pub use action::{Action, Direction, KeyContext};

use crate::player::{AudioRequest, PlaybackReport, Playhead, SCOPE_COLUMNS, TrackMix};
use crate::project::CHAIN_ENTRIES;
use crate::project::{Chain, ChainEntry, ChainId, Phrase, PhraseId, Project, Step};
use crate::screen::grid::Grid;
use focus::Focus;
use panes::chain::{ChainPane, ChainView};
use panes::overview::OverviewPane;
use panes::phrase::{PhrasePane, PhraseView};
use panes::song::SongPane;
use std::sync::Arc;

const NOTHING_TO_PLAY: &str = "NOTHING TO PLAY";
const NO_CHAIN_HERE: &str = "NO CHAIN HERE";
const NO_PHRASE_HERE: &str = "NO PHRASE HERE";
const FOLLOW_ON: &str = "FOLLOW ON";
const FOLLOW_OFF: &str = "FOLLOW OFF";

/// The whole application, with no window or audio device attached.
#[derive(Debug, Default)]
pub struct App {
    project: Project,
    focus: Focus,
    song: SongPane,
    chain: ChainPane,
    phrase: PhrasePane,
    overview: OverviewPane,
    /// The last focused of the Song, Chain and Phrase panes, which Space plays.
    played_pane: Focus,
    playing: bool,
    /// How many plays have been asked for, which numbers the latest one.
    plays: u64,
    /// The Project as the Player last heard it.
    heard: Arc<Project>,
    track_mix: TrackMix,
    playhead: Option<Playhead>,
    /// Whether the panes track the Playhead.
    follow: bool,
    scope: [i8; SCOPE_COLUMNS],
    /// Whether a note is sounding until its key is released.
    previewing: bool,
    status: Option<&'static str>,
    audio_requests: Vec<AudioRequest>,
}

impl App {
    /// A new app with the Song pane focused.
    pub fn new() -> Self {
        Self::default()
    }

    /// A new app editing this Project, with the Song pane focused.
    pub fn with_project(project: Project) -> Self {
        Self {
            project,
            ..Self::default()
        }
    }

    /// The Project being edited.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The Chain under the Song cursor, which the Chain pane shows.
    pub fn chain_under_cursor(&self) -> Option<ChainId> {
        self.project.song_cell(self.song.row(), self.song.track())
    }

    /// The Phrase under the Chain cursor, which the Phrase pane shows.
    pub fn phrase_under_cursor(&self) -> Option<PhraseId> {
        self.entry_under_cursor().map(ChainEntry::phrase)
    }

    fn entry_under_cursor(&self) -> Option<ChainEntry> {
        let chain = self.chain_under_cursor()?;
        self.project.chain(chain).entry(self.chain.entry())
    }

    /// The Phrase the Phrase pane draws: a blank one when no Phrase is under the Chain cursor.
    fn shown_phrase(&self) -> Phrase {
        self.phrase_under_cursor()
            .map_or_else(Phrase::default, |id| *self.project.phrase(id))
    }

    /// What the keys that type characters do at the cursor.
    pub fn key_context(&self) -> KeyContext {
        match self.focus {
            Focus::Song => KeyContext::ArrangeValue,
            Focus::Chain if self.chain.takes_digits() => KeyContext::ArrangeValue,
            Focus::Phrase => self.phrase.key_context(),
            _ => KeyContext::Navigation,
        }
    }

    /// Hands over the audio requests made since the last call.
    pub fn take_audio_requests(&mut self) -> Vec<AudioRequest> {
        std::mem::take(&mut self.audio_requests)
    }

    /// Tells the app that a play has ended by itself, by the number the Player gave it. A report
    /// about any play but the latest changes nothing.
    pub fn playback_finished(&mut self, play: u64) {
        if play == self.plays {
            self.playing = false;
        }
    }

    /// Takes in where playback has got to, and says whether the screen needs drawing again.
    pub fn show_playback(&mut self, report: &PlaybackReport) -> bool {
        self.playback_finished(report.finished_play);
        let playhead = report.playhead.filter(|_| self.playing);
        let moved = playhead != self.playhead;
        let changed = moved || report.scope != self.scope;
        self.playhead = playhead;
        self.scope = report.scope;
        if moved && self.follow {
            self.follow_playhead();
        }
        changed
    }

    /// Tells the app that the held key has gone up, which ends a note preview.
    pub fn key_released(&mut self) {
        if self.previewing {
            self.previewing = false;
            self.audio_requests.push(AudioRequest::EndPreview);
        }
    }

    /// The Step about to be edited, if the action can change a note that a preview would play.
    fn step_before_edit(&self, action: Action) -> Option<(usize, Step)> {
        let edits_note = matches!(
            action,
            Action::PianoKey(_) | Action::HexDigit(_) | Action::Nudge { .. }
        );
        if !edits_note || self.focus != Focus::Phrase {
            return None;
        }
        let index = self.phrase.note_step()?;
        Some((index, self.shown_phrase().step(index)))
    }

    /// Previews an entered or changed note while nothing is playing.
    fn preview_edit(&mut self, action: Action, index: usize, before: Step) {
        let phrase = self.shown_phrase();
        let step = phrase.step(index);
        let changed = step.pitch() != before.pitch() || step.instrument() != before.instrument();
        let entered = matches!(action, Action::PianoKey(_));
        if self.playing || step.is_silent() || !(entered || changed) {
            return;
        }
        self.previewing = true;
        self.audio_requests.push(AudioRequest::PreviewStep {
            phrase,
            step: index,
            transpose: self.entry_under_cursor().map_or(0, ChainEntry::transpose),
        });
    }

    /// Moves the panes to the Playhead.
    fn follow_playhead(&mut self) {
        let Some(playhead) = self.playhead else {
            return;
        };
        if let Some((row, k)) = playhead.chain_row {
            self.song.follow(row);
            self.chain.follow(k);
        }
        let step = self
            .phrase_under_cursor()
            .and_then(|phrase| playhead.step_in(phrase, self.song.track()));
        if let Some(step) = step {
            self.phrase.follow(step);
        }
        let shown = self.shown_phrase();
        self.phrase.keep_inside(&shown);
    }

    fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        self.status = Some(if self.follow { FOLLOW_ON } else { FOLLOW_OFF });
    }

    /// Applies one named action.
    pub fn apply(&mut self, action: Action) {
        self.status = None;
        if !matches!(action, Action::HexDigit(_)) {
            self.song.finish_typing();
            self.chain.finish_typing();
            self.phrase.finish_typing();
        }
        let before_edit = self.step_before_edit(action);
        match action {
            Action::Move(direction) => self.move_cursor(direction),
            Action::FocusNext => self.set_focus(self.focus.next()),
            Action::FocusPrevious => self.set_focus(self.focus.previous()),
            Action::PlayFocusedPane => self.play_or_stop(),
            Action::PlaySongFromCursor => self.play_song(),
            Action::ToggleFollow => self.toggle_follow(),
            Action::Mute(track) => self.change_track_mix(|mix| {
                mix.toggle_mute(usize::from(track) - 1);
            }),
            Action::Solo(track) => self.change_track_mix(|mix| {
                mix.toggle_solo(usize::from(track) - 1);
            }),
            Action::ClearMutesAndSolos => {
                self.change_track_mix(|mix| *mix = TrackMix::default());
            }
            Action::OctaveUp => self.phrase.change_octave(1),
            Action::OctaveDown => self.phrase.change_octave(-1),
            Action::SetSongLoopRow => self.song.toggle_loop_row(&mut self.project),
            Action::ToggleFill if self.focus == Focus::Chain => self.toggle_fill(),
            Action::Clear => self.clear(),
            Action::PianoKey(key) if self.focus == Focus::Phrase => {
                self.edit_phrase(|pane, phrase| pane.enter_note(phrase, key));
            }
            Action::HexDigit(digit) => self.type_digit(digit),
            Action::Nudge { up, coarse } => self.nudge(up, coarse),
            _ => {}
        }
        if let Some((index, before)) = before_edit {
            self.preview_edit(action, index, before);
        }
        let shown = self.shown_phrase();
        self.phrase.keep_inside(&shown);
        self.send_edits_to_the_player();
    }

    fn change_track_mix(&mut self, change: impl FnOnce(&mut TrackMix)) {
        change(&mut self.track_mix);
        self.audio_requests
            .push(AudioRequest::SetTrackMix(self.track_mix));
    }

    /// Passes the Project to the Player when an action has changed what is playing.
    fn send_edits_to_the_player(&mut self) {
        if self.playing && *self.heard != self.project {
            let project = self.project_for_player();
            self.audio_requests
                .push(AudioRequest::UpdateProject(project));
        }
    }

    /// A copy of the Project for the Player, remembered as what it has heard.
    fn project_for_player(&mut self) -> Arc<Project> {
        self.heard = Arc::new(self.project.clone());
        Arc::clone(&self.heard)
    }

    fn type_digit(&mut self, digit: u8) {
        match self.focus {
            Focus::Song => self.song.type_digit(&mut self.project, digit),
            Focus::Chain => self.edit_chain(|pane, chain| pane.type_digit(chain, digit)),
            Focus::Phrase => self.edit_phrase(|pane, phrase| pane.type_digit(phrase, digit)),
            Focus::RightSlot => {}
        }
    }

    fn nudge(&mut self, up: bool, coarse: bool) {
        match self.focus {
            Focus::Song => self.song.nudge(&mut self.project, up, coarse),
            Focus::Chain => self.edit_chain(|pane, chain| pane.nudge(chain, up, coarse)),
            Focus::Phrase => self.edit_phrase(|pane, phrase| pane.nudge(phrase, up, coarse)),
            Focus::RightSlot => {}
        }
    }

    fn clear(&mut self) {
        match self.focus {
            Focus::Song => self.song.clear(&mut self.project),
            Focus::Chain => self.edit_chain(|pane, chain| pane.clear(chain)),
            Focus::Phrase | Focus::RightSlot => {}
        }
    }

    fn toggle_fill(&mut self) {
        if self.chain_under_cursor().is_some() && self.entry_under_cursor().is_none() {
            self.status = Some(NO_PHRASE_HERE);
            return;
        }
        self.edit_chain(|pane, chain| pane.toggle_fill(chain));
    }

    /// Edits the Chain under the Song cursor, or says why there is nothing to edit.
    fn edit_chain(&mut self, edit: impl FnOnce(&mut ChainPane, &mut Chain)) {
        let Some(id) = self.chain_under_cursor() else {
            self.status = Some(NO_CHAIN_HERE);
            return;
        };
        if self.entry_under_cursor().is_none() && !self.chain.edits_empty_entries() {
            self.status = Some(NO_PHRASE_HERE);
            return;
        }
        edit(&mut self.chain, self.project.chain_mut(id));
    }

    /// Edits the Phrase under the Chain cursor, or says that there is none.
    fn edit_phrase(&mut self, edit: impl FnOnce(&mut PhrasePane, &mut Phrase)) {
        match self.phrase_under_cursor() {
            Some(id) => edit(&mut self.phrase, self.project.phrase_mut(id)),
            None => self.status = Some(NO_PHRASE_HERE),
        }
    }

    fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
        if focus != Focus::RightSlot {
            self.played_pane = focus;
        }
    }

    fn play_or_stop(&mut self) {
        if self.playing {
            self.playing = false;
            self.playhead = None;
            self.audio_requests.push(AudioRequest::Stop);
            return;
        }
        match self.played_pane {
            Focus::Song | Focus::RightSlot => self.play_song(),
            Focus::Chain => self.play_chain(),
            Focus::Phrase => self.play_phrase(),
        }
    }

    fn start_playing(&mut self, request: AudioRequest) {
        self.playing = true;
        self.previewing = false;
        self.plays += 1;
        self.audio_requests.push(request);
    }

    fn play_song(&mut self) {
        let from_row = self.song.row();
        if self.project.song_row_is_empty(from_row) {
            self.status = Some(NOTHING_TO_PLAY);
            return;
        }
        let project = self.project_for_player();
        self.start_playing(AudioRequest::PlaySong { project, from_row });
    }

    fn play_chain(&mut self) {
        if !self.chain_under_cursor_sounds() {
            self.status = Some(NOTHING_TO_PLAY);
            return;
        }
        let project = self.project_for_player();
        self.start_playing(AudioRequest::PlayChain {
            project,
            song_row: self.song.row(),
            track: self.song.track(),
            from_entry: self.chain.entry(),
        });
    }

    /// Whether any entry of the Chain under the Song cursor holds a Phrase with a note in it.
    fn chain_under_cursor_sounds(&self) -> bool {
        self.chain_under_cursor().is_some_and(|id| {
            let chain = self.project.chain(id);
            (0..CHAIN_ENTRIES)
                .filter_map(|index| chain.entry(index))
                .any(|entry| !self.project.phrase(entry.phrase()).is_silent())
        })
    }

    fn play_phrase(&mut self) {
        let Some(phrase) = self
            .phrase_under_cursor()
            .filter(|id| !self.project.phrase(*id).is_silent())
        else {
            self.status = Some(NOTHING_TO_PLAY);
            return;
        };
        let project = self.project_for_player();
        self.start_playing(AudioRequest::PlayPhrase {
            project,
            phrase,
            transpose: self.entry_under_cursor().map_or(0, ChainEntry::transpose),
            from_step: self.phrase.step(),
        });
    }

    fn move_cursor(&mut self, direction: Direction) {
        if self.follow {
            self.follow = false;
            self.status = Some(FOLLOW_OFF);
        }
        match self.focus {
            Focus::Song => self.song.move_cursor(direction),
            Focus::Chain => self.chain.move_cursor(direction),
            Focus::Phrase => {
                let phrase = self.shown_phrase();
                self.phrase.move_cursor(direction, &phrase);
            }
            Focus::RightSlot => self.overview.move_cursor(direction),
        }
    }

    /// Draws the current state onto a fresh character grid.
    pub fn grid(&self) -> Grid {
        let mut grid = Grid::new();
        let chain_row = self.playhead.and_then(|playhead| playhead.chain_row);
        self.song.draw(
            &mut grid,
            layout::SONG,
            self.focus == Focus::Song,
            &self.project,
            &self.track_mix,
            chain_row.map(|(row, _)| row),
        );
        self.chain.draw(
            &mut grid,
            layout::CHAIN,
            self.focus == Focus::Chain,
            &self.project,
            ChainView {
                song_row: self.song.row(),
                track: self.song.track(),
                playing_entry: chain_row
                    .filter(|(row, _)| *row == self.song.row())
                    .map(|(_, k)| k),
            },
        );
        let phrase = self.phrase_under_cursor();
        self.phrase.draw(
            &mut grid,
            layout::PHRASE,
            self.focus == Focus::Phrase,
            PhraseView {
                id: phrase,
                phrase: &self.shown_phrase(),
                transpose: self.entry_under_cursor().map_or(0, ChainEntry::transpose),
                playing_step: self
                    .playhead
                    .zip(phrase)
                    .and_then(|(playhead, phrase)| playhead.step_in(phrase, self.song.track())),
            },
        );
        self.overview.draw(
            &mut grid,
            layout::RIGHT_SLOT,
            self.focus == Focus::RightSlot,
            &self.scope,
        );
        layout::draw_separators(&mut grid);
        status_line::draw(&mut grid, self.status);
        grid
    }
}
