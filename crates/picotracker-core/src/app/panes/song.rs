//! The Song pane: rows of Chains, one column per Track.

use super::{HEADING, draw_title, draw_value, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::screen::grid::Grid;

const TRACKS: usize = 8;
/// Song rows the cursor can reach, as many as a two-digit hex row label can name.
const SONG_ROWS: usize = 256;
/// Rows above the first Song row: the title and the Track headers.
const HEADER_ROWS: usize = 2;
const FIRST_TRACK_COLUMN: usize = 4;
const TRACK_WIDTH: usize = 3;

/// The Song pane's cursor.
#[derive(Debug, Default)]
pub struct SongPane {
    row: usize,
    track: usize,
}

impl SongPane {
    /// The Song row under the cursor.
    pub fn row(&self) -> usize {
        self.row
    }

    /// The Track under the cursor, counting from 0.
    pub fn track(&self) -> usize {
        self.track
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.delta();
        self.track = stepped(self.track, columns, TRACKS);
        self.row = stepped(self.row, rows, SONG_ROWS);
    }

    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool) {
        draw_title(grid, area.column, 0, "SONG", focused);
        for track in 0..TRACKS {
            let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
            grid.put(column, 1, &format!("T{}", track + 1), HEADING);
        }

        let visible = area.rows - HEADER_ROWS;
        let top = self
            .row
            .saturating_sub(visible / 2)
            .min(SONG_ROWS - visible);
        for offset in 0..visible {
            let row = HEADER_ROWS + offset;
            let song_row = top + offset;
            grid.put(area.column, row, &format!("{song_row:02X}"), HEADING);
            for track in 0..TRACKS {
                let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
                let is_cursor = focused && song_row == self.row && track == self.track;
                draw_value(grid, column, row, "--", is_cursor);
            }
        }
    }
}
