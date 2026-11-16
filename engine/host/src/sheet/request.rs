//! [`SheetRequest`]: what `describe_sheet` is asked for (which sheet, which
//! cells, how many), checked before the workbook is opened, and
//! [`SheetRequestError`]: why a request is not one.

use thiserror::Error;
use xlsx_graft_diagnostics::Echo;
use xlsx_graft_read::{CellRange, SheetChoice};

/// A request for the cells of one sheet: the sheet, the range to start
/// from, and the most rows and columns to show.
///
/// The rows and columns shown start at the top left cell of the range
/// (A1 when there is none) and are at most `max_rows` rows and
/// `max_columns` columns, and never past the range. Only a request that
/// [`SheetRequest::new`] accepted exists, so
/// [`describe_sheet`](crate::describe_sheet) takes no invalid one.
///
/// Whatever the request, a view lists at most [`MAX_CELLS`](Self::MAX_CELLS)
/// cells (100,000), each string or error value cut at 1,000 characters, and
/// at most [`MAX_TEXT_CHARS`](Self::MAX_TEXT_CHARS) characters of them in
/// all (1,000,000): a sheet of long strings is cut short, below the rows
/// asked for, even with the default window, and says so in `truncated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetRequest {
    pub(super) sheet: Selection,
    pub(super) range: Option<CellRange>,
    pub(super) max_rows: u32,
    pub(super) max_columns: u16,
}

/// Which sheet a [`SheetRequest`] asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Selection {
    /// The sheet at index 0, whatever its kind.
    First,
    /// The sheet of this name, letter case aside.
    Name(String),
    /// The sheet at this index of the workbook's list.
    Index(usize),
}

impl SheetRequest {
    /// The rows shown when the request gives no `max_rows`.
    pub const DEFAULT_MAX_ROWS: u32 = 100;
    /// The most `max_rows` a request may give.
    pub const HARD_MAX_ROWS: u32 = 10_000;
    /// The columns shown when the request gives no `max_columns`.
    pub const DEFAULT_MAX_COLUMNS: u16 = 30;
    /// The most `max_columns` a request may give.
    pub const HARD_MAX_COLUMNS: u16 = 1_000;
    /// The most cells with a value that a view lists, whatever the rows and
    /// columns asked for: a row that would pass it, and the rows after it,
    /// are not listed.
    pub const MAX_CELLS: usize = 100_000;
    /// The most merged ranges that a view lists, of those that overlap the
    /// range shown, the first ones in the sheet's order.
    pub const MAX_MERGES: usize = 100_000;
    /// The most characters the strings and error values listed may have in
    /// all (each is cut at 1,000 characters), whatever the rows and columns
    /// asked for: a row that would pass it, and the rows after it, are not
    /// listed. The first row always fits: at most 1,000 columns of 1,000
    /// characters.
    pub const MAX_TEXT_CHARS: usize = 1_000_000;

    /// A request for `sheet` (the first sheet, index 0 whatever its kind,
    /// when `None`), the cells of `range` (an A1 range such as `A1:H50`, or
    /// one cell such as `B2`, in either order of corners; the whole sheet
    /// when `None`), at most `max_rows` rows (`DEFAULT_MAX_ROWS` when
    /// `None`) and `max_columns` columns (`DEFAULT_MAX_COLUMNS` when
    /// `None`).
    ///
    /// # Errors
    ///
    /// [`SheetRequestError::Range`] when `range` is not an A1 range of the
    /// sheet, [`SheetRequestError::MaxRows`] when `max_rows` is not from 1
    /// to `HARD_MAX_ROWS`, and [`SheetRequestError::MaxColumns`] when
    /// `max_columns` is not from 1 to `HARD_MAX_COLUMNS`. The first of the
    /// three that applies, in that order.
    pub fn new(
        sheet: Option<SheetChoice<'_>>,
        range: Option<&str>,
        max_rows: Option<u64>,
        max_columns: Option<u64>,
    ) -> Result<Self, SheetRequestError> {
        let sheet = match sheet {
            None => Selection::First,
            Some(SheetChoice::Name(name)) => Selection::Name(name.to_owned()),
            Some(SheetChoice::Index(index)) => Selection::Index(index),
        };
        let range = (range.map(|text| {
            CellRange::parse(text).map_err(|_| SheetRequestError::Range(echoed(text)))
        }))
        .transpose()?;
        let max_rows = match max_rows {
            None => Self::DEFAULT_MAX_ROWS,
            Some(rows) => u32::try_from(rows)
                .ok()
                .filter(|rows| (1..=Self::HARD_MAX_ROWS).contains(rows))
                .ok_or(SheetRequestError::MaxRows(rows))?,
        };
        let max_columns = match max_columns {
            None => Self::DEFAULT_MAX_COLUMNS,
            Some(columns) => u16::try_from(columns)
                .ok()
                .filter(|columns| (1..=Self::HARD_MAX_COLUMNS).contains(columns))
                .ok_or(SheetRequestError::MaxColumns(columns))?,
        };
        Ok(Self {
            sheet,
            range,
            max_rows,
            max_columns,
        })
    }
}

/// `text` as an [`Echo`] writes it: the request's range comes from a caller
/// (the MCP client's model), and the error is written back to it.
fn echoed(text: &str) -> String {
    Echo::new(text).as_str().to_owned()
}

/// Why the arguments of a [`SheetRequest`] are not a request. The caller
/// tells them from a failure to read the workbook by this type (the CLI
/// exits 2, the MCP server answers invalid params). The message is English
/// for a person and may change.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SheetRequestError {
    /// The range is not an A1 range of the sheet, `A1:H50` or `A1`. The text
    /// is the caller's, written as an [`Echo`] writes it.
    #[error(
        "range \"{0}\" is not a range of cells such as A1:H50 (columns A to XFD, rows 1 to 1048576)"
    )]
    Range(String),
    /// `maxRows` is not from 1 to 10,000.
    #[error("maxRows {0} is not between 1 and 10000")]
    MaxRows(u64),
    /// `maxColumns` is not from 1 to 1,000.
    #[error("maxColumns {0} is not between 1 and 1000")]
    MaxColumns(u64),
}

#[cfg(test)]
mod tests;
