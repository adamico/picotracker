//! The App core: named actions in; the character grid out.

mod action;
mod focus;
mod layout;
mod panes;
mod status_line;

pub use action::{Action, Direction, KeyContext};

use crate::player::AudioRequest;
use crate::project::{FIRST_PHRASE, PhraseId, Project};
use crate::screen::grid::Grid;
use focus::Focus;
use panes::chain::ChainPane;
use panes::overview::OverviewPane;
use panes::phrase::PhrasePane;
use panes::song::SongPane;

const NOTHING_TO_PLAY: &str = "NOTHING TO PLAY";

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
    status: Option<&'static str>,
    audio_requests: Vec<AudioRequest>,
}

impl App {
    /// A new app with the Song pane focused.
    pub fn new() -> Self {
        Self::default()
    }

    /// The Project being edited.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The Phrase the Phrase pane shows. Until Chains exist this is always the first Phrase; this
    /// is the one place that decides it.
    pub fn edited_phrase(&self) -> PhraseId {
        FIRST_PHRASE
    }

    /// What the keys that type characters do at the cursor.
    pub fn key_context(&self) -> KeyContext {
        match self.focus {
            Focus::Phrase => self.phrase.key_context(),
            _ => KeyContext::Navigation,
        }
    }

    /// Hands over the audio requests made since the last call.
    pub fn take_audio_requests(&mut self) -> Vec<AudioRequest> {
        std::mem::take(&mut self.audio_requests)
    }

    /// Applies one named action.
    pub fn apply(&mut self, action: Action) {
        self.status = None;
        if !matches!(action, Action::HexDigit(_)) {
            self.phrase.finish_typing();
        }
        match action {
            Action::Move(direction) => self.move_cursor(direction),
            Action::FocusNext => self.set_focus(self.focus.next()),
            Action::FocusPrevious => self.set_focus(self.focus.previous()),
            Action::PlayFocusedPane => self.play_or_stop(),
            Action::OctaveUp => self.phrase.change_octave(1),
            Action::OctaveDown => self.phrase.change_octave(-1),
            Action::PianoKey(key) if self.focus == Focus::Phrase => {
                let id = self.edited_phrase();
                self.phrase.enter_note(self.project.phrase_mut(id), key);
            }
            Action::HexDigit(digit) if self.focus == Focus::Phrase => {
                let id = self.edited_phrase();
                self.phrase.type_digit(self.project.phrase_mut(id), digit);
            }
            Action::Nudge { up, coarse } if self.focus == Focus::Phrase => {
                let id = self.edited_phrase();
                self.phrase.nudge(self.project.phrase_mut(id), up, coarse);
            }
            _ => {}
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
        } else if self.played_pane == Focus::Phrase {
            self.play_phrase();
        }
    }

    fn play_phrase(&mut self) {
        let phrase = *self.project.phrase(self.edited_phrase());
        if phrase.is_silent() {
            self.status = Some(NOTHING_TO_PLAY);
            return;
        }
        self.playing = true;
        self.audio_requests.push(AudioRequest::PlayPhrase {
            phrase,
            from_step: self.phrase.step(),
        });
    }

    fn move_cursor(&mut self, direction: Direction) {
        match self.focus {
            Focus::Song => self.song.move_cursor(direction),
            Focus::Chain => self.chain.move_cursor(direction),
            Focus::Phrase => {
                let phrase = self.project.phrase(self.edited_phrase());
                self.phrase.move_cursor(direction, phrase);
            }
            Focus::RightSlot => self.overview.move_cursor(direction),
        }
    }

    /// Draws the current state onto a fresh character grid.
    pub fn grid(&self) -> Grid {
        let mut grid = Grid::new();
        self.song
            .draw(&mut grid, layout::SONG, self.focus == Focus::Song);
        self.chain.draw(
            &mut grid,
            layout::CHAIN,
            self.focus == Focus::Chain,
            self.song.row(),
            self.song.track(),
        );
        self.phrase.draw(
            &mut grid,
            layout::PHRASE,
            self.focus == Focus::Phrase,
            self.project.phrase(self.edited_phrase()),
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
