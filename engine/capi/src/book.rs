//! [`XlsxGraftBook`]: a workbook opened to read rows from, behind the
//! book handle, and the deciding logic of its entry points in safe Rust;
//! `abi/book.rs` only marshals.
//!
//! A workbook the engine cannot read still makes a book: it holds the
//! diagnostics that say why, and refuses its sheets with
//! [`XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE`]. Only the host failing makes none.
//!
//! The diagnostics are shared with every rows handle the book opens
//! ([`Shared`]), so they hold what reading those rows found too, as
//! `read`'s end line does, and so does the first failure: a sheet refused
//! as `read` refuses it, or the host failing. Closing the book closes those
//! rows handles' rows.

use crate::message::failed;
use crate::rows::XlsxGraftRows;
use crate::shared::{Failure, Shared};
use crate::sheet::XlsxGraftSheet;
use crate::status::{
    XLSX_GRAFT_ERR_SHEET_AMBIGUOUS, XLSX_GRAFT_ERR_SHEET_NOT_FOUND,
    XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET, XLSX_GRAFT_ERR_SHEET_UNREADABLE,
    XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE,
};
use serde::Serialize;
use std::io::{Read, Seek};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostics, Record};
use xlsx_graft_package::Limits;
use xlsx_graft_read::{Book, ReadLimits, Refusal, SheetChoice};
use xlsx_graft_spool::TempFiles;

/// What a book reads from: a file or a copy of the caller's bytes.
pub(crate) trait Source: Read + Seek {}

impl<T: Read + Seek> Source for T {}

/// The reader every book and its rows share the type of.
pub(crate) type Reader = Box<dyn Source>;

/// A workbook opened through the C ABI.
pub struct XlsxGraftBook {
    /// `None` when the workbook cannot be read.
    book: Option<Book<Reader>>,
    /// The sheets, their names lent from `book`.
    sheets: Vec<XlsxGraftSheet>,
    shared: Rc<Shared>,
    /// The diagnostics as JSON, as last lent out.
    json: String,
}

/// `xlsx_graft_book_diagnostics`'s JSON.
#[derive(Serialize)]
struct Report<'a> {
    /// Whether any diagnostic is an error.
    has_errors: bool,
    /// Why the book or its rows stopped short, first.
    error: Option<Failure>,
    diagnostics: &'a [Record],
}

impl XlsxGraftBook {
    /// Opens the workbook `reader` reads, under the default limits, its
    /// shared strings spilling to a temporary file.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) when the host fails,
    /// its message kept.
    pub(crate) fn open(reader: Reader) -> Result<Self, i32> {
        let mut diagnostics = Diagnostics::new();
        let (limits, read_limits) = (Limits::default(), ReadLimits::default());
        let mut spools = TempFiles::new();
        let book = Book::open(reader, &limits, &read_limits, &mut spools, &mut diagnostics);
        let book = book.map_err(failed)?;
        let sheets = book.iter().flat_map(Book::sheets);
        let opened = Self {
            sheets: sheets.map(XlsxGraftSheet::of).collect(),
            book,
            shared: Rc::new(Shared::new(diagnostics)),
            json: String::new(),
        };
        // A workbook that cannot be read is the book's error from the
        // start, as `read`'s end line says it.
        if opened.book.is_none() {
            opened.refuse(&Refusal::WorkbookUnreadable);
        }
        Ok(opened)
    }

    /// What the book shares with its rows.
    pub(crate) fn shared(&self) -> &Rc<Shared> {
        &self.shared
    }

    /// The sheets, in the order the workbook lists them.
    pub(crate) fn sheets(&self) -> Result<&[XlsxGraftSheet], i32> {
        self.readable()?;
        Ok(&self.sheets)
    }

    /// A reader of the rows of the sheet `choice` names, or the status
    /// saying why there is none, kept as the book's failure.
    pub(crate) fn rows(&self, choice: SheetChoice<'_>) -> Result<XlsxGraftRows, i32> {
        let book = self.readable()?;
        let rows = book
            .choose(choice)
            .and_then(|(index, _)| book.open_rows(index))
            .map_err(|refusal| self.refuse(&refusal))?;
        XlsxGraftRows::new(rows, Rc::clone(&self.shared))
    }

    /// Every problem found so far, in the book and in the rows read from
    /// it, as JSON: `{"has_errors": B, "error": E, "diagnostics": [...]}`,
    /// `error` the first failure or null, each diagnostic as every host
    /// prints it.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) if the JSON cannot
    /// be written, its message kept.
    pub(crate) fn diagnostics(&mut self) -> Result<&str, i32> {
        // Nothing else borrows them while a call lasts: a handle is used
        // from one thread at a time, and no call re-enters another.
        let diagnostics = self.shared.diagnostics.try_borrow().map_err(failed)?;
        let has_errors = diagnostics.has_errors();
        let records = diagnostics.clone().finish();
        drop(diagnostics);
        let report = Report {
            has_errors,
            error: self.shared.failure(),
            diagnostics: &records,
        };
        self.json = serde_json::to_string(&report).map_err(failed)?;
        Ok(&self.json)
    }

    /// The workbook, or [`XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE`], kept.
    fn readable(&self) -> Result<&Book<Reader>, i32> {
        (self.book.as_ref()).ok_or_else(|| self.refuse(&Refusal::WorkbookUnreadable))
    }

    /// Keeps `refusal` as the book's failure, and returns its status.
    fn refuse(&self, refusal: &Refusal) -> i32 {
        self.shared.fail(refusal.code(), refusal.to_string());
        match refusal {
            Refusal::WorkbookUnreadable => XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE,
            Refusal::NoSuchName(_) | Refusal::NoSuchIndex { .. } => XLSX_GRAFT_ERR_SHEET_NOT_FOUND,
            Refusal::Duplicate(_) => XLSX_GRAFT_ERR_SHEET_AMBIGUOUS,
            Refusal::NotWorksheet { .. } => XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET,
            Refusal::SheetUnreadable(_) => XLSX_GRAFT_ERR_SHEET_UNREADABLE,
        }
    }
}

/// Closing the book closes the rows opened from it.
impl Drop for XlsxGraftBook {
    fn drop(&mut self) {
        self.shared.close_rows();
    }
}

#[cfg(test)]
mod tests;
