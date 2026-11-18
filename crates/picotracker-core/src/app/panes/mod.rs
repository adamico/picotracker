//! The panes on the grid. Each pane owns its cursor and draws itself.

pub mod chain;
pub mod overview;
pub mod phrase;
pub mod song;

use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const TITLE: Style = Style::fg(Colour::Blue);
const FOCUSED_TITLE: Style = Style::fg(Colour::Yellow);
const HEADING: Style = Style::fg(Colour::Lavender);
const EMPTY: Style = Style::fg(Colour::DarkGrey);
const CURSOR: Style = Style {
    fg: Colour::Black,
    bg: Colour::Blue,
};
const CURSOR_BRACKET: Style = Style::fg(Colour::Yellow);

/// Moves a cursor position by a delta, stopping at the first and last position.
fn stepped(position: usize, delta: isize, positions: usize) -> usize {
    position
        .saturating_add_signed(delta)
        .min(positions.saturating_sub(1))
}

/// Draws a pane's title, brighter when the pane has focus.
fn draw_title(grid: &mut Grid, column: usize, row: usize, title: &str, focused: bool) {
    let style = if focused { FOCUSED_TITLE } else { TITLE };
    grid.put(column, row, title, style);
}

/// Draws one editable value. The cursor is drawn in brackets, so it shows in a text dump too.
fn draw_value(grid: &mut Grid, column: usize, row: usize, text: &str, is_cursor: bool) {
    if is_cursor {
        grid.put(column - 1, row, "[", CURSOR_BRACKET);
        grid.put(column, row, text, CURSOR);
        grid.put(column + text.chars().count(), row, "]", CURSOR_BRACKET);
    } else {
        grid.put(column, row, text, EMPTY);
    }
}
