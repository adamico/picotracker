//! The Phrase pane: the header and Steps of the Phrase under the Chain cursor.

use super::{EMPTY, HEADING, draw_title, draw_value, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::screen::grid::Grid;

/// The Steps shown when no Phrase is under the Chain cursor: a new Phrase's default length.
const STEPS: usize = 16;
/// Rows above the first Step: the title, three header rows and the column headings.
const HEADER_ROWS: usize = 5;

/// The columns of a Step: where each starts and what an empty Step shows.
const COLUMNS: [(usize, &str); 4] = [(3, "---"), (7, "--"), (10, "-"), (12, "-")];

/// The Phrase pane's cursor.
#[derive(Debug, Default)]
pub struct PhrasePane {
    step: usize,
    column: usize,
}

impl PhrasePane {
    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.delta();
        self.column = stepped(self.column, columns, COLUMNS.len());
        self.step = stepped(self.step, rows, STEPS);
    }

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool) {
        draw_title(grid, area.column, 0, "PHRASE --", focused);
        grid.put(area.column, 1, "SPD -- BPM ---", EMPTY);
        grid.put(area.column, 2, "LEN -- NZ- BZ-", EMPTY);
        grid.put(area.column, 3, "DT- RV- DP-", EMPTY);
        grid.put(area.column, 4, "   NOT IN V F", HEADING);

        for step in 0..STEPS {
            let row = HEADER_ROWS + step;
            grid.put(area.column, row, &format!("{step:02X}"), HEADING);
            for (index, (offset, empty)) in COLUMNS.iter().enumerate() {
                let is_cursor = focused && step == self.step && index == self.column;
                draw_value(grid, area.column + offset, row, empty, is_cursor);
            }
        }
    }
}
