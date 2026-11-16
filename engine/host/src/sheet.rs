//! [`Grid`]: the cells of one sheet, what the `describe_sheet` view shows,
//! and the JSON every host prints it as.
//!
//! ```json
//! {
//!   "ok": true, "error": null, "diagnostics": [],
//!   "sheet": { "index": 0, "name": "Sheet1", "kind": "worksheet", "state": "visible" },
//!   "dimension": "A1:C3",
//!   "used": "A1:C3",
//!   "shown": "A1:C3",
//!   "truncated": { "rows": false, "columns": false, "merges": false },
//!   "complete": true,
//!   "rows": [
//!     { "row": 1, "cells": [
//!       { "ref": "A1", "type": "string", "value": "Name" },
//!       { "ref": "C1", "type": "number", "value": 2.5 } ] }
//!   ],
//!   "merges": ["A1:B1"]
//! }
//! ```
//!
//! After `ok`, `error` and `diagnostics` ([`View`](crate::View)), the keys
//! and their types are the contract; a key may be added, never renamed or
//! retyped.
//!
//! - `sheet` is the sheet that was read: `index` in the workbook's list,
//!   its `name` as an [`Echo`], its `kind` (`worksheet`, `chartsheet`,
//!   `dialogsheet`, `macrosheet`, or `unknown` when its part is not known)
//!   and its `state` (`visible`, `hidden`, `veryHidden`). It is `null` when
//!   no sheet was chosen (the workbook could not be read, or no sheet is
//!   the one asked for, or two have its name) and when the package could not
//!   be read before a sheet was chosen (`error.code` is `io`). When `io`
//!   occurs while the rows of the chosen sheet are read, `sheet` is set.
//! - `dimension` is the `ref` of the sheet's `dimension`, as written, as an
//!   [`Echo`] (it is not checked to be a range, and writers do not all keep
//!   it right); `null` when the sheet has none.
//! - `used` is the range from the first to the last row and column of the
//!   cells that hold a value, in the whole sheet, whatever the request's
//!   range; `null` when none does, and when the sheet could not be read.
//! - `shown` is the range the view looked at: from the top left corner of
//!   the request's range (A1 without one), at most `maxRows` rows and
//!   `maxColumns` columns and never past the range, and not past the last
//!   row and column of `used`. It is `null` when that is nothing: when
//!   `used` is `null`, or when the corner is below or to the right of the
//!   last row or column of `used`. It may hold no cell with a value (with
//!   `maxRows` 1 and a value only in A5, it is `A1:A1` and `rows` is
//!   empty). Every range (`used`, `shown` and each of `merges`) is written
//!   with two corners, `A1:A1` for one cell.
//! - `truncated` says whether cells with a value, or merged ranges, were left
//!   out (`rows`, `columns` and `merges`, always all three). `rows` when
//!   some in the request's range lie below `shown`, `columns` when some in
//!   it lie to the right of the last column of `shown`, in any row (below
//!   `shown` too). At most [`SheetRequest::MAX_CELLS`] cells (100,000) and
//!   [`SheetRequest::MAX_TEXT_CHARS`] characters of strings and error values
//!   (1,000,000, each value cut at 1,000) are listed whatever the request:
//!   a row that would pass either, and the rows below, are left out whole
//!   and set `rows`. A sheet of long strings is cut short even with the
//!   default window. `merges` is true when more than
//!   [`SheetRequest::MAX_MERGES`] (100,000) merged ranges overlap `shown`,
//!   of which the first are listed, or when the sheet has more merged
//!   ranges than `maxMergedCells`, of which only the first were read.
//! - `complete` is whether the sheet was read to its end with nothing
//!   stopping it. When it is false, `used` and `truncated` are those of the
//!   rows read, and the diagnostics say what stopped the reading. The
//!   `mergeCells` stand after `sheetData`, so `merges` is empty when the
//!   reading stopped in the rows.
//! - `rows` lists, from the top, the rows of `shown` that have a cell with a
//!   value, each with those cells from the left (see [`GridCell`] for a
//!   cell's keys: `ref`, `type`, `value`, and `truncated` when its text was
//!   cut).
//! - `merges` lists, up to [`SheetRequest::MAX_MERGES`], the merged ranges
//!   that overlap `shown`, in the order of the sheet. A `mergeCell` whose
//!   `ref` is not a range of cells is passed over. When the sheet has more
//!   than `maxMergedCells`, the first are kept, and the view has the
//!   diagnostic `merged_cells_too_many` if the sheet was read to its end.
//!
//! # How large a result is
//!
//! Every output that grows with the workbook is bounded, in the JSON and in
//! [`GridText`] (UTF-8, one line of JSON; a character is at most 4 bytes):
//!
//! | Output | Bound | At most |
//! | --- | --- | --- |
//! | cells with a value | [`SheetRequest::MAX_CELLS`], 100,000, a row at a time | 8 MB |
//! | text of a string or an error value | 1,000 characters each, [`SheetRequest::MAX_TEXT_CHARS`], 1,000,000, in all | 4 MB |
//! | rows | at most `maxRows`, 10,000, those with a value | 0.3 MB |
//! | the table of the text | [`GridText::MAX_DRAWN_CELLS`], 100,000 cells drawn, else a list of the cells with a value | 0.5 MB |
//! | `merges` | [`SheetRequest::MAX_MERGES`], 100,000 | 2.4 MB |
//! | `sheet.name`, `dimension` | an [`Echo`], 200 characters | 800 bytes each |
//! | `error.message` | a fixed sentence, a name as an [`Echo`], and for `io` the system's message and the path the caller gave, as it was given | a few KB and the path |
//! | `diagnostics` | `maxDiagnostics`, 10,000 at most, 10 KiB each at most | 100 MB at most, 10 MB at the default of 1,000 |
//!
//! Without the diagnostics, a result is some 15 MB of JSON or 13 MB of text
//! at the most, and some 25 MB with the diagnostics of the default policy;
//! a policy that raises `maxDiagnostics` to its hard max adds up to some
//! 100 MB of diagnostics, as it does to a verify result. What the view
//! holds in memory does not grow with the sheet's rows: the cells listed,
//! the merged ranges read (`maxMergedCells`, at 16 bytes each), one row
//! (`maxRowBytes`), one XML event (`maxXmlEventBytes`), the shared strings
//! (up to twice `sharedStringsSpillBytes`, then a write buffer), and the
//! output written from them, which is held beside them, so the peak is about
//! twice the output. The time grows with the rows.

mod cell;
mod collect;
mod describe;
mod request;
mod text;

use serde::{Serialize, Serializer};
use xlsx_graft_diagnostics::Echo;
use xlsx_graft_read::{CellRange, Sheet, SheetKind, SheetState};

pub use cell::GridCell;
#[cfg(test)]
pub(crate) use describe::describe;
pub use describe::{describe_sheet, describe_sheet_bytes, describe_sheet_path};
pub use request::{SheetRequest, SheetRequestError};
pub use text::GridText;

/// What the `describe_sheet` view shows of a sheet; its content is read
/// through [`View::to_json`](crate::View::to_json) and [`GridText`].
#[derive(Debug, Default, Serialize)]
pub struct Grid {
    /// The sheet that was read; `null` when none was chosen or the package
    /// could not be read.
    pub(crate) sheet: Option<GridSheet>,
    /// The `ref` of the sheet's `dimension`, as written.
    pub(crate) dimension: Option<Echo>,
    /// The range of the cells that hold a value, in the whole sheet.
    pub(crate) used: Option<Span>,
    /// The range the view looked at.
    pub(crate) shown: Option<Span>,
    /// Whether cells with a value were left out below or to the right of
    /// `shown`, or merged ranges were left out.
    pub(crate) truncated: Truncated,
    /// Whether the sheet was read to its end with nothing stopping it.
    pub(crate) complete: bool,
    /// The rows of `shown` that have a cell with a value.
    pub(crate) rows: Vec<GridRow>,
    /// The merged ranges that overlap `shown`.
    pub(crate) merges: Vec<Span>,
    /// Whether more than [`SheetRequest::MAX_MERGES`] merged ranges overlap
    /// `shown`, which `truncated.merges` says too. Not written; `GridText`
    /// tells it from the sheet having more merged ranges than were read.
    #[serde(skip)]
    pub(crate) merges_over_max: bool,
    /// How many rows below `shown` have a cell with a value in the
    /// request's range. Not written; `GridText` says it.
    #[serde(skip)]
    pub(crate) rows_below: u64,
}

/// The sheet a [`Grid`] shows.
#[derive(Debug, Serialize)]
pub struct GridSheet {
    /// Its index in the workbook's list, from 0.
    pub(crate) index: usize,
    /// Its name.
    pub(crate) name: Echo,
    /// `worksheet`, `chartsheet`, `dialogsheet`, `macrosheet`, or `unknown`
    /// when its part is not known.
    pub(crate) kind: &'static str,
    /// `visible`, `hidden` or `veryHidden`.
    pub(crate) state: &'static str,
}

/// Which cells with a value, and which merged ranges, a [`Grid`] left out.
#[derive(Debug, Default, Serialize)]
pub struct Truncated {
    /// Some lie below `shown`.
    pub(crate) rows: bool,
    /// Some lie to the right of `shown`.
    pub(crate) columns: bool,
    /// More than `MAX_MERGES` merged ranges overlap `shown`, or the sheet has
    /// more than `maxMergedCells`, of which only the first were read.
    pub(crate) merges: bool,
}

/// One row of a [`Grid`]: its number and its cells with a value.
#[derive(Debug, Serialize)]
pub struct GridRow {
    /// The row's number, from 1.
    pub(crate) row: u32,
    /// The row's cells that hold a value, from the left.
    pub(crate) cells: Vec<GridCell>,
}

/// A range, written with its two corners: `A1:C3`, `B2:B2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span(pub(crate) CellRange);

impl Serialize for Span {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

impl GridSheet {
    fn of(index: usize, sheet: &Sheet) -> Self {
        let kind = match sheet.part().map(|part| part.kind()) {
            Some(SheetKind::Worksheet) => "worksheet",
            Some(SheetKind::Chartsheet) => "chartsheet",
            Some(SheetKind::Dialogsheet) => "dialogsheet",
            Some(SheetKind::Macrosheet) => "macrosheet",
            None => "unknown",
        };
        let state = match sheet.state() {
            SheetState::Visible => "visible",
            SheetState::Hidden => "hidden",
            SheetState::VeryHidden => "veryHidden",
        };
        Self {
            index,
            name: Echo::new(sheet.name()),
            kind,
            state,
        }
    }
}

#[cfg(test)]
mod tests;
