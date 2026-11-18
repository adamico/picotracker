//! The character grid: what the App core draws, and what tests read back as text.

use super::palette::Colour;

/// Columns in the character grid.
pub const GRID_COLUMNS: usize = 106;
/// Rows in the character grid.
pub const GRID_ROWS: usize = 45;

/// The colours of one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub fg: Colour,
    pub bg: Colour,
}

impl Style {
    /// A foreground colour on the black background.
    pub const fn fg(fg: Colour) -> Self {
        Self {
            fg,
            bg: Colour::Black,
        }
    }
}

/// One character on the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

/// A 106×45 grid of styled characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cells: Vec<Cell>,
}

impl Grid {
    /// A blank grid.
    pub fn new() -> Self {
        let blank = Cell {
            ch: ' ',
            style: Style::fg(Colour::LightGrey),
        };
        Self {
            cells: vec![blank; GRID_COLUMNS * GRID_ROWS],
        }
    }

    /// The cell at a column and row.
    ///
    /// # Panics
    ///
    /// Panics if the position is outside the grid.
    pub fn cell(&self, column: usize, row: usize) -> Cell {
        assert!(column < GRID_COLUMNS && row < GRID_ROWS);
        self.cells[row * GRID_COLUMNS + column]
    }

    /// Writes text from a position, clipping whatever falls outside the grid.
    pub fn put(&mut self, column: usize, row: usize, text: &str, style: Style) {
        if row >= GRID_ROWS {
            return;
        }
        for (x, ch) in (column..GRID_COLUMNS).zip(text.chars()) {
            self.cells[row * GRID_COLUMNS + x] = Cell { ch, style };
        }
    }

    /// Blanks a whole row in one background colour.
    pub fn fill_row(&mut self, row: usize, style: Style) {
        self.put(0, row, &" ".repeat(GRID_COLUMNS), style);
    }

    /// The grid as text: one line per row, without trailing spaces.
    pub fn text_dump(&self) -> String {
        let mut dump = String::new();
        for row in self.cells.chunks(GRID_COLUMNS) {
            let line: String = row.iter().map(|cell| cell.ch).collect();
            dump.push_str(line.trim_end());
            dump.push('\n');
        }
        dump
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}
