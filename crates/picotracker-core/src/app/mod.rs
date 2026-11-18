//! The App core: named actions in; the character grid out.

mod action;
mod focus;
mod layout;
mod panes;
mod status_line;

pub use action::{Action, Direction, KeyContext};

use crate::project::{FIRST_PHRASE, PhraseId, Project};
use crate::screen::grid::Grid;
use focus::Focus;
use panes::chain::ChainPane;
use panes::overview::OverviewPane;
use panes::phrase::PhrasePane;
use panes::song::SongPane;

/// The whole application, with no window or audio device attached.
#[derive(Debug, Default)]
pub struct App {
    project: Project,
    focus: Focus,
    song: SongPane,
    chain: ChainPane,
    phrase: PhrasePane,
    overview: OverviewPane,
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

    /// Applies one named action.
    pub fn apply(&mut self, action: Action) {
        if !matches!(action, Action::HexDigit(_)) {
            self.phrase.finish_typing();
        }
        match action {
            Action::Move(direction) => self.move_cursor(direction),
            Action::FocusNext => self.focus = self.focus.next(),
            Action::FocusPrevious => self.focus = self.focus.previous(),
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
        status_line::draw(&mut grid, None);
        grid
    }
}
