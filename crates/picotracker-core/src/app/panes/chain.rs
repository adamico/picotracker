//! The Chain pane: the entries of the Chain under the Song cursor, and the Chain row strip.

use super::{
    EMPTY, FILL, HEADING, VALUE, draw_styled_value, draw_title, id_text, nudged_id, stepped,
    typed_value,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::project::{CHAIN_ENTRIES, Chain, ChainEntry, ChainId, PhraseId};
use crate::screen::grid::{Grid, Style};

const COARSE_TRANSPOSE: i32 = 12;
/// Rows above the first Chain entry: the title and the column headings.
const HEADER_ROWS: usize = 2;
const STRIP_TITLE_ROW: usize = 19;
const STRIP_HEADING_ROW: usize = 20;
const STRIP_FIRST_ROW: usize = 21;
const STRIP_TRACKS: usize = 8;
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

impl ChainPane {
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

    /// Draws the pane for the Song row and Track under the Song cursor.
    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool, shown: ShownChain<'_>) {
        let number = id_text(shown.id.map(ChainId::number));
        let title = format!("CHAIN {number}  T{} ROW {:02X}", shown.track + 1, shown.row);
        draw_title(grid, area.column, 0, &title, focused);
        grid.put(area.column, 1, "K PH TSP FILL", HEADING);

        for index in 0..CHAIN_ENTRIES {
            let row = HEADER_ROWS + index;
            grid.put(area.column, row, &format!("{index:X}"), HEADING);
            let entry = shown.chain.and_then(|chain| chain.entry(index));
            for (column, (cell, offset, empty)) in COLUMNS.iter().enumerate() {
                let is_cursor = focused && index == self.entry && column == self.column;
                let (text, style) = match entry {
                    Some(entry) => cell_text(*cell, entry),
                    None => ((*empty).to_owned(), EMPTY),
                };
                draw_styled_value(grid, area.column + offset, row, &text, is_cursor, style);
            }
        }

        draw_chain_row_strip(grid, area);
    }
}

/// What the Chain pane shows: the Chain under the Song cursor, if any.
#[derive(Clone, Copy, Debug)]
pub struct ShownChain<'a> {
    pub id: Option<ChainId>,
    pub chain: Option<&'a Chain>,
    pub row: usize,
    pub track: usize,
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

fn draw_chain_row_strip(grid: &mut Grid, area: Area) {
    draw_title(grid, area.column, STRIP_TITLE_ROW, "CHAIN ROWS", false);
    let heading = "K T1 T2 T3 T4 T5 T6 T7 T8  TICK";
    grid.put(area.column, STRIP_HEADING_ROW, heading, HEADING);

    for entry in 0..CHAIN_ENTRIES {
        let row = STRIP_FIRST_ROW + entry;
        grid.put(area.column, row, &format!("{entry:X}"), HEADING);
        for track in 0..STRIP_TRACKS {
            let offset = STRIP_FIRST_TRACK_COLUMN + track * STRIP_TRACK_WIDTH;
            grid.put(area.column + offset, row, "--", EMPTY);
        }
        grid.put(area.column + STRIP_TICKS_COLUMN, row, "   -", EMPTY);
    }
}
