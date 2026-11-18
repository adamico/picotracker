//! The Song pane: rows of Chains, one column per Track.

use super::{
    EMPTY, HEADING, SELECTED, VALUE, draw_row_label, draw_styled_value, draw_title, id_text,
    nudged_id, stepped, typed_value,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::app::selection::{CellPosition, CellRange};
use crate::player::TrackMix;
use crate::project::{ChainId, Project, SONG_ROWS, TRACKS};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const SOLOED: Style = Style::fg(Colour::Yellow);
/// The PICO-8 channel an Exported Track goes to, right after its header.
const EXPORT_CHANNEL: Style = Style::fg(Colour::Green);

/// Rows above the first Song row: the title and the Track headers.
const HEADER_ROWS: usize = 2;
const FIRST_TRACK_COLUMN: usize = 4;
const TRACK_WIDTH: usize = 3;

/// The letter and style of a Track's header. A Track both muted and soloed is heard, so it shows
/// as soloed.
fn track_header(track_mix: &TrackMix, track: usize) -> (char, Style) {
    if track_mix.is_soloed(track) {
        ('S', SOLOED)
    } else if track_mix.is_muted(track) {
        ('M', EMPTY)
    } else {
        ('T', HEADING)
    }
}

/// The Song pane's cursor.
#[derive(Debug, Default)]
pub struct SongPane {
    row: usize,
    track: usize,
    /// The digit typed just before this one into the same cell.
    typed_digit: Option<u8>,
}

/// Where the Song pane's cursor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongSpot {
    row: usize,
    track: usize,
}

impl SongPane {
    /// Where the cursor is now.
    pub fn spot(&self) -> SongSpot {
        SongSpot {
            row: self.row,
            track: self.track,
        }
    }

    /// Puts the cursor back where it was.
    pub fn return_to(&mut self, spot: SongSpot) {
        self.row = spot.row;
        self.track = spot.track;
    }

    /// The Song row under the cursor.
    pub fn row(&self) -> usize {
        self.row
    }

    /// The Track under the cursor, counting from 0.
    pub fn track(&self) -> usize {
        self.track
    }

    /// Puts the cursor on a Song row and a Track.
    pub fn jump_to(&mut self, row: usize, track: usize) {
        self.row = row.min(SONG_ROWS - 1);
        self.track = track.min(TRACKS - 1);
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.delta();
        self.track = stepped(self.track, columns, TRACKS);
        self.row = stepped(self.row, rows, SONG_ROWS);
    }

    /// Moves the cursor to the Song row the Playhead is on.
    pub fn follow(&mut self, row: usize) {
        if self.row != row {
            self.row = row;
            self.finish_typing();
        }
    }

    /// Ends the Chain number being typed, so that the next digit starts a new one.
    pub fn finish_typing(&mut self) {
        self.typed_digit = None;
    }

    /// Types a hexadecimal digit of the Chain under the cursor.
    pub fn type_digit(&mut self, project: &mut Project, digit: u8) {
        let number = typed_value(&mut self.typed_digit, digit);
        self.set_chain(project, Some(ChainId::new(number)));
    }

    /// Steps the cell under the cursor through the Chains.
    pub fn nudge(&mut self, project: &mut Project, up: bool, coarse: bool) {
        let number = project.song_cell(self.row, self.track).map(ChainId::number);
        self.set_chain(project, Some(ChainId::new(nudged_id(number, up, coarse))));
    }

    /// Takes the Chain off the cell under the cursor. The Chain itself is kept.
    pub fn clear(&mut self, project: &mut Project) {
        self.set_chain(project, None);
    }

    fn set_chain(&self, project: &mut Project, chain: Option<ChainId>) {
        project.set_song_cell(self.row, self.track, chain);
    }

    /// The cell under the cursor: a Track and a Song row.
    pub fn cell(&self) -> CellPosition {
        CellPosition {
            column: self.track,
            row: self.row,
        }
    }

    /// Steps every Chain in a cells. Empty cells stay empty.
    pub fn nudge_cells(&self, project: &mut Project, cells: CellRange, up: bool, coarse: bool) {
        for row in cells.rows() {
            for track in cells.columns() {
                if let Some(chain) = project.song_cell(row, track) {
                    let number = nudged_id(Some(chain.number()), up, coarse);
                    project.set_song_cell(row, track, Some(ChainId::new(number)));
                }
            }
        }
    }

    /// Takes the Chains off a cells of cells. The Chains themselves are kept.
    pub fn clear_cells(&self, project: &mut Project, cells: CellRange) {
        for row in cells.rows() {
            for track in cells.columns() {
                project.set_song_cell(row, track, None);
            }
        }
    }

    /// The Chains in a cells of cells, row by row.
    pub fn copy(&self, project: &Project, cells: CellRange) -> Vec<Vec<Option<ChainId>>> {
        cells
            .rows()
            .map(|row| {
                cells
                    .columns()
                    .map(|track| project.song_cell(row, track))
                    .collect()
            })
            .collect()
    }

    /// Overwrites cells from the cursor, and says how many rows lost cells past the Song's edges.
    pub fn paste(&self, project: &mut Project, rows: &[Vec<Option<ChainId>>]) -> usize {
        let fitting_rows = rows.len().min(SONG_ROWS - self.row);
        for (row, cells) in (self.row..).zip(&rows[..fitting_rows]) {
            for (track, chain) in (self.track..TRACKS).zip(cells) {
                project.set_song_cell(row, track, *chain);
            }
        }
        let width = rows.first().map_or(0, Vec::len);
        if self.track + width > TRACKS {
            rows.len()
        } else {
            rows.len() - fitting_rows
        }
    }

    /// The Song row drawn at the top of the pane.
    fn top_row(&self, visible: usize) -> usize {
        self.row
            .saturating_sub(visible / 2)
            .min(SONG_ROWS - visible)
    }

    /// Marks the selected cells, leaving the cursor as it is.
    pub fn draw_selection(&self, grid: &mut Grid, area: Area, cells: CellRange) {
        let visible = area.rows - HEADER_ROWS;
        let top = self.top_row(visible);
        for song_row in cells
            .rows()
            .filter(|row| (top..top + visible).contains(row))
        {
            for track in cells.columns() {
                if (song_row, track) != (self.row, self.track) {
                    let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
                    grid.restyle(column, HEADER_ROWS + song_row - top, 2, SELECTED);
                }
            }
        }
    }

    /// Makes the row under the cursor the Song loop row, or turns the loop off if it already is.
    pub fn toggle_loop_row(&self, project: &mut Project) {
        let row = (project.song_loop_row() != Some(self.row)).then_some(self.row);
        project.set_song_loop_row(row);
    }

    pub fn draw(
        &self,
        grid: &mut Grid,
        area: Area,
        focused: bool,
        project: &Project,
        track_mix: &TrackMix,
        playing_row: Option<usize>,
    ) {
        let loop_row = project
            .song_loop_row()
            .map_or_else(|| "--".to_owned(), |row| format!("{row:02X}"));
        let title = format!("SONG  LOOP {loop_row}");
        draw_title(grid, area.column, 0, &title, focused);
        let export_tracks = project.export_tracks();
        for track in 0..TRACKS {
            let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
            let (letter, style) = track_header(track_mix, track);
            grid.put(column, 1, &format!("{letter}{}", track + 1), style);
            let number = track as u8 + 1;
            let channel = export_tracks
                .iter()
                .position(|exported| *exported == Some(number));
            if let Some(channel) = channel {
                grid.put(column + 2, 1, &channel.to_string(), EXPORT_CHANNEL);
            }
        }

        let visible = area.rows - HEADER_ROWS;
        let top = self.top_row(visible);
        for offset in 0..visible {
            let row = HEADER_ROWS + offset;
            let song_row = top + offset;
            let label = format!("{song_row:02X}");
            draw_row_label(
                grid,
                area.column,
                row,
                &label,
                playing_row == Some(song_row),
            );
            for track in 0..TRACKS {
                let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
                let is_cursor = focused && song_row == self.row && track == self.track;
                let chain = project.song_cell(song_row, track).map(ChainId::number);
                let style = if chain.is_some() { VALUE } else { EMPTY };
                draw_styled_value(grid, column, row, &id_text(chain), is_cursor, style);
            }
        }
    }
}
