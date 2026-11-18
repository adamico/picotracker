//! The status line on the bottom row of the grid.

use super::layout::STATUS_ROW;
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const BAR: Style = Style {
    fg: Colour::LightGrey,
    bg: Colour::DarkBlue,
};

/// Draws the status line.
pub fn draw(grid: &mut Grid) {
    grid.fill_row(STATUS_ROW, BAR);
    grid.put(0, STATUS_ROW, "READY", BAR);
}
