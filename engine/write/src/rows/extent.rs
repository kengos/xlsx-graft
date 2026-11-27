//! `Extent`: the rows and columns the cells written so far span.

use xlsx_graft_cell::CellRef;

/// The first and last row and column of the cells written.
#[derive(Debug, Clone, Copy)]
pub(super) struct Extent {
    first_row: u32,
    last_row: u32,
    first_column: u16,
    last_column: u16,
}

impl Extent {
    /// `extent` with its last row set to `row` and its columns extended to
    /// include `first` to `last`. `row` is past every row `extent` spans, because
    /// rows are written in ascending order.
    pub(super) fn with_row(extent: Option<Self>, row: u32, first: u16, last: u16) -> Self {
        match extent {
            None => Self {
                first_row: row,
                last_row: row,
                first_column: first,
                last_column: last,
            },
            Some(before) => Self {
                last_row: row,
                first_column: before.first_column.min(first),
                last_column: before.last_column.max(last),
                ..before
            },
        }
    }

    /// The top-left and bottom-right cell of the extent.
    pub(super) fn corners(self) -> Option<(CellRef, CellRef)> {
        let top_left = CellRef::new(self.first_column, self.first_row);
        let bottom_right = CellRef::new(self.last_column, self.last_row);
        top_left.zip(bottom_right)
    }
}
