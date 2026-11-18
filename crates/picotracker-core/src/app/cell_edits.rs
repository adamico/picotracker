//! Selection, cells Nudge and Clear, and the clipboard.

use super::messages::Report;
use super::selection::{CellPosition, CellRange};
use super::{App, Chain, ChainPane, Focus, NO_CHAIN_HERE, NO_PHRASE_HERE};
use crate::project::{ChainEntry, ChainId, Step};
use crate::screen::grid::Grid;
use std::ops::RangeInclusive;

pub(super) const NO_STEP_HERE: &str = "NO STEP HERE";
const NOTHING_TO_PASTE: &str = "NOTHING TO PASTE";

/// What was last copied, which remembers the kind of pane it came from.
#[derive(Clone, Debug)]
pub(super) enum Clipboard {
    /// References to Chains, row by row.
    Song(Vec<Vec<Option<ChainId>>>),
    /// Chain entries, and which of their columns were copied.
    Chain {
        columns: RangeInclusive<usize>,
        rows: Vec<Option<ChainEntry>>,
    },
    /// Step values, and which of their columns were copied.
    Phrase {
        columns: RangeInclusive<usize>,
        rows: Vec<Step>,
    },
}

impl Clipboard {
    /// What the clipboard holds, as messages name it.
    const fn contents(&self) -> &'static str {
        match self {
            Clipboard::Song(_) => "SONG CELLS",
            Clipboard::Chain { .. } => "CHAIN ENTRIES",
            Clipboard::Phrase { .. } => "STEPS",
        }
    }
}

/// The name of a pane in messages.
const fn pane_name(focus: Focus) -> &'static str {
    match focus {
        Focus::Song => "SONG",
        Focus::Chain => "CHAIN",
        Focus::Phrase => "PHRASE",
        Focus::RightSlot => "RIGHT-HAND",
    }
}

impl App {
    /// The cell under the cursor of the focused pane, if a selection can hold it.
    fn cursor_cell(&self) -> Option<CellPosition> {
        match self.focus {
            Focus::Song => Some(self.song.cell()),
            Focus::Chain => Some(self.chain.position()),
            Focus::Phrase => self.phrase.cell(),
            Focus::RightSlot => None,
        }
    }

    /// The cells selected in the focused pane.
    pub(super) fn selected_cells(&self) -> Option<CellRange> {
        Some(CellRange::between(self.selection?, self.cursor_cell()?))
    }

    /// The cells an action works on: the selection, or else the cell under the cursor.
    pub(super) fn target_cells(&self) -> Option<CellRange> {
        self.selected_cells()
            .or_else(|| self.cursor_cell().map(CellRange::at))
    }

    /// Starts a selection at the cursor. The arrow keys then extend it.
    pub(super) fn start_selection(&mut self) {
        match self.cursor_cell() {
            Some(cell) => {
                self.stop_following();
                self.selection = Some(cell);
            }
            None if self.focus == Focus::Phrase => self.status = Some(NO_STEP_HERE),
            None => {}
        }
    }

    /// Edits the Chain under the Song cursor, or says that there is none.
    fn edit_selected_chain(&mut self, edit: impl FnOnce(&ChainPane, &mut Chain)) {
        match self.chain_under_cursor() {
            Some(id) => edit(&self.chain, self.project.chain_mut(id)),
            None => self.status = Some(NO_CHAIN_HERE),
        }
    }

    /// Raises or lowers every selected cell.
    pub(super) fn nudge_selection(&mut self, up: bool, coarse: bool) {
        let Some(cells) = self.selected_cells() else {
            return;
        };
        match self.focus {
            Focus::Song => self.song.nudge_cells(&mut self.project, cells, up, coarse),
            Focus::Chain => {
                self.edit_selected_chain(|pane, chain| pane.nudge_cells(chain, cells, up, coarse));
            }
            Focus::Phrase => {
                self.edit_phrase(|pane, phrase| pane.nudge_cells(phrase, cells, up, coarse));
            }
            Focus::RightSlot => {}
        }
    }

    /// Empties the selected cells, or else the cell under the cursor.
    pub(super) fn clear_cells(&mut self) {
        match (self.focus, self.selected_cells()) {
            (Focus::Song, Some(cells)) => self.song.clear_cells(&mut self.project, cells),
            (Focus::Song, None) => self.song.clear(&mut self.project),
            (Focus::Chain, Some(cells)) => {
                self.edit_selected_chain(|pane, chain| pane.clear_cells(chain, cells));
            }
            (Focus::Chain, None) => self.edit_chain(|pane, chain| pane.clear(chain)),
            (Focus::Phrase, _) => {
                if let Some(cells) = self.target_cells() {
                    self.edit_phrase(|pane, phrase| pane.clear_cells(phrase, cells));
                }
            }
            (Focus::RightSlot, _) if !self.log_open => self.overview.clear(&mut self.project),
            (Focus::RightSlot, _) => {}
        }
    }

    /// A copy of the cells an action works on, or a remark about why there is none.
    fn copied_cells(&mut self) -> Option<Clipboard> {
        let cells = self.target_cells();
        match self.focus {
            Focus::Song => Some(Clipboard::Song(self.song.copy(&self.project, cells?))),
            Focus::Chain => {
                let Some(id) = self.chain_under_cursor() else {
                    self.status = Some(NO_CHAIN_HERE);
                    return None;
                };
                let cells = cells?;
                Some(Clipboard::Chain {
                    columns: cells.columns(),
                    rows: self.chain.copy(self.project.chain(id), cells),
                })
            }
            Focus::Phrase => {
                let Some(id) = self.phrase_under_cursor() else {
                    self.status = Some(NO_PHRASE_HERE);
                    return None;
                };
                let Some(cells) = cells else {
                    self.status = Some(NO_STEP_HERE);
                    return None;
                };
                Some(Clipboard::Phrase {
                    columns: cells.columns(),
                    rows: self.phrase.copy(self.project.phrase(id), cells),
                })
            }
            Focus::RightSlot => None,
        }
    }

    pub(super) fn copy(&mut self) {
        if let Some(copied) = self.copied_cells() {
            self.clipboard = Some(copied);
            self.selection = None;
        }
    }

    /// Copies the cells and empties them, without shifting anything.
    pub(super) fn cut(&mut self) {
        if let Some(copied) = self.copied_cells() {
            self.clipboard = Some(copied);
            self.clear_cells();
            self.selection = None;
        }
    }

    /// Overwrites cells from the cursor, and says how many rows did not fit.
    pub(super) fn paste(&mut self) {
        if self.focus == Focus::RightSlot {
            return;
        }
        let Some(clipboard) = self.clipboard.clone() else {
            self.status = Some(NOTHING_TO_PASTE);
            return;
        };
        let lost = match (&clipboard, self.focus) {
            (Clipboard::Song(rows), Focus::Song) => Some(self.song.paste(&mut self.project, rows)),
            (Clipboard::Chain { columns, rows }, Focus::Chain) => {
                self.paste_into_chain(columns, rows)
            }
            (Clipboard::Phrase { columns, rows }, Focus::Phrase) => {
                self.paste_into_phrase(columns, rows)
            }
            _ => {
                self.report(Report::failure(format!(
                    "CANNOT PASTE {} INTO THE {} PANE",
                    clipboard.contents(),
                    pane_name(self.focus),
                )));
                return;
            }
        };
        let Some(lost) = lost else {
            return;
        };
        self.selection = None;
        if lost > 0 {
            let rows = if lost == 1 { "ROW" } else { "ROWS" };
            self.report(Report::warning(format!("PASTE LOST {lost} {rows}")));
        }
    }

    fn paste_into_chain(
        &mut self,
        columns: &RangeInclusive<usize>,
        rows: &[Option<ChainEntry>],
    ) -> Option<usize> {
        let Some(id) = self.chain_under_cursor() else {
            self.status = Some(NO_CHAIN_HERE);
            return None;
        };
        Some(self.chain.paste(self.project.chain_mut(id), columns, rows))
    }

    fn paste_into_phrase(
        &mut self,
        columns: &RangeInclusive<usize>,
        rows: &[Step],
    ) -> Option<usize> {
        let Some(id) = self.phrase_under_cursor() else {
            self.status = Some(NO_PHRASE_HERE);
            return None;
        };
        if self.phrase.cell().is_none() {
            self.status = Some(NO_STEP_HERE);
            return None;
        }
        Some(
            self.phrase
                .paste(self.project.phrase_mut(id), columns, rows),
        )
    }

    /// Marks the selected cells on the grid.
    pub(super) fn draw_selection(&self, grid: &mut Grid) {
        let Some(cells) = self.selected_cells() else {
            return;
        };
        match self.focus {
            Focus::Song => self.song.draw_selection(grid, super::layout::SONG, cells),
            Focus::Chain => self.chain.draw_selection(grid, super::layout::CHAIN, cells),
            Focus::Phrase => self
                .phrase
                .draw_selection(grid, super::layout::PHRASE, cells),
            Focus::RightSlot => {}
        }
    }
}
