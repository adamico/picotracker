//! The App core: named actions in; the character grid out.

mod action;
mod focus;
mod layout;
mod panes;
mod status_line;

pub use action::{Action, Direction, KeyContext};

use crate::player::AudioRequest;
use crate::project::CHAIN_ENTRIES;
use crate::project::{Chain, ChainEntry, ChainId, Phrase, PhraseId, Project};
use crate::screen::grid::Grid;
use focus::Focus;
use panes::chain::ChainPane;
use panes::overview::OverviewPane;
use panes::phrase::PhrasePane;
use panes::song::SongPane;
use std::sync::Arc;

const NOTHING_TO_PLAY: &str = "NOTHING TO PLAY";
const NO_CHAIN_HERE: &str = "NO CHAIN HERE";
const NO_PHRASE_HERE: &str = "NO PHRASE HERE";

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

    /// Applies one named action.
    pub fn apply(&mut self, action: Action) {
        self.status = None;
        if !matches!(action, Action::HexDigit(_)) {
            self.song.finish_typing();
            self.chain.finish_typing();
            self.phrase.finish_typing();
        }
        match action {
            Action::Move(direction) => self.move_cursor(direction),
            Action::FocusNext => self.set_focus(self.focus.next()),
            Action::FocusPrevious => self.set_focus(self.focus.previous()),
            Action::PlayFocusedPane => self.play_or_stop(),
            Action::PlaySongFromCursor => self.play_song(),
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
        let shown = self.shown_phrase();
        self.phrase.keep_inside(&shown);
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
        self.plays += 1;
        self.audio_requests.push(request);
    }

    fn play_song(&mut self) {
        let from_row = self.song.row();
        if self.project.song_row_is_empty(from_row) {
            self.status = Some(NOTHING_TO_PLAY);
            return;
        }
        self.start_playing(AudioRequest::PlaySong {
            project: Arc::new(self.project.clone()),
            from_row,
        });
    }

    fn play_chain(&mut self) {
        if !self.chain_under_cursor_sounds() {
            self.status = Some(NOTHING_TO_PLAY);
            return;
        }
        self.start_playing(AudioRequest::PlayChain {
            project: Arc::new(self.project.clone()),
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
        let phrase = self.shown_phrase();
        if phrase.is_silent() {
            self.status = Some(NOTHING_TO_PLAY);
            return;
        }
        self.start_playing(AudioRequest::PlayPhrase {
            phrase,
            transpose: self.entry_under_cursor().map_or(0, ChainEntry::transpose),
            from_step: self.phrase.step(),
        });
    }

    fn move_cursor(&mut self, direction: Direction) {
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
        self.song.draw(
            &mut grid,
            layout::SONG,
            self.focus == Focus::Song,
            &self.project,
        );
        self.chain.draw(
            &mut grid,
            layout::CHAIN,
            self.focus == Focus::Chain,
            &self.project,
            self.song.row(),
            self.song.track(),
        );
        self.phrase.draw(
            &mut grid,
            layout::PHRASE,
            self.focus == Focus::Phrase,
            self.phrase_under_cursor(),
            &self.shown_phrase(),
            self.entry_under_cursor().map_or(0, ChainEntry::transpose),
        );
        self.overview.draw(
            &mut grid,
            layout::RIGHT_SLOT,
            self.focus == Focus::RightSlot,
        );
        layout::draw_separators(&mut grid);
        status_line::draw(&mut grid, self.status);
        grid
    }
}
