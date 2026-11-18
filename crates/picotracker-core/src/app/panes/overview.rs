//! Overview, the default view of the right-hand slot: the Export Tracks and the scope.

use super::{HEADING, draw_title, draw_value, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::player::{SCOPE_COLUMNS, SCOPE_RANGE};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const TRACE: Style = Style::fg(Colour::Green);

const CHANNELS: usize = 4;
const FIRST_CHANNEL_ROW: usize = 2;
const CHANNEL_VALUE_COLUMN: usize = 5;
const SCOPE_ROW: usize = 7;

/// The Overview's cursor.
#[derive(Debug, Default)]
pub struct OverviewPane {
    channel: usize,
}

impl OverviewPane {
    pub fn move_cursor(&mut self, direction: Direction) {
        let (_, rows) = direction.delta();
        self.channel = stepped(self.channel, rows, CHANNELS);
    }

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool, scope: &[i8; SCOPE_COLUMNS]) {
        draw_title(grid, area.column, 0, "OVERVIEW", focused);
        grid.put(area.column, 1, "EXPORT TRACKS", HEADING);

        for channel in 0..CHANNELS {
            let row = FIRST_CHANNEL_ROW + channel;
            grid.put(area.column, row, &format!("CH{channel}"), HEADING);
            let is_cursor = focused && channel == self.channel;
            draw_value(
                grid,
                area.column + CHANNEL_VALUE_COLUMN,
                row,
                "--",
                is_cursor,
            );
        }

        draw_title(grid, area.column, SCOPE_ROW, "SCOPE", false);
        draw_scope(grid, area, scope);
    }
}

/// Draws the mix as one waveform filling the rows below the title. Silence draws nothing.
fn draw_scope(grid: &mut Grid, area: Area, scope: &[i8; SCOPE_COLUMNS]) {
    if scope.iter().all(|value| *value == 0) {
        return;
    }
    let first_row = SCOPE_ROW + 1;
    let half_height = (area.rows - first_row - 1) as i32 / 2;
    let middle = first_row as i32 + half_height;
    for (column, value) in scope.iter().enumerate() {
        let offset = i32::from(*value) * half_height / i32::from(SCOPE_RANGE);
        grid.put(area.column + column, (middle - offset) as usize, "*", TRACE);
    }
}
