//! The App core: named actions in; the character grid out.

mod action;
mod focus;
mod layout;
mod panes;
mod status_line;

pub use action::{Action, Direction};

use crate::screen::grid::Grid;
use focus::Focus;
use panes::chain::ChainPane;
use panes::overview::OverviewPane;
use panes::phrase::PhrasePane;
use panes::song::SongPane;

/// The whole application, with no window or audio device attached.
#[derive(Debug, Default)]
pub struct App {
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

    /// Applies one named action.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Move(direction) => self.move_cursor(direction),
            Action::FocusNext => self.focus = self.focus.next(),
            Action::FocusPrevious => self.focus = self.focus.previous(),
            _ => {}
        }
    }

    fn move_cursor(&mut self, direction: Direction) {
        match self.focus {
            Focus::Song => self.song.move_cursor(direction),
            Focus::Chain => self.chain.move_cursor(direction),
            Focus::Phrase => self.phrase.move_cursor(direction),
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
        self.phrase
            .draw(&mut grid, layout::PHRASE, self.focus == Focus::Phrase);
        self.overview.draw(
            &mut grid,
            layout::RIGHT_SLOT,
            self.focus == Focus::RightSlot,
        );
        layout::draw_separators(&mut grid);
        status_line::draw(&mut grid);
        grid
    }
}
