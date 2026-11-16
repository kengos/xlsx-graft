//! [`CellRange`]: a rectangle of cells, read from its A1 reference (a
//! `mergeCell`'s or a `dimension`'s `ref`, ST_Ref, ECMA-376-1:2016
//! §18.18.62): two corners, or one cell.

use crate::{CellRef, ValueError};
use std::fmt;

/// A rectangle of cells, its corners within the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    /// The top left corner.
    first: CellRef,
    /// The bottom right corner.
    last: CellRef,
}

impl CellRange {
    /// The rectangle with corners `a` and `b`, given in either order: its
    /// top left corner is the leftmost column and the topmost row of the
    /// two, its bottom right corner the rightmost column and the bottom
    /// row.
    #[must_use]
    pub fn new(a: CellRef, b: CellRef) -> Self {
        Self {
            first: corner(a.column().min(b.column()), a.row().min(b.row()), a),
            last: corner(a.column().max(b.column()), a.row().max(b.row()), a),
        }
    }

    /// Reads an A1 reference to a range: two cells with a colon between
    /// (`A1:B2`), in either order, or one cell (`A1`), which is the range
    /// of that cell alone.
    ///
    /// # Errors
    ///
    /// [`ValueError::Form`] when the text is not one cell or two joined by
    /// a colon, and [`ValueError::Range`] when a cell is outside the sheet,
    /// as [`CellRef::parse`] has it.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        match text.split_once(':') {
            Some((a, b)) => Ok(Self::new(CellRef::parse(a)?, CellRef::parse(b)?)),
            None => {
                let cell = CellRef::parse(text)?;
                Ok(Self::new(cell, cell))
            }
        }
    }

    /// The top left corner.
    #[must_use]
    pub fn first(self) -> CellRef {
        self.first
    }

    /// The bottom right corner.
    #[must_use]
    pub fn last(self) -> CellRef {
        self.last
    }

    /// Whether this range and `other` have a cell in common.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.intersect(other).is_some()
    }

    /// The cells this range and `other` have in common, `None` when there
    /// are none.
    #[must_use]
    pub fn intersect(self, other: Self) -> Option<Self> {
        let (left, top) = (
            self.first.column().max(other.first.column()),
            self.first.row().max(other.first.row()),
        );
        let (right, bottom) = (
            self.last.column().min(other.last.column()),
            self.last.row().min(other.last.row()),
        );
        (left <= right && top <= bottom).then(|| Self {
            first: corner(left, top, self.first),
            last: corner(right, bottom, self.last),
        })
    }
}

/// The cell at `column` and `row`, which are within the sheet whenever
/// they come from the corners of ranges; `fallback` stands for the
/// impossible other case.
fn corner(column: u16, row: u32, fallback: CellRef) -> CellRef {
    CellRef::new(column, row).unwrap_or(fallback)
}

impl fmt::Display for CellRange {
    /// Both corners, `A1:B2`, also for a range of one cell (`A1:A1`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.first, self.last)
    }
}

#[cfg(test)]
mod tests;
