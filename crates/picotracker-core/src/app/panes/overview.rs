//! Overview, the default view of the right-hand slot: the Export Tracks and the scope.

use super::{HEADING, draw_title, draw_value, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::screen::grid::Grid;

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

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool) {
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
    }
}
