use std::ops::RangeInclusive;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPosition {
    pub column: usize,
    pub row: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    first: CellPosition,
    last: CellPosition,
}

impl CellRange {
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
