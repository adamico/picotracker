//! The Chain pane: the entries of the Chain under the Song cursor, and the Chain row strip.

use super::{
    EMPTY, FILL, HEADING, VALUE, draw_out_of_range_flag, draw_row_label, draw_styled_value,
    draw_title, id_text, nudged_id, stepped, typed_value,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::project::{CHAIN_ENTRIES, Chain, ChainEntry, ChainId, PhraseId, Project, TRACKS};
use crate::screen::grid::{Grid, Style};

const COARSE_TRANSPOSE: i32 = 12;
/// Rows above the first Chain entry: the title and the column headings.
const HEADER_ROWS: usize = 2;
const STRIP_TITLE_ROW: usize = 19;
const STRIP_HEADING_ROW: usize = 20;
const STRIP_FIRST_ROW: usize = 21;
const STRIP_FIRST_TRACK_COLUMN: usize = 2;
const STRIP_TRACK_WIDTH: usize = 3;
const STRIP_TICKS_COLUMN: usize = 27;

/// The cells of a Chain entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryCell {
    Phrase,
    Transpose,
    Fill,
}

/// The cells of a Chain entry: which each is, where it starts and what an empty entry shows.
const COLUMNS: [(EntryCell, usize, &str); 3] = [
    (EntryCell::Phrase, 2, "--"),
    (EntryCell::Transpose, 5, "---"),
    (EntryCell::Fill, 9, "----"),
];

/// What the Chain pane shows.
#[derive(Clone, Copy, Debug)]
pub struct ChainView {
    pub song_row: usize,
    pub track: usize,
    /// The entry the Playhead is on, when it is on this Song row.
    pub playing_entry: Option<usize>,
}

/// The Chain pane's cursor.
#[derive(Debug, Default)]
pub struct ChainPane {
    entry: usize,
    column: usize,
    /// The digit typed just before this one into the same cell.
    typed_digit: Option<u8>,
    /// Whether the transpose being typed is below zero.
    typing_downwards: bool,
}

/// Where the Chain pane's cursor is.
#[derive(Clone, Copy, Debug)]
pub struct ChainSpot {
    entry: usize,
    column: usize,
}

impl ChainPane {
    /// Where the cursor is now.
    pub fn spot(&self) -> ChainSpot {
        ChainSpot {
            entry: self.entry,
            column: self.column,
        }
    }

    /// Puts the cursor back where it was.
    pub fn return_to(&mut self, spot: ChainSpot) {
        self.entry = spot.entry;
        self.column = spot.column;
    }

    /// Puts the cursor on a Chain entry.
    pub fn jump_to(&mut self, entry: usize) {
        self.entry = entry.min(CHAIN_ENTRIES - 1);
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.delta();
        self.column = stepped(self.column, columns, COLUMNS.len());
        self.entry = stepped(self.entry, rows, CHAIN_ENTRIES);
    }

    /// The entry under the cursor.
    pub fn entry(&self) -> usize {
        self.entry
    }

    fn cell(&self) -> EntryCell {
        COLUMNS[self.column].0
    }

    /// Whether the cell under the cursor can be edited in an entry holding no Phrase.
    pub fn edits_empty_entries(&self) -> bool {
        self.cell() == EntryCell::Phrase
    }

    /// Whether hexadecimal digits mean anything in the cell under the cursor.
    pub fn takes_digits(&self) -> bool {
        self.cell() != EntryCell::Fill
    }

    /// Ends the value being typed, so that the next digit starts a new one.
    pub fn finish_typing(&mut self) {
        self.typed_digit = None;
    }

    /// Types a hexadecimal digit into the cell under the cursor.
    pub fn type_digit(&mut self, chain: &mut Chain, digit: u8) {
        let first_digit = self.typed_digit.is_none();
        let value = typed_value(&mut self.typed_digit, digit);
        let slot = chain.entry_mut(self.entry);
        match (self.cell(), slot.as_mut()) {
            (EntryCell::Phrase, _) => set_phrase(slot, PhraseId::new(value)),
            (EntryCell::Transpose, Some(entry)) => {
                if first_digit {
                    self.typing_downwards = entry.transpose() < 0;
                }
                let sign = if self.typing_downwards { -1 } else { 1 };
                entry.set_transpose(sign * i32::from(value));
            }
            _ => {}
        }
    }

    /// Raises or lowers the cell under the cursor. Nudging a Fill mark flips it.
    pub fn nudge(&mut self, chain: &mut Chain, up: bool, coarse: bool) {
        let slot = chain.entry_mut(self.entry);
        match (self.cell(), slot.as_mut()) {
            (EntryCell::Phrase, entry) => {
                let number = entry.map(|entry| entry.phrase().number());
                set_phrase(slot, PhraseId::new(nudged_id(number, up, coarse)));
            }
            (EntryCell::Transpose, Some(entry)) => {
                let amount = if coarse { COARSE_TRANSPOSE } else { 1 };
                let delta = if up { amount } else { -amount };
                entry.set_transpose(i32::from(entry.transpose()) + delta);
            }
            (EntryCell::Fill, Some(entry)) => entry.set_fill(!entry.fill()),
            _ => {}
        }
    }

    /// Clears the cell under the cursor. Clearing the Phrase empties the whole entry.
    pub fn clear(&mut self, chain: &mut Chain) {
        let slot = chain.entry_mut(self.entry);
        match (self.cell(), slot.as_mut()) {
            (EntryCell::Phrase, _) => *slot = None,
            (EntryCell::Transpose, Some(entry)) => entry.set_transpose(0),
            (EntryCell::Fill, Some(entry)) => entry.set_fill(false),
            _ => {}
        }
    }

    /// Flips the Fill mark of the entry under the cursor.
    pub fn toggle_fill(&mut self, chain: &mut Chain) {
        if let Some(entry) = chain.entry_mut(self.entry) {
            entry.set_fill(!entry.fill());
        }
    }

    /// Moves the cursor to the entry the Playhead is on.
    pub fn follow(&mut self, entry: usize) {
        if self.entry != entry {
            self.entry = entry;
            self.finish_typing();
        }
    }

    /// Draws the pane for the Song row and Track under the Song cursor.
    pub fn draw(
        &self,
        grid: &mut Grid,
        area: Area,
        focused: bool,
        project: &Project,
        view: ChainView,
    ) {
        let ChainView {
            song_row,
            track,
            playing_entry,
        } = view;
        let id = project.song_cell(song_row, track);
        let number = id_text(id.map(ChainId::number));
        let title = format!("CHAIN {number}  T{} ROW {song_row:02X}", track + 1);
        draw_title(grid, area.column, 0, &title, focused);
        grid.put(area.column, 1, "K PH TSP FILL", HEADING);

        for index in 0..CHAIN_ENTRIES {
            let row = HEADER_ROWS + index;
            let label = format!("{index:X}");
            draw_row_label(grid, area.column, row, &label, playing_entry == Some(index));
            let entry = project.chain_row_entry(song_row, track, index);
            for (column, (cell, offset, empty)) in COLUMNS.iter().enumerate() {
                let is_cursor = focused && index == self.entry && column == self.column;
                let (text, style) = match entry {
                    Some(entry) => cell_text(*cell, entry),
                    None => ((*empty).to_owned(), EMPTY),
                };
                draw_styled_value(grid, area.column + offset, row, &text, is_cursor, style);
            }
            let out_of_range = entry.is_some_and(|entry| {
                project
                    .phrase(entry.phrase())
                    .is_out_of_range(entry.transpose())
            });
            if out_of_range {
                draw_out_of_range_flag(grid, area.column, row);
            }
        }

        draw_chain_row_strip(grid, area, project, song_row, playing_entry);
    }
}

/// Puts a Phrase in an entry, keeping the transpose and Fill mark of one already there.
fn set_phrase(slot: &mut Option<ChainEntry>, phrase: PhraseId) {
    match slot {
        Some(entry) => entry.set_phrase(phrase),
        None => *slot = Some(ChainEntry::new(phrase)),
    }
}

/// The text and style of one cell of a filled entry.
fn cell_text(cell: EntryCell, entry: ChainEntry) -> (String, Style) {
    match cell {
        EntryCell::Phrase => (format!("{:02X}", entry.phrase().number()), VALUE),
        EntryCell::Transpose => {
            let transpose = i16::from(entry.transpose());
            let sign = if transpose < 0 { '-' } else { '+' };
            (format!("{sign}{:02X}", transpose.abs()), VALUE)
        }
        EntryCell::Fill if entry.fill() => ("FILL".to_owned(), FILL),
        EntryCell::Fill => ("----".to_owned(), EMPTY),
    }
}

/// Draws the k-th entry of all 8 Tracks on this Song row, and how long each Chain row lasts.
fn draw_chain_row_strip(
    grid: &mut Grid,
    area: Area,
    project: &Project,
    song_row: usize,
    playing_entry: Option<usize>,
) {
    draw_title(grid, area.column, STRIP_TITLE_ROW, "CHAIN ROWS", false);
    let heading = "K T1 T2 T3 T4 T5 T6 T7 T8  TICK";
    grid.put(area.column, STRIP_HEADING_ROW, heading, HEADING);

    for k in 0..CHAIN_ENTRIES {
        let row = STRIP_FIRST_ROW + k;
        draw_row_label(
            grid,
            area.column,
            row,
            &format!("{k:X}"),
            playing_entry == Some(k),
        );
        for track in 0..TRACKS {
            let column = area.column + STRIP_FIRST_TRACK_COLUMN + track * STRIP_TRACK_WIDTH;
            draw_strip_entry(
                grid,
                column,
                row,
                project.chain_row_entry(song_row, track, k),
            );
        }
        let ticks = project.chain_row_ticks(song_row, k);
        let (text, style) = if ticks > 0 {
            (format!("{ticks:>4}"), VALUE)
        } else {
            ("   -".to_owned(), EMPTY)
        };
        grid.put(area.column + STRIP_TICKS_COLUMN, row, &text, style);
    }
}

/// Draws one Track's entry in the strip, with an `F` after a Phrase marked Fill.
fn draw_strip_entry(grid: &mut Grid, column: usize, row: usize, entry: Option<ChainEntry>) {
    let Some(entry) = entry else {
        grid.put(column, row, "--", EMPTY);
        return;
    };
    let number = format!("{:02X}", entry.phrase().number());
    if entry.fill() {
        grid.put(column, row, &number, FILL);
        grid.put(column + number.len(), row, "F", FILL);
    } else {
        grid.put(column, row, &number, VALUE);
    }
}
