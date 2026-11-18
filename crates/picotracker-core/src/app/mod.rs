//! The App core: named actions in; the character grid out.

mod action;
mod files;
mod focus;
mod history;
mod layout;
mod messages;
mod panes;
mod status_line;

pub use action::{Action, Direction, KeyContext};
pub use files::{DirectoryEntry, FileRequest, FileResponse};
pub use messages::{Location, LogItem, Report, Severity};

use crate::player::AudioRequest;
use crate::project::CHAIN_ENTRIES;
use crate::project::{Chain, ChainEntry, ChainId, Phrase, PhraseId, Project};
use crate::screen::grid::Grid;
use files::{Question, Waiting};
use focus::Focus;
use history::{History, Merge, Spot};
use messages::Notice;
use panes::chain::ChainPane;
use panes::file_list::{FileList, Purpose};
use panes::log::LogPane;
use panes::overview::OverviewPane;
use panes::phrase::PhrasePane;
use panes::song::SongPane;
use status_line::StatusLine;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const NOTHING_TO_PLAY: &str = "NOTHING TO PLAY";
const NO_CHAIN_HERE: &str = "NO CHAIN HERE";
const NO_PHRASE_HERE: &str = "NO PHRASE HERE";
const NOTHING_TO_UNDO: &str = "NOTHING TO UNDO";
const NOTHING_TO_REDO: &str = "NOTHING TO REDO";
const OPEN_LIST_KEYS: &str = "UP/DOWN PICK  ENTER CHOOSE  ESC CANCEL";
const SAVE_LIST_KEYS: &str = "TYPE A NAME  UP/DOWN PICK  ENTER CHOOSE  ESC CANCEL";

/// The whole application, with no window or audio device attached.
#[derive(Debug)]
pub struct App {
    project: Project,
    /// The Project as it was last saved or opened.
    saved_project: Project,
    history: History,
    /// The file the Project was last saved to or opened from.
    project_path: Option<PathBuf>,
    /// The folder the file list starts in.
    directory: PathBuf,
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
    /// A brief remark about the last action, gone on the next one.
    status: Option<&'static str>,
    /// The latest result on the status line.
    notice: Option<Notice>,
    question: Option<Question>,
    log: LogPane,
    /// Whether the right-hand slot shows the Log instead of the Overview.
    log_open: bool,
    /// The pane that focus goes back to when the Log closes.
    log_return: Focus,
    file_list: Option<FileList>,
    waiting: Option<Waiting>,
    file_requests: Vec<FileRequest>,
    audio_requests: Vec<AudioRequest>,
}

impl Default for App {
    fn default() -> Self {
        Self::with_project(Project::default())
    }
}

impl App {
    /// A new app with the Song pane focused.
    pub fn new() -> Self {
        Self::default()
    }

    /// A new app editing this Project, with the Song pane focused.
    pub fn with_project(project: Project) -> Self {
        Self {
            saved_project: project.clone(),
            project,
            history: History::default(),
            project_path: None,
            directory: PathBuf::from("."),
            focus: Focus::default(),
            song: SongPane::default(),
            chain: ChainPane::default(),
            phrase: PhrasePane::default(),
            overview: OverviewPane::default(),
            played_pane: Focus::default(),
            playing: false,
            plays: 0,
            status: None,
            notice: None,
            question: None,
            log: LogPane::default(),
            log_open: false,
            log_return: Focus::default(),
            file_list: None,
            waiting: None,
            file_requests: Vec::new(),
            audio_requests: Vec::new(),
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
        if self.question.is_some() {
            return KeyContext::Navigation;
        }
        if let Some(list) = &self.file_list {
            return match list.purpose() {
                Purpose::Open => KeyContext::Navigation,
                Purpose::SaveAs => KeyContext::FileName,
            };
        }
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

    /// Tells the user the outcome of something they asked for. A result with items opens the Log
    /// without taking focus.
    pub fn report(&mut self, report: Report) {
        self.notice = Some(Notice::new(report.severity, report.status_text()));
        if !report.items.is_empty() && !self.log_open {
            self.log_open = true;
            self.log_return = self.focus;
        }
        self.log.add(report.into_log_items());
    }

    /// Lets time pass, and says whether the status line now looks different.
    pub fn advance_time(&mut self, elapsed: Duration) -> bool {
        let Some(notice) = self.notice.as_mut() else {
            return false;
        };
        let look = notice.look();
        if notice.grow_older(elapsed) {
            self.notice = None;
            return true;
        }
        notice.look() != look
    }

    /// How long until the status line next changes by itself, if it ever will.
    pub fn time_until_status_change(&self) -> Option<Duration> {
        self.notice.as_ref()?.time_until_change()
    }

    /// Applies one named action. Whatever it changes in the Project becomes one undo step.
    pub fn apply(&mut self, action: Action) {
        self.history.begin(&self.project, self.spot());
        self.perform(action);
        let merge = match action {
            Action::Nudge { .. } => Some(Merge::Nudge),
            Action::HexDigit(_) => Some(Merge::Digit),
            _ => None,
        };
        self.history.finish(&self.project, merge);
    }

    /// Where the cursor is in every pane.
    fn spot(&self) -> Spot {
        Spot {
            focus: self.focus,
            song: self.song.spot(),
            chain: self.chain.spot(),
            phrase: self.phrase.spot(),
        }
    }

    /// Puts the cursor back where a change was made.
    fn return_to(&mut self, spot: Spot) {
        self.set_focus(spot.focus);
        self.song.return_to(spot.song);
        self.chain.return_to(spot.chain);
        self.phrase.return_to(spot.phrase);
    }

    fn undo(&mut self) {
        match self.history.undo(&mut self.project) {
            Some(spot) => self.return_to(spot),
            None => self.status = Some(NOTHING_TO_UNDO),
        }
    }

    fn redo(&mut self) {
        match self.history.redo(&mut self.project) {
            Some(spot) => self.return_to(spot),
            None => self.status = Some(NOTHING_TO_REDO),
        }
    }

    fn perform(&mut self, action: Action) {
        self.status = None;
        if !matches!(action, Action::HexDigit(_)) {
            self.song.finish_typing();
            self.chain.finish_typing();
            self.phrase.finish_typing();
        }
        if self.question.is_some() {
            self.answer_question(action);
            return;
        }
        if self.file_list.is_some() {
            self.use_file_list(action);
            return;
        }
        match action {
            Action::NewProject => self.new_project(),
            Action::OpenProject => self.open_project(),
            Action::Save => self.save(),
            Action::SaveAs => self.save_as(),
            Action::ToggleLog => self.toggle_log(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Confirm => self.confirm(),
            Action::Cancel => self.cancel(),
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

    fn log_has_focus(&self) -> bool {
        self.log_open && self.focus == Focus::RightSlot
    }

    fn toggle_log(&mut self) {
        if self.log_open {
            self.close_log();
        } else {
            self.log_open = true;
            self.log_return = self.focus;
            self.set_focus(Focus::RightSlot);
        }
    }

    fn close_log(&mut self) {
        self.log_open = false;
        if self.focus == Focus::RightSlot {
            self.set_focus(self.log_return);
        }
    }

    /// Enter in the Log jumps to the picked item's location.
    fn confirm(&mut self) {
        if !self.log_has_focus() {
            return;
        }
        if let Some(location) = self.log.picked_location() {
            self.song.jump_to(location.song_row, location.track);
            self.chain.jump_to(location.chain_row);
            self.phrase.jump_to(location.step);
            self.set_focus(Focus::Phrase);
        }
    }

    /// Esc dismisses a lasting result first, and then closes the Log.
    fn cancel(&mut self) {
        if self.notice.as_ref().is_some_and(Notice::is_persistent) {
            self.notice = None;
        } else if self.log_has_focus() {
            self.close_log();
        }
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
            Focus::RightSlot if self.log_open => self.log.move_cursor(direction),
            Focus::RightSlot => self.overview.move_cursor(direction),
        }
    }

    /// Draws the current state onto a fresh character grid.
    pub fn grid(&self) -> Grid {
        let mut grid = Grid::new();
        // The file list takes the keys while it is open, so no pane shows a cursor.
        let focus = self.file_list.is_none().then_some(self.focus);
        self.song.draw(
            &mut grid,
            layout::SONG,
            focus == Some(Focus::Song),
            &self.project,
        );
        self.chain.draw(
            &mut grid,
            layout::CHAIN,
            focus == Some(Focus::Chain),
            &self.project,
            self.song.row(),
            self.song.track(),
        );
        self.phrase.draw(
            &mut grid,
            layout::PHRASE,
            focus == Some(Focus::Phrase),
            self.phrase_under_cursor(),
            &self.shown_phrase(),
            self.entry_under_cursor().map_or(0, ChainEntry::transpose),
        );
        let right_slot_focused = focus == Some(Focus::RightSlot);
        if let Some(list) = &self.file_list {
            list.draw(&mut grid, layout::RIGHT_SLOT);
        } else if self.log_open {
            self.log
                .draw(&mut grid, layout::RIGHT_SLOT, right_slot_focused);
        } else {
            self.overview
                .draw(&mut grid, layout::RIGHT_SLOT, right_slot_focused);
        }
        layout::draw_separators(&mut grid);
        let project = self.project_name();
        status_line::draw(&mut grid, &self.status_line(&project));
        grid
    }

    /// What the status line shows: what most needs the user's attention comes first.
    fn status_line<'a>(&'a self, project: &'a str) -> StatusLine<'a> {
        if let Some(question) = &self.question {
            return StatusLine::Question(&question.text);
        }
        let notice = self.notice.as_ref();
        if let Some(notice) = notice.filter(|notice| notice.is_persistent()) {
            return StatusLine::Notice(notice);
        }
        if let Some(remark) = self.status {
            return StatusLine::Remark(remark);
        }
        if let Some(notice) = notice {
            return StatusLine::Notice(notice);
        }
        match self.file_list.as_ref().map(FileList::purpose) {
            Some(Purpose::Open) => StatusLine::Hints(OPEN_LIST_KEYS),
            Some(Purpose::SaveAs) => StatusLine::Hints(SAVE_LIST_KEYS),
            None => StatusLine::Idle {
                project,
                unsaved: self.has_unsaved_changes(),
            },
        }
    }
}
