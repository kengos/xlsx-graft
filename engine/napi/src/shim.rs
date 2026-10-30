//! The N-API surface: the only part of this crate JavaScript sees.
//!
//! Compiled only with the `shim` feature, so the coverage gate never builds
//! it; the JS SDK's tests run it through the real addon instead. It must
//! stay a mapping from the owned values of `call` to JavaScript objects and
//! classes: a rule that lives here is a rule no Rust test covers.
//!
//! Every call is synchronous, on the thread JavaScript runs on: the handles
//! are not `Send`, so they never leave it, and the garbage collector drops
//! a class it finalizes there too. Each method copies what the C ABI lends
//! into owned values before it builds a JavaScript one, so a finalizer that
//! runs while it builds closes nothing a C call is still reading.

use crate::call::{self, Cell, Next, Outcome, Row, Sheet, Sheets};
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

/// One call's result, as the JS SDK receives it.
#[napi(object)]
pub struct JsOutcome {
    /// The C ABI's status code.
    pub status: i32,
    /// The call's value, present only when `status` is `XLSX_GRAFT_OK`.
    pub value: Option<String>,
    /// Why the host failed, present only when `status` is
    /// `XLSX_GRAFT_ERR_IO`.
    pub message: Option<String>,
}

impl From<Outcome> for JsOutcome {
    fn from(outcome: Outcome) -> Self {
        JsOutcome {
            status: outcome.status,
            value: outcome.value,
            message: outcome.message,
        }
    }
}

/// The engine version. A panic in the addon becomes a JavaScript exception
/// rather than an abort.
#[napi(catch_unwind)]
pub fn version() -> JsOutcome {
    call::version().into()
}

/// A workbook opened to read rows from: `Book.openPath(path)` or
/// `Book.openBytes(buffer)`, then `status` before anything else.
///
/// When any call on it or on its rows returns `XLSX_GRAFT_ERR_PANIC` (2),
/// every later call on them but `close` returns it too, as the C ABI's
/// header says: close them.
///
/// Closing it closes every `Rows` opened from it, whose `nextRow()` then
/// returns `XLSX_GRAFT_ERR_CLOSED` (10). A `Rows` holds its book open, so
/// the garbage collector finalizing the book alone closes nothing.
#[napi]
pub struct Book {
    inner: call::Book,
}

#[napi]
impl Book {
    /// Opens the workbook at `path`.
    #[napi(factory, catch_unwind)]
    pub fn open_path(path: String) -> Self {
        Book {
            inner: call::Book::open_path(&path),
        }
    }

    /// Opens the workbook in `data`, copied first.
    #[napi(factory, catch_unwind)]
    pub fn open_bytes(data: Buffer) -> Self {
        Book {
            inner: call::Book::open_bytes(&data),
        }
    }

    /// The open's status.
    #[napi(getter)]
    pub fn status(&self) -> i32 {
        self.inner.status
    }

    /// Why the host failed, when `status` is `XLSX_GRAFT_ERR_IO`.
    #[napi(getter)]
    pub fn message(&self) -> Option<String> {
        self.inner.message.clone()
    }

    /// The sheets.
    #[napi(catch_unwind)]
    pub fn sheets(&self) -> JsSheets {
        self.inner.sheets().into()
    }

    /// The rows of the sheet at `index`, hidden sheets included.
    #[napi(catch_unwind)]
    pub fn rows(&self, index: u32) -> Rows {
        Rows {
            inner: self.inner.rows(index as usize),
        }
    }

    /// The rows of the sheet named `name`, letter case aside.
    #[napi(catch_unwind)]
    pub fn rows_named(&self, name: String) -> Rows {
        Rows {
            inner: self.inner.rows_named(&name),
        }
    }

    /// The diagnostics as JSON, in `value`.
    #[napi(catch_unwind)]
    pub fn diagnostics(&self) -> JsOutcome {
        self.inner.diagnostics().into()
    }

    /// Closes the book; every call after returns `XLSX_GRAFT_ERR_NULL_ARG`.
    #[napi(catch_unwind)]
    pub fn close(&mut self) {
        self.inner.close();
    }
}

/// The rows of one sheet: `status` first, then `nextRow()` until its `row` is
/// absent. They live no longer than their book, as [`Book`] says.
#[napi]
pub struct Rows {
    inner: call::Rows,
}

#[napi]
impl Rows {
    /// The open's status.
    #[napi(getter)]
    pub fn status(&self) -> i32 {
        self.inner.status
    }

    /// The next row.
    #[napi(catch_unwind)]
    pub fn next_row(&mut self) -> JsNext {
        self.inner.next_row().into()
    }

    /// Closes the rows; every call after returns `XLSX_GRAFT_ERR_NULL_ARG`.
    #[napi(catch_unwind)]
    pub fn close(&mut self) {
        self.inner.close();
    }
}

/// The sheets of a book, or the status refusing them.
#[napi(object)]
pub struct JsSheets {
    /// The C ABI's status code.
    pub status: i32,
    /// The sheets, empty unless `status` is `XLSX_GRAFT_OK`.
    pub sheets: Vec<JsSheet>,
}

/// One sheet of a book.
#[napi(object)]
pub struct JsSheet {
    /// Its name.
    pub name: String,
    /// `worksheet`, `chartsheet`, `dialogsheet`, `macrosheet` or `unknown`.
    pub kind: String,
    /// `visible`, `hidden`, `veryHidden` or `unknown`.
    pub state: String,
}

/// What one read of a row came to.
#[napi(object)]
pub struct JsNext {
    /// The C ABI's status code.
    pub status: i32,
    /// Why the host failed, when `status` is `XLSX_GRAFT_ERR_IO`.
    pub message: Option<String>,
    /// The row, absent at the end of the sheet and on a status other than
    /// `XLSX_GRAFT_OK`.
    pub row: Option<JsRow>,
}

/// One row the sheet writes.
#[napi(object)]
pub struct JsRow {
    /// Its number, from 1.
    pub number: u32,
    /// Its cells, from column A to its last.
    pub cells: Vec<JsCell>,
}

/// One cell: its type, as `read --format json` names it, and the fields
/// that type sets.
#[napi(object)]
pub struct JsCell {
    /// `empty`, `string`, `number`, `boolean`, `date`, `time`,
    /// `datetime`, `elapsed`, `error` or `unknown`.
    #[napi(js_name = "type")]
    pub kind: String,
    /// A string's or an error value's text.
    pub text: Option<String>,
    /// A number's value.
    pub number: Option<f64>,
    /// A boolean's value.
    pub boolean: Option<bool>,
    /// `[year, month, day]`.
    pub date: Option<Vec<u32>>,
    /// `[hour, minute, second, millisecond]`.
    pub time: Option<Vec<u32>>,
    /// An elapsed time, in milliseconds.
    pub milliseconds: Option<f64>,
}

impl From<Sheets> for JsSheets {
    fn from(sheets: Sheets) -> Self {
        JsSheets {
            status: sheets.status,
            sheets: sheets.sheets.into_iter().map(JsSheet::from).collect(),
        }
    }
}

impl From<Sheet> for JsSheet {
    fn from(sheet: Sheet) -> Self {
        JsSheet {
            name: sheet.name,
            kind: sheet.kind.to_owned(),
            state: sheet.state.to_owned(),
        }
    }
}

impl From<Next> for JsNext {
    fn from(next: Next) -> Self {
        JsNext {
            status: next.status,
            message: next.message,
            row: next.row.map(JsRow::from),
        }
    }
}

impl From<Row> for JsRow {
    fn from(row: Row) -> Self {
        JsRow {
            number: row.number,
            cells: row.cells.into_iter().map(JsCell::from).collect(),
        }
    }
}

impl From<Cell> for JsCell {
    fn from(cell: Cell) -> Self {
        JsCell {
            kind: cell.kind.to_owned(),
            text: cell.text,
            number: cell.number,
            boolean: cell.boolean,
            date: cell.date.map(Vec::from),
            time: cell.time.map(Vec::from),
            milliseconds: cell.milliseconds,
        }
    }
}
