//! The Song pane: rows of Chains, one column per Track.

use super::{
    EMPTY, HEADING, VALUE, draw_styled_value, draw_title, id_text, nudged_id, stepped, typed_value,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::player::TrackMix;
use crate::project::{ChainId, Project, SONG_ROWS, TRACKS};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const SOLOED: Style = Style::fg(Colour::Yellow);

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
    ) {
        let loop_row = project
            .song_loop_row()
            .map_or_else(|| "--".to_owned(), |row| format!("{row:02X}"));
        let title = format!("SONG  LOOP {loop_row}");
        draw_title(grid, area.column, 0, &title, focused);
        for track in 0..TRACKS {
            let column = area.column + FIRST_TRACK_COLUMN + track * TRACK_WIDTH;
            let (letter, style) = track_header(track_mix, track);
            grid.put(column, 1, &format!("{letter}{}", track + 1), style);
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
                let chain = project.song_cell(song_row, track).map(ChainId::number);
                let style = if chain.is_some() { VALUE } else { EMPTY };
                draw_styled_value(grid, column, row, &id_text(chain), is_cursor, style);
            }
        }
    }
}
