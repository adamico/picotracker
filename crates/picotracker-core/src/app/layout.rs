//! Where each pane sits on the grid.

use crate::screen::grid::{GRID_ROWS, Grid, Style};
use crate::screen::palette::Colour;

/// A pane's position on the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub column: usize,
    pub rows: usize,
}

pub const STATUS_ROW: usize = GRID_ROWS - 1;

const PANE_ROWS: usize = STATUS_ROW;

pub const SONG: Area = Area {
    column: 0,
    rows: PANE_ROWS,
};
pub const CHAIN: Area = Area {
    column: 29,
    rows: PANE_ROWS,
};
pub const PHRASE: Area = Area {
    column: 63,
    rows: PANE_ROWS,
};
pub const RIGHT_SLOT: Area = Area {
    column: 83,
    rows: PANE_ROWS,
};

const SEPARATOR_COLUMNS: [usize; 3] = [28, 62, 82];

/// Draws the lines between the panes.
pub fn draw_separators(grid: &mut Grid) {
    for column in SEPARATOR_COLUMNS {
        for row in 0..PANE_ROWS {
            grid.put(column, row, "│", Style::fg(Colour::DarkBlue));
        }
    }
}
