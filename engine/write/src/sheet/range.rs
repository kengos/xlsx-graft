//! [`CellRange`]: a rectangle of cells, as `mergeCell`'s `ref` writes it.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use xlsx_graft_cell::{CellRef, ValueError};

/// A rectangle of cells, its corners given in either order and kept as the
/// top-left and the bottom-right. Ranges are ordered by the top row, then the
/// left column, then the bottom row, then the right column (row-major: the
/// top-left cell first, then the bottom-right one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    first: CellRef,
    last: CellRef,
}

impl CellRange {
    /// The range with the two cells as opposite corners.
    #[must_use]
    pub fn new(a: CellRef, b: CellRef) -> Self {
        // Both corners are cells of the sheet, so the smaller column and the
        // smaller row (and the larger of each) are too; `a` and `b` are the
        // fallback for `CellRef::new` returning `None`, which these corners
        // never cause.
        let corner = |column: u16, row: u32, fallback: CellRef| {
            CellRef::new(column, row).unwrap_or(fallback)
        };
        Self {
            first: corner(a.column().min(b.column()), a.row().min(b.row()), a),
            last: corner(a.column().max(b.column()), a.row().max(b.row()), b),
        }
    }

    /// Reads `A1:B2`, `B2:A1` (the corners in either order) or `A1` (one
    /// cell), each cell as [`CellRef::parse`] reads it.
    ///
    /// # Errors
    ///
    /// The [`ValueError`] of the cell [`CellRef::parse`] refuses.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        match text.split_once(':') {
            Some((a, b)) => Ok(Self::new(CellRef::parse(a)?, CellRef::parse(b)?)),
            None => {
                let cell = CellRef::parse(text)?;
                Ok(Self::new(cell, cell))
            }
        }
    }

    /// The top-left cell.
    #[must_use]
    pub fn first(self) -> CellRef {
        self.first
    }

    /// The bottom-right cell.
    #[must_use]
    pub fn last(self) -> CellRef {
        self.last
    }

    /// Whether the range is one cell.
    #[must_use]
    pub fn is_single_cell(self) -> bool {
        self.first == self.last
    }

    /// Whether the two ranges share a cell; ranges that only touch at an
    /// edge do not.
    pub(crate) fn overlaps(self, other: Self) -> bool {
        self.first.column() <= other.last.column()
            && other.first.column() <= self.last.column()
            && self.first.row() <= other.last.row()
            && other.first.row() <= self.last.row()
    }
}

impl CellRange {
    /// The top row, the left column, the bottom row and the right column:
    /// the fields `Ord` and `Hash` compare, in this order.
    fn key(self) -> (u32, u16, u32, u16) {
        (
            self.first.row(),
            self.first.column(),
            self.last.row(),
            self.last.column(),
        )
    }
}

impl Hash for CellRange {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl PartialOrd for CellRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CellRange {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl fmt::Display for CellRange {
    /// The two corners, `A1:B2`; a single cell is `A1:A1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.first, self.last)
    }
}
