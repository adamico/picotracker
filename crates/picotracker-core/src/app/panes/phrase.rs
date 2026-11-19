//! The Phrase pane: the header and Steps of the Phrase being edited.

use super::{
    EMPTY, HEADING, SELECTED, VALUE, draw_out_of_range_flag, draw_row_label, draw_styled_value,
    draw_title, id_text, stepped,
};
use crate::app::layout::Area;
use crate::app::selection::{CellPosition, CellRange};
use crate::app::{Direction, KeyContext};
use crate::project::{FIRST_CUSTOM_INSTRUMENT, MAX_STEPS, NOTE_NAMES, Phrase, PhraseId, Step};
use crate::screen::grid::Grid;
use std::ops::RangeInclusive;

/// Rows above the first Step: the title, three header rows and the column headings.
const HEADER_ROWS: usize = 5;
const FIRST_HEADER_ROW: usize = 1;

const MAX_OCTAVE: u8 = 4;
const SEMITONES_PER_OCTAVE: i32 = 12;
const COARSE_SPEED: i32 = 16;
const COARSE_LENGTH: i32 = 4;

/// The cells of a Step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepCell {
    Pitch,
    Instrument,
    Volume,
    Effect,
}

/// The cells of a Step: which each is, where it starts and what a silent Step shows.
const STEP_CELLS: [(StepCell, usize, &str); 4] = [
    (StepCell::Pitch, 3, "..."),
    (StepCell::Instrument, 7, ".."),
    (StepCell::Volume, 10, "."),
    (StepCell::Effect, 12, "."),
];

/// The editable values of the Phrase header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeaderCell {
    Speed,
    Length,
    Noiz,
    Buzz,
    Detune,
    Reverb,
    Dampen,
}

/// The header's rows of editable values, each with the column its value starts in.
const HEADER_CELLS: [&[(HeaderCell, usize)]; 3] = [
    &[(HeaderCell::Speed, 4)],
    &[
        (HeaderCell::Length, 4),
        (HeaderCell::Noiz, 10),
        (HeaderCell::Buzz, 15),
    ],
    &[
        (HeaderCell::Detune, 3),
        (HeaderCell::Reverb, 8),
        (HeaderCell::Dampen, 13),
    ],
];

/// What the Phrase pane shows.
#[derive(Clone, Copy, Debug)]
pub struct PhraseView<'a> {
    pub id: Option<PhraseId>,
    pub phrase: &'a Phrase,
    pub transpose: i8,
    /// The Step the Playhead is on, when it is in this Phrase.
    pub playing_step: Option<usize>,
}

/// The Phrase pane's cursor, and the values that new notes take.
#[derive(Debug)]
pub struct PhrasePane {
    step: usize,
    column: usize,
    /// The header row under the cursor, when the cursor is in the header.
    header_row: Option<usize>,
    octave: u8,
    instrument: u8,
    volume: u8,
    /// The digit typed just before this one into the same two-digit cell.
    typed_digit: Option<u8>,
}

impl Default for PhrasePane {
    fn default() -> Self {
        Self {
            step: 0,
            column: 0,
            header_row: None,
            octave: 2,
            instrument: 2,
            volume: 5,
            typed_digit: None,
        }
    }
}

/// Where the Phrase pane's cursor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhraseCursorPlace {
    step: usize,
    column: usize,
    header_row: Option<usize>,
}

impl PhrasePane {
    /// Where the cursor is now.
    pub fn cursor_place(&self) -> PhraseCursorPlace {
        PhraseCursorPlace {
            step: self.step,
            column: self.column,
            header_row: self.header_row,
        }
    }

    /// Puts the cursor back where it was.
    pub fn return_to(&mut self, place: PhraseCursorPlace) {
        self.step = place.step;
        self.column = place.column;
        self.header_row = place.header_row;
    }

    /// The Step under the cursor.
    pub fn step(&self) -> usize {
        self.step
    }

    /// The Step under the cursor, when the cursor is on its pitch or Instrument.
    pub fn note_step(&self) -> Option<usize> {
        matches!(
            self.step_cell(),
            Some(StepCell::Pitch | StepCell::Instrument)
        )
        .then_some(self.step)
    }

    /// Puts the cursor on a Step.
    pub fn jump_to(&mut self, step: usize) {
        self.step = step.min(MAX_STEPS - 1);
        self.header_row = None;
    }

    /// Puts the cursor back on the first Step, keeping the values that new notes take.
    pub fn return_to_top(&mut self) {
        self.step = 0;
        self.column = 0;
        self.header_row = None;
    }

    /// Keeps the cursor inside a Phrase the pane has just moved to.
    pub fn keep_inside(&mut self, phrase: &Phrase) {
        self.step = self.step.min(phrase.length() - 1);
    }

    /// What the keys that type characters do here.
    pub fn key_context(&self) -> KeyContext {
        if self.step_cell() == Some(StepCell::Pitch) {
            KeyContext::PhrasePitch
        } else {
            KeyContext::PhraseValue
        }
    }

    pub fn move_cursor(&mut self, direction: Direction, phrase: &Phrase) {
        let (columns, rows) = direction.delta();
        match self.header_row {
            Some(row) => self.move_in_header(row, columns, rows),
            None if rows < 0 && self.step == 0 => self.enter_header(),
            None => {
                self.column = stepped(self.column, columns, STEP_CELLS.len());
                self.step = stepped(self.step, rows, phrase.length());
            }
        }
    }

    fn move_in_header(&mut self, row: usize, columns: isize, rows: isize) {
        if rows > 0 && row + 1 == HEADER_CELLS.len() {
            self.header_row = None;
            return;
        }
        let row = stepped(row, rows, HEADER_CELLS.len());
        self.header_row = Some(row);
        self.column = stepped(self.column, columns, HEADER_CELLS[row].len());
    }

    fn enter_header(&mut self) {
        let row = HEADER_CELLS.len() - 1;
        self.header_row = Some(row);
        self.column = self.column.min(HEADER_CELLS[row].len() - 1);
    }

    fn step_cell(&self) -> Option<StepCell> {
        match self.header_row {
            Some(_) => None,
            None => Some(STEP_CELLS[self.column].0),
        }
    }

    fn header_cell(&self) -> Option<HeaderCell> {
        let row = self.header_row?;
        Some(HEADER_CELLS[row][self.column].0)
    }

    /// Enters a note with the current Instrument and volume, then moves down one Step.
    pub fn enter_note(&mut self, phrase: &mut Phrase, piano_key: u8) {
        if self.step_cell() != Some(StepCell::Pitch) {
            return;
        }
        let pitch = i32::from(self.octave) * SEMITONES_PER_OCTAVE + i32::from(piano_key);
        let step = phrase.step_mut(self.step);
        step.set_pitch(pitch);
        step.set_instrument(i32::from(self.instrument));
        step.set_volume(i32::from(self.volume));
        self.step = stepped(self.step, 1, phrase.length());
    }

    /// Changes the octave that piano keys play in, stopping at 0 and 4.
    pub fn change_octave(&mut self, delta: i8) {
        self.octave = self.octave.saturating_add_signed(delta).min(MAX_OCTAVE);
    }

    /// Ends the two-digit value being typed, so that the next digit starts a new one.
    pub fn finish_typing(&mut self) {
        self.typed_digit = None;
    }

    /// Types a hexadecimal digit into the cell under the cursor. Two digits in a row make one
    /// value in the cells that hold two.
    pub fn type_digit(&mut self, phrase: &mut Phrase, digit: u8) {
        let typed = self.typed_digit.map_or(0, |high| high << 4) | digit;
        let two_digits = matches!(self.step_cell(), Some(StepCell::Instrument))
            || matches!(
                self.header_cell(),
                Some(HeaderCell::Speed | HeaderCell::Length)
            );
        self.typed_digit = two_digits.then_some(digit);
        let value = i32::from(if two_digits { typed } else { digit });
        self.set_cell(phrase, value);
    }

    /// Raises or lowers the cell under the cursor.
    pub fn nudge(&mut self, phrase: &mut Phrase, up: bool, coarse: bool) {
        let amount = if coarse { self.coarse_amount() } else { 1 };
        let delta = if up { amount } else { -amount };
        let value = self.cell_value(phrase) + delta;
        self.set_cell(phrase, value);
    }

    fn coarse_amount(&self) -> i32 {
        match (self.step_cell(), self.header_cell()) {
            (Some(StepCell::Pitch), _) => SEMITONES_PER_OCTAVE,
            (_, Some(HeaderCell::Speed)) => COARSE_SPEED,
            (_, Some(HeaderCell::Length)) => COARSE_LENGTH,
            _ => 1,
        }
    }

    fn cell_value(&self, phrase: &Phrase) -> i32 {
        let step = phrase.step(self.step);
        let filters = phrase.filters();
        match (self.step_cell(), self.header_cell()) {
            (Some(StepCell::Pitch), _) => i32::from(step.pitch()),
            (Some(StepCell::Instrument), _) => i32::from(step.instrument()),
            (Some(StepCell::Volume), _) => i32::from(step.volume()),
            (Some(StepCell::Effect), _) => i32::from(step.effect()),
            (_, Some(HeaderCell::Speed)) => i32::from(phrase.speed()),
            (_, Some(HeaderCell::Length)) => phrase.length() as i32,
            (_, Some(HeaderCell::Noiz)) => i32::from(filters.noiz()),
            (_, Some(HeaderCell::Buzz)) => i32::from(filters.buzz()),
            (_, Some(HeaderCell::Detune)) => i32::from(filters.detune()),
            (_, Some(HeaderCell::Reverb)) => i32::from(filters.reverb()),
            (_, Some(HeaderCell::Dampen)) => i32::from(filters.dampen()),
            (None, None) => 0,
        }
    }

    fn set_cell(&mut self, phrase: &mut Phrase, value: i32) {
        if let Some(cell) = self.step_cell() {
            self.set_step_cell(phrase, cell, value);
        }
        match self.header_cell() {
            Some(HeaderCell::Speed) => phrase.set_speed(value),
            Some(HeaderCell::Length) => {
                phrase.set_length(value);
                self.step = self.step.min(phrase.length() - 1);
            }
            Some(HeaderCell::Noiz) => phrase.filters_mut().set_noiz(value > 0),
            Some(HeaderCell::Buzz) => phrase.filters_mut().set_buzz(value > 0),
            Some(HeaderCell::Detune) => phrase.filters_mut().set_detune(value),
            Some(HeaderCell::Reverb) => phrase.filters_mut().set_reverb(value),
            Some(HeaderCell::Dampen) => phrase.filters_mut().set_dampen(value),
            None => {}
        }
    }

    fn set_step_cell(&mut self, phrase: &mut Phrase, cell: StepCell, value: i32) {
        let step = phrase.step_mut(self.step);
        match cell {
            StepCell::Pitch => step.set_pitch(value),
            StepCell::Instrument => {
                step.set_instrument(value);
                self.instrument = step.instrument();
            }
            StepCell::Volume => {
                step.set_volume(value);
                self.volume = step.volume();
            }
            StepCell::Effect => step.set_effect(value),
        }
    }

    /// The cell under the cursor: a column and a Step. The header holds no cells to select.
    pub fn cell(&self) -> Option<CellPosition> {
        self.header_row.is_none().then_some(CellPosition {
            column: self.column,
            row: self.step,
        })
    }

    /// Raises or lowers every cell in the range. Silent Steps show as empty, so they stay as they
    /// are.
    pub fn nudge_cells(&self, phrase: &mut Phrase, cells: CellRange, up: bool, coarse: bool) {
        for index in cells.rows() {
            let step = phrase.step_mut(index);
            if !step.is_silent() {
                for column in cells.columns() {
                    nudge_step_cell(step, STEP_CELLS[column].0, up, coarse);
                }
            }
        }
    }

    /// Empties every cell in the range. A Step has no empty pitch or Instrument, so clearing either
    /// silences the Step and keeps its values.
    pub fn clear_cells(&self, phrase: &mut Phrase, cells: CellRange) {
        for index in cells.rows() {
            let step = phrase.step_mut(index);
            for column in cells.columns() {
                match STEP_CELLS[column].0 {
                    StepCell::Effect => step.set_effect(0),
                    _ => step.set_volume(0),
                }
            }
        }
    }

    /// The Steps the range of cells is on.
    pub fn copy(&self, phrase: &Phrase, cells: CellRange) -> Vec<Step> {
        cells.rows().map(|index| phrase.step(index)).collect()
    }

    /// Overwrites the copied columns of the Steps from the cursor down, and says how many rows
    /// fell past LEN.
    pub fn paste(
        &self,
        phrase: &mut Phrase,
        columns: &RangeInclusive<usize>,
        rows: &[Step],
    ) -> usize {
        let fitting = rows.len().min(phrase.length().saturating_sub(self.step));
        for (index, source) in (self.step..).zip(&rows[..fitting]) {
            let step = phrase.step_mut(index);
            *step = pasted_step(*step, *source, columns);
        }
        rows.len() - fitting
    }

    /// Marks the selected cells, leaving the cursor as it is.
    pub fn draw_selection(&self, grid: &mut Grid, area: Area, cells: CellRange) {
        for index in cells.rows() {
            for column in cells.columns() {
                if (index, column) != (self.step, self.column) {
                    let (_, offset, empty) = STEP_CELLS[column];
                    let row = HEADER_ROWS + index;
                    grid.restyle(area.column + offset, row, empty.len(), SELECTED);
                }
            }
        }
    }

    /// Moves the cursor to the Step the Playhead is on.
    pub fn follow(&mut self, step: usize) {
        if self.step != step {
            self.step = step;
            self.finish_typing();
        }
    }

    /// Draws a Phrase, or a blank Phrase under the title `--` when there is none to show.
    pub fn draw(&self, grid: &mut Grid, area: Area, focused: bool, view: PhraseView) {
        let PhraseView {
            id,
            phrase,
            transpose,
            playing_step,
        } = view;
        let number = id_text(id.map(PhraseId::number));
        let title = format!("PHRASE {number}   OCT {}", self.octave);
        draw_title(grid, area.column, 0, &title, focused);
        self.draw_header(grid, area, focused, phrase);
        grid.put(area.column, HEADER_ROWS - 1, "   NOT IN V F", HEADING);

        for index in 0..phrase.length().min(MAX_STEPS) {
            let row = HEADER_ROWS + index;
            let label = format!("{index:02X}");
            draw_row_label(grid, area.column, row, &label, playing_step == Some(index));
            let on_this_step = focused && self.header_row.is_none() && index == self.step;
            let texts = step_texts(phrase.step(index));
            for (cell, ((_, offset, _), text)) in STEP_CELLS.iter().zip(&texts).enumerate() {
                let is_cursor = on_this_step && cell == self.column;
                let style = if phrase.step(index).is_silent() {
                    EMPTY
                } else {
                    VALUE
                };
                draw_styled_value(grid, area.column + offset, row, text, is_cursor, style);
            }
            if phrase.step_is_out_of_range(index, transpose) {
                draw_out_of_range_flag(grid, area.column, row);
            }
        }
    }

    fn draw_header(&self, grid: &mut Grid, area: Area, focused: bool, phrase: &Phrase) {
        let filters = phrase.filters();
        let labels = [
            format!("SPD    BPM {}", phrase.bpm()),
            "LEN    NZ   BZ".to_owned(),
            "DT   RV   DP".to_owned(),
        ];
        for (row, label) in labels.iter().enumerate() {
            grid.put(area.column, FIRST_HEADER_ROW + row, label, HEADING);
        }

        for (row, cells) in HEADER_CELLS.iter().enumerate() {
            for (column, (cell, offset)) in cells.iter().enumerate() {
                let text = match cell {
                    HeaderCell::Speed => format!("{:02X}", phrase.speed()),
                    HeaderCell::Length => format!("{:02X}", phrase.length()),
                    HeaderCell::Noiz => u8::from(filters.noiz()).to_string(),
                    HeaderCell::Buzz => u8::from(filters.buzz()).to_string(),
                    HeaderCell::Detune => filters.detune().to_string(),
                    HeaderCell::Reverb => filters.reverb().to_string(),
                    HeaderCell::Dampen => filters.dampen().to_string(),
                };
                let is_cursor = focused && self.header_row == Some(row) && self.column == column;
                let screen_row = FIRST_HEADER_ROW + row;
                draw_styled_value(
                    grid,
                    area.column + offset,
                    screen_row,
                    &text,
                    is_cursor,
                    VALUE,
                );
            }
        }
    }
}

/// Raises or lowers one cell of a Step, stopping at its limits. Custom Instruments are left alone,
/// because nudging only steps through the Built-in Instruments.
fn nudge_step_cell(step: &mut Step, cell: StepCell, up: bool, coarse: bool) {
    let amount = if coarse && cell == StepCell::Pitch {
        SEMITONES_PER_OCTAVE
    } else {
        1
    };
    let delta = if up { amount } else { -amount };
    match cell {
        StepCell::Pitch => step.set_pitch(i32::from(step.pitch()) + delta),
        StepCell::Instrument if step.instrument() < FIRST_CUSTOM_INSTRUMENT => {
            step.set_instrument(i32::from(step.instrument()) + delta);
        }
        StepCell::Instrument => {}
        StepCell::Volume => step.set_volume(i32::from(step.volume()) + delta),
        StepCell::Effect => step.set_effect(i32::from(step.effect()) + delta),
    }
}

/// A Step with the copied columns of another written over it. A silent source shows as empty, so
/// pasting its pitch or Instrument silences the Step, as clearing those cells does.
fn pasted_step(target: Step, source: Step, columns: &RangeInclusive<usize>) -> Step {
    let copied = |cell| {
        STEP_CELLS
            .iter()
            .position(|(column, _, _)| *column == cell)
            .is_some_and(|column| columns.contains(&column))
    };
    let pick = |cell, from_source, from_target| {
        if copied(cell) {
            from_source
        } else {
            from_target
        }
    };
    let empties_note =
        source.is_silent() && (copied(StepCell::Pitch) || copied(StepCell::Instrument));
    let volume = if empties_note {
        0
    } else {
        pick(StepCell::Volume, source.volume(), target.volume())
    };
    Step::new(
        pick(StepCell::Pitch, source.pitch(), target.pitch()),
        pick(
            StepCell::Instrument,
            source.instrument(),
            target.instrument(),
        ),
        volume,
        pick(StepCell::Effect, source.effect(), target.effect()),
    )
    .unwrap_or(target)
}

/// The text of each cell of a Step. A silent Step shows as empty.
fn step_texts(step: Step) -> [String; 4] {
    if step.is_silent() {
        return STEP_CELLS.map(|(_, _, empty)| empty.to_owned());
    }
    let pitch = usize::from(step.pitch());
    [
        format!("{}{}", NOTE_NAMES[pitch % 12], pitch / 12),
        format!("{:02X}", step.instrument()),
        step.volume().to_string(),
        step.effect().to_string(),
    ]
}
