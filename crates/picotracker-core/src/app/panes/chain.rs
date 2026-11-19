//! The Chain pane: the entries of the Chain under the Song cursor, and the Chain row strip.

use super::{
    EMPTY, FILL, HEADING, SELECTED, VALUE, draw_out_of_range_flag, draw_row_label,
    draw_styled_value, draw_title, id_text, nudged_id, stepped, typed_value,
};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::app::selection::{CellPosition, CellRange};
use crate::project::{CHAIN_ENTRIES, Chain, ChainEntry, ChainId, PhraseId, Project, TRACKS};
use crate::screen::grid::{Grid, Style};
use std::ops::RangeInclusive;

const COARSE_TRANSPOSE: i32 = 12;
const HEADER_ROWS: usize = 2;
const STRIP_TITLE_ROW: usize = 19;
const STRIP_HEADING_ROW: usize = 20;
const STRIP_FIRST_ROW: usize = 21;
const STRIP_FIRST_TRACK_COLUMN: usize = 2;
const STRIP_TRACK_WIDTH: usize = 3;
const STRIP_TICKS_COLUMN: usize = 27;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryCell {
    Phrase,
    Transpose,
    Fill,
}

const ENTRY_CELLS: [(EntryCell, usize, &str); 3] = [
    (EntryCell::Phrase, 2, "--"),
    (EntryCell::Transpose, 5, "---"),
    (EntryCell::Fill, 9, "----"),
];

/// What the Chain pane shows.
#[derive(Clone, Copy, Debug)]
pub struct ChainView {
    pub song_row: usize,
    pub track: usize,
    pub playing_entry: Option<usize>,
}

/// The Chain pane's cursor.
#[derive(Debug, Default)]
pub struct ChainPane {
    entry: usize,
    column: usize,
    first_digit: Option<u8>,
    typing_negative: bool,
}

/// Where the Chain pane's cursor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainCursorPlace {
    entry: usize,
    column: usize,
}

impl ChainPane {
    /// Where the cursor is now.
    pub fn cursor_place(&self) -> ChainCursorPlace {
        ChainCursorPlace {
            entry: self.entry,
            column: self.column,
        }
    }

    /// Puts the cursor back where it was.
    pub fn return_to(&mut self, place: ChainCursorPlace) {
        self.entry = place.entry;
        self.column = place.column;
    }

    /// Puts the cursor on a Chain entry.
    pub fn jump_to(&mut self, entry: usize) {
        self.entry = entry.min(CHAIN_ENTRIES - 1);
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (columns, rows) = direction.delta();
        self.column = stepped(self.column, columns, ENTRY_CELLS.len());
        self.entry = stepped(self.entry, rows, CHAIN_ENTRIES);
    }

    /// The entry under the cursor.
    pub fn entry(&self) -> usize {
        self.entry
    }

    fn cell(&self) -> EntryCell {
        ENTRY_CELLS[self.column].0
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
        self.first_digit = None;
    }

    /// Types a hexadecimal digit into the cell under the cursor.
    pub fn type_digit(&mut self, chain: &mut Chain, digit: u8) {
        let is_first_digit = self.first_digit.is_none();
        let value = typed_value(&mut self.first_digit, digit);
        let slot = chain.entry_mut(self.entry);
        match (self.cell(), slot.as_mut()) {
            (EntryCell::Phrase, _) => set_phrase(slot, PhraseId::new(value)),
            (EntryCell::Transpose, Some(entry)) => {
                if is_first_digit {
                    self.typing_negative = entry.transpose() < 0;
                }
                let sign = if self.typing_negative { -1 } else { 1 };
                entry.set_transpose(sign * i32::from(value));
            }
            _ => {}
        }
    }

    /// Raises or lowers the cell under the cursor. Nudging a Fill mark flips it.
    pub fn nudge(&mut self, chain: &mut Chain, up: bool, coarse: bool) {
        nudge_cell(chain.entry_mut(self.entry), self.cell(), up, coarse);
    }

    /// Clears the cell under the cursor. Clearing the Phrase empties the whole entry.
    pub fn clear(&mut self, chain: &mut Chain) {
        clear_cell(chain.entry_mut(self.entry), self.cell());
    }

    /// The cell under the cursor: a column and a Chain entry.
    pub fn position(&self) -> CellPosition {
        CellPosition {
            column: self.column,
            row: self.entry,
        }
    }

    /// Raises or lowers every cell in the range. Empty entries stay empty.
    pub fn nudge_cells(&self, chain: &mut Chain, cells: CellRange, up: bool, coarse: bool) {
        for entry in cells.rows() {
            let slot = chain.entry_mut(entry);
            if slot.is_some() {
                for column in cells.columns() {
                    nudge_cell(slot, ENTRY_CELLS[column].0, up, coarse);
                }
            }
        }
    }

    /// Clears every cell in the range.
    pub fn clear_cells(&self, chain: &mut Chain, cells: CellRange) {
        for entry in cells.rows() {
            for column in cells.columns() {
                clear_cell(chain.entry_mut(entry), ENTRY_CELLS[column].0);
            }
        }
    }

    /// The entries the range of cells is on.
    pub fn copy(&self, chain: &Chain, cells: CellRange) -> Vec<Option<ChainEntry>> {
        cells.rows().map(|entry| chain.entry(entry)).collect()
    }

    /// Overwrites the copied columns of the entries from the cursor down, and says how many rows
    /// fell past the last entry.
    pub fn paste(
        &self,
        chain: &mut Chain,
        columns: &RangeInclusive<usize>,
        rows: &[Option<ChainEntry>],
    ) -> usize {
        let fitting = rows.len().min(CHAIN_ENTRIES - self.entry);
        for (entry, source) in (self.entry..).zip(&rows[..fitting]) {
            paste_entry(chain.entry_mut(entry), *source, columns);
        }
        rows.len() - fitting
    }

    /// Marks the selected cells, leaving the cursor as it is.
    pub fn draw_selection(&self, grid: &mut Grid, area: Area, cells: CellRange) {
        for entry in cells.rows() {
            for column in cells.columns() {
                if (entry, column) != (self.entry, self.column) {
                    let (_, offset, empty) = ENTRY_CELLS[column];
                    let row = HEADER_ROWS + entry;
                    grid.restyle(area.column + offset, row, empty.len(), SELECTED);
                }
            }
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
            for (column, (cell, offset, empty)) in ENTRY_CELLS.iter().enumerate() {
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

/// Raises or lowers one cell of an entry. Nudging a Fill mark flips it.
fn nudge_cell(slot: &mut Option<ChainEntry>, cell: EntryCell, up: bool, coarse: bool) {
    match (cell, slot.as_mut()) {
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

/// Clears one cell of an entry. Clearing the Phrase empties the whole entry.
fn clear_cell(slot: &mut Option<ChainEntry>, cell: EntryCell) {
    match (cell, slot.as_mut()) {
        (EntryCell::Phrase, _) => *slot = None,
        (EntryCell::Transpose, Some(entry)) => entry.set_transpose(0),
        (EntryCell::Fill, Some(entry)) => entry.set_fill(false),
        _ => {}
    }
}

/// Overwrites the copied columns of one entry. An empty source empties those columns.
fn paste_entry(
    slot: &mut Option<ChainEntry>,
    source: Option<ChainEntry>,
    columns: &RangeInclusive<usize>,
) {
    let copied = |cell| {
        ENTRY_CELLS
            .iter()
            .position(|(column, _, _)| *column == cell)
            .is_some_and(|column| columns.contains(&column))
    };
    if copied(EntryCell::Phrase) {
        match source {
            Some(source) => set_phrase(slot, source.phrase()),
            None => *slot = None,
        }
    }
    let Some(target) = slot.as_mut() else {
        return;
    };
    if copied(EntryCell::Transpose) {
        target.set_transpose(source.map_or(0, |source| i32::from(source.transpose())));
    }
    if copied(EntryCell::Fill) {
        target.set_fill(source.is_some_and(ChainEntry::fill));
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
