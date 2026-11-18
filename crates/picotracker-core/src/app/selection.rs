//! Rectangles of cells within one pane.

use std::ops::RangeInclusive;

/// One cell of a pane: its column and its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPosition {
    pub column: usize,
    pub row: usize,
}

/// A rectangle of cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    first: CellPosition,
    last: CellPosition,
}

impl Block {
    /// The rectangle with these two cells at opposite corners.
    pub fn between(one: CellPosition, other: CellPosition) -> Self {
        Self {
            first: CellPosition {
                column: one.column.min(other.column),
                row: one.row.min(other.row),
            },
            last: CellPosition {
                column: one.column.max(other.column),
                row: one.row.max(other.row),
            },
        }
    }

    /// The rectangle of one cell.
    pub fn at(cell: CellPosition) -> Self {
        Self::between(cell, cell)
    }

    pub fn columns(self) -> RangeInclusive<usize> {
        self.first.column..=self.last.column
    }

    pub fn rows(self) -> RangeInclusive<usize> {
        self.first.row..=self.last.row
    }
}
