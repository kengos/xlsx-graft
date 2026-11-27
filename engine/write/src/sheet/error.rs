//! [`SheetError`]: why a worksheet value cannot be made.

use super::CellRange;
use thiserror::Error;

/// A value for a worksheet that Excel cannot hold: the caller's mistake,
/// refused when the value is made, so any value `new` returned can be
/// written without another check. A reason can be added later, so a
/// `match` needs a wildcard arm.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum SheetError {
    /// Columns outside 1 to 16,384 (A to XFD), or the first after the last.
    #[error("the columns {first} to {last} are not in 1 to 16384 in order")]
    ColumnRange {
        /// The first column, from 1.
        first: u16,
        /// The last column.
        last: u16,
    },
    /// A column width that is not finite, below 0 or above 255.
    #[error("the column width {width} is not a number from 0 to 255")]
    ColumnWidth {
        /// The width as the caller gave it.
        width: f64,
    },
    /// Column widths whose columns are not in ascending order without
    /// overlap (Excel rewrites overlapping `col` elements).
    #[error(
        "the columns {first} to {last} overlap or precede the columns of the previous column width"
    )]
    ColumnsOverlap {
        /// The first column of the width that breaks the order.
        first: u16,
        /// The last column of the width that breaks the order.
        last: u16,
    },
    /// A row outside 1 to 1,048,576.
    #[error("the row {row} is not in 1 to 1048576")]
    RowRange {
        /// The row as the caller gave it.
        row: u32,
    },
    /// A row height that is not finite, below 0 or above 409.
    #[error("the row height {height} is not a number from 0 to 409")]
    RowHeight {
        /// The height as the caller gave it.
        height: f64,
    },
    /// More rows or columns frozen than the sheet has to scroll: past
    /// 1,048,575 rows or 16,383 columns.
    #[error("{rows} rows and {columns} columns cannot be frozen: at most 1048575 and 16383")]
    FreezeRange {
        /// The rows as the caller gave them.
        rows: u32,
        /// The columns as the caller gave them.
        columns: u16,
    },
    /// A merged range of one cell.
    #[error("the range {range} is one cell, which cannot be merged")]
    MergeSingleCell {
        /// The range.
        range: CellRange,
    },
    /// Two merged ranges that share a cell.
    #[error("the merged range {range} overlaps {other}")]
    MergeOverlap {
        /// The range that comes later in the order given.
        range: CellRange,
        /// The range it overlaps.
        other: CellRange,
    },
    /// More merged ranges than a sheet holds.
    #[error("{count} merged ranges are more than the {limit} a sheet holds")]
    TooManyMerges {
        /// How many ranges the caller gave.
        count: usize,
        /// The most a sheet holds.
        limit: usize,
    },
    /// A sheet protection password with no character.
    #[error("the sheet protection password is empty")]
    PasswordEmpty,
    /// A password longer than 255 UTF-16 code units.
    #[error("the sheet protection password is {length} UTF-16 code units, at most 255")]
    PasswordTooLong {
        /// The length in UTF-16 code units.
        length: usize,
    },
    /// A spin count of 0 or past 10,000,000.
    #[error("the spin count {spin_count} is not in 1 to 10000000")]
    SpinCount {
        /// The spin count as the caller gave it.
        spin_count: u32,
    },
}
