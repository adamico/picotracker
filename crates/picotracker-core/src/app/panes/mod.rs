//! The panes on the grid. Each pane owns its cursor and draws itself.

pub mod cart_view;
pub mod chain;
pub mod file_list;
pub mod instrument;
pub mod log;
pub mod midi_dialog;
pub mod overview;
pub mod phrase;
pub mod song;
pub mod song_picker;

use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const TITLE: Style = Style::fg(Colour::Blue);
const FOCUSED_TITLE: Style = Style::fg(Colour::Yellow);
const HEADING: Style = Style::fg(Colour::Lavender);
const EMPTY: Style = Style::fg(Colour::DarkGrey);
const VALUE: Style = Style::fg(Colour::White);
const CURSOR: Style = Style {
    fg: Colour::Black,
    bg: Colour::Blue,
};
const CURSOR_BRACKET: Style = Style::fg(Colour::Yellow);
const SELECTED: Style = Style {
    fg: Colour::Black,
    bg: Colour::Lavender,
};
const FILL: Style = Style::fg(Colour::Pink);
const OUT_OF_RANGE: Style = Style::fg(Colour::Red);
const PLAYHEAD: Style = Style::fg(Colour::Green);
const OUT_OF_RANGE_COLUMN: usize = 14;
const COARSE_ID: i32 = 16;
const MAX_ID: i32 = 255;

/// The value of a two-digit cell after one more digit: the digit before it becomes the high digit.
fn typed_value(first_digit: &mut Option<u8>, digit: u8) -> u8 {
    let value = first_digit.map_or(0, |high| high << 4) | digit;
    *first_digit = Some(digit);
    value
}

/// A Chain or Phrase number after a nudge, starting from 00 on an empty cell.
fn nudged_id(number: Option<u8>, up: bool, coarse: bool) -> u8 {
    let amount = if coarse { COARSE_ID } else { 1 };
    let delta = if up { amount } else { -amount };
    number.map_or(0, |number| {
        (i32::from(number) + delta).clamp(0, MAX_ID) as u8
    })
}

/// The text of a Chain or Phrase number, or of an empty cell.
fn id_text(number: Option<u8>) -> String {
    number.map_or_else(|| "--".to_owned(), |number| format!("{number:02X}"))
}

/// Moves a cursor position by a delta, stopping at the first and last position.
fn stepped(position: usize, delta: isize, positions: usize) -> usize {
    position
        .saturating_add_signed(delta)
        .min(positions.saturating_sub(1))
}

/// Flags a row whose notes a transpose pushes outside PICO-8's pitch range.
fn draw_out_of_range_flag(grid: &mut Grid, column: usize, row: usize) {
    grid.put(column + OUT_OF_RANGE_COLUMN, row, "!", OUT_OF_RANGE);
}

/// Draws a row's label. The row the Playhead is on shows a marker over the label's first character.
fn draw_row_label(grid: &mut Grid, column: usize, row: usize, label: &str, playing: bool) {
    grid.put(column, row, label, HEADING);
    if playing {
        grid.put(column, row, ">", PLAYHEAD);
    }
}

/// Draws a pane's title, brighter when the pane has focus.
fn draw_title(grid: &mut Grid, column: usize, row: usize, title: &str, focused: bool) {
    let style = if focused { FOCUSED_TITLE } else { TITLE };
    grid.put(column, row, title, style);
}

/// Draws one editable value in a style of its own when the cursor is not on it. The cursor is
/// drawn in brackets, so it shows in a text dump too.
fn draw_styled_value(
    grid: &mut Grid,
    column: usize,
    row: usize,
    text: &str,
    is_cursor: bool,
    style: Style,
) {
    if is_cursor {
        grid.put(column - 1, row, "[", CURSOR_BRACKET);
        grid.put(column, row, text, CURSOR);
        grid.put(column + text.chars().count(), row, "]", CURSOR_BRACKET);
    } else {
        grid.put(column, row, text, style);
    }
}
