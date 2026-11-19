//! The status line on the bottom row of the grid.

use super::layout::STATUS_ROW;
use super::messages::{Look, Notice};
use crate::screen::grid::{GRID_COLUMNS, Grid, Style};
use crate::screen::palette::Colour;

const BAR: Style = Style {
    fg: Colour::LightGrey,
    bg: Colour::DarkBlue,
};
const INFO: Style = Style {
    fg: Colour::Black,
    bg: Colour::Green,
};
const FADING_INFO: Style = Style {
    fg: Colour::LightGrey,
    bg: Colour::DarkGreen,
};
const WARNING: Style = Style {
    fg: Colour::Black,
    bg: Colour::Orange,
};
const FAILURE: Style = Style {
    fg: Colour::White,
    bg: Colour::Red,
};
const QUESTION: Style = Style {
    fg: Colour::White,
    bg: Colour::Blue,
};

const KEY_HINTS: &str = "^N NEW  ^O OPEN  ^I IMPORT  ^S SAVE  ^L LOG  SPACE PLAY";

/// What the status line shows.
pub enum StatusLine<'a> {
    /// A question waiting for Enter or Esc.
    Question(&'a str),
    /// A result of something the user asked for.
    Notice(&'a Notice),
    /// A brief remark about the last action.
    Remark(&'a str),
    /// The keys of the file list.
    Hints(&'a str),
    /// Nothing to say: the usual keys, and the Project's name.
    Idle { project: &'a str, unsaved: bool },
}

/// Draws the status line.
pub fn draw(grid: &mut Grid, status: &StatusLine<'_>) {
    match status {
        StatusLine::Question(text) => draw_bar(grid, text, QUESTION),
        StatusLine::Notice(notice) => draw_bar(grid, notice.text(), notice_style(notice)),
        StatusLine::Remark(text) | StatusLine::Hints(text) => draw_bar(grid, text, BAR),
        StatusLine::Idle { project, unsaved } => {
            draw_bar(grid, KEY_HINTS, BAR);
            let name = format!("{project}{}", if *unsaved { "*" } else { "" });
            let column = GRID_COLUMNS.saturating_sub(name.chars().count());
            grid.put(column, STATUS_ROW, &name, BAR);
        }
    }
}

fn draw_bar(grid: &mut Grid, text: &str, style: Style) {
    grid.fill_row(STATUS_ROW, style);
    grid.put(0, STATUS_ROW, text, style);
}

fn notice_style(notice: &Notice) -> Style {
    match notice.look() {
        Look::Info => INFO,
        Look::FadingInfo => FADING_INFO,
        Look::Warning => WARNING,
        Look::Failure => FAILURE,
    }
}
