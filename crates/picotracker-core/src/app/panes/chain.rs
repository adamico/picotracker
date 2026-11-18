//! The Chain pane: the entries of the Chain under the Song cursor, and the Chain row strip.

use super::{EMPTY, HEADING, draw_title, draw_value, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::screen::grid::Grid;

const ENTRIES: usize = 16;
/// Rows above the first Chain entry: the title and the column headings.
const HEADER_ROWS: usize = 2;
const STRIP_TITLE_ROW: usize = 19;
const STRIP_HEADING_ROW: usize = 20;
const STRIP_FIRST_ROW: usize = 21;
const STRIP_TRACKS: usize = 8;
const STRIP_FIRST_TRACK_COLUMN: usize = 2;
const STRIP_TRACK_WIDTH: usize = 3;
const STRIP_TICKS_COLUMN: usize = 27;

/// The columns of a Chain entry: where each starts and what an empty entry shows.
const COLUMNS: [(usize, &str); 3] = [(2, "--"), (5, "---"), (9, "----")];

/// The Chain pane's cursor.
#[derive(Debug, Default)]
pub struct ChainPane {
    entry: usize,
    column: usize,
}

impl ChainPane {
    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.delta();
        self.column = stepped(self.column, columns, COLUMNS.len());
        self.entry = stepped(self.entry, rows, ENTRIES);
    }

    /// Draws the pane for the Song row and Track under the Song cursor.
    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool, song_row: usize, track: usize) {
        let title = format!("CHAIN --  T{} ROW {song_row:02X}", track + 1);
        draw_title(grid, area.column, 0, &title, focused);
        grid.put(area.column, 1, "K PH TSP FILL", HEADING);

        for entry in 0..ENTRIES {
            let row = HEADER_ROWS + entry;
            grid.put(area.column, row, &format!("{entry:X}"), HEADING);
            for (index, (offset, empty)) in COLUMNS.iter().enumerate() {
                let is_cursor = focused && entry == self.entry && index == self.column;
                draw_value(grid, area.column + offset, row, empty, is_cursor);
            }
        }

        draw_chain_row_strip(grid, area);
    }
}

fn draw_chain_row_strip(grid: &mut Grid, area: Area) {
    draw_title(grid, area.column, STRIP_TITLE_ROW, "CHAIN ROWS", false);
    let heading = "K T1 T2 T3 T4 T5 T6 T7 T8  TICK";
    grid.put(area.column, STRIP_HEADING_ROW, heading, HEADING);

    for entry in 0..ENTRIES {
        let row = STRIP_FIRST_ROW + entry;
        grid.put(area.column, row, &format!("{entry:X}"), HEADING);
        for track in 0..STRIP_TRACKS {
            let offset = STRIP_FIRST_TRACK_COLUMN + track * STRIP_TRACK_WIDTH;
            grid.put(area.column + offset, row, "--", EMPTY);
        }
        grid.put(area.column + STRIP_TICKS_COLUMN, row, "   -", EMPTY);
    }
}
