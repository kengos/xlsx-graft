//! [`XlsxGraftBook`]: a workbook opened to read rows from, behind the
//! book handle, and the deciding logic of its entry points in safe Rust;
//! `abi/book.rs` only marshals.
//!
//! A workbook the engine cannot read still makes a book: it holds the
//! diagnostics that say why, and refuses its sheets with
//! [`XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE`]. Only the host failing makes none.
//!
//! The diagnostics are shared with every rows handle the book opens, so
//! they hold what reading those rows found too, as `read`'s end line does.

use crate::message::failed;
use crate::rows::XlsxGraftRows;
use crate::sheet::XlsxGraftSheet;
use crate::status::{
    XLSX_GRAFT_ERR_SHEET_AMBIGUOUS, XLSX_GRAFT_ERR_SHEET_NOT_FOUND,
    XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET, XLSX_GRAFT_ERR_SHEET_UNREADABLE,
    XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE,
};
use serde::Serialize;
use std::cell::RefCell;
use std::io::{Read, Seek};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostics, Record};
use xlsx_graft_package::Limits;
use xlsx_graft_read::{Book, ReadLimits, SheetKind, SheetLookup};
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
    diagnostics: Rc<RefCell<Diagnostics>>,
    /// The diagnostics as JSON, as last lent out.
    json: String,
}

/// Which sheet to open the rows of.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Choice<'a> {
    /// The sheet at this index of the list, hidden ones included.
    Index(usize),
    /// The sheet of this name, letter case aside.
    Name(&'a str),
}

/// `xlsx_graft_book_diagnostics`'s JSON.
#[derive(Serialize)]
struct Report<'a> {
    /// Whether any diagnostic is an error.
    has_errors: bool,
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
        Ok(Self {
            sheets: sheets.map(XlsxGraftSheet::of).collect(),
            book,
            diagnostics: Rc::new(RefCell::new(diagnostics)),
            json: String::new(),
        })
    }

    /// The sheets, in the order the workbook lists them.
    pub(crate) fn sheets(&self) -> Result<&[XlsxGraftSheet], i32> {
        self.book
            .as_ref()
            .ok_or(XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE)?;
        Ok(&self.sheets)
    }

    /// A reader of the rows of the sheet `choice` names, or the status
    /// saying why there is none.
    pub(crate) fn rows(&self, choice: Choice<'_>) -> Result<XlsxGraftRows, i32> {
        let book = self
            .book
            .as_ref()
            .ok_or(XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE)?;
        let index = match choice {
            Choice::Index(index) => index,
            Choice::Name(name) => match book.sheet(name) {
                SheetLookup::Found(index) => index,
                SheetLookup::Missing => return Err(XLSX_GRAFT_ERR_SHEET_NOT_FOUND),
                SheetLookup::Duplicate => return Err(XLSX_GRAFT_ERR_SHEET_AMBIGUOUS),
            },
        };
        let sheet = book
            .sheets()
            .get(index)
            .ok_or(XLSX_GRAFT_ERR_SHEET_NOT_FOUND)?;
        match sheet.part().map(|part| part.kind()) {
            Some(SheetKind::Worksheet) | None => {}
            Some(_) => return Err(XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET),
        }
        let rows = book.rows(index).ok_or(XLSX_GRAFT_ERR_SHEET_UNREADABLE)?;
        Ok(XlsxGraftRows::new(rows, Rc::clone(&self.diagnostics)))
    }

    /// Every problem found so far, in the book and in the rows read from
    /// it, as JSON: `{"has_errors": B, "diagnostics": [...]}`, each
    /// diagnostic as every host prints it.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) if the JSON cannot
    /// be written, its message kept.
    pub(crate) fn diagnostics(&mut self) -> Result<&str, i32> {
        // As in `XlsxGraftRows::next`, nothing else borrows them now.
        let diagnostics = self.diagnostics.try_borrow().map_err(failed)?;
        let has_errors = diagnostics.has_errors();
        let records = diagnostics.clone().finish();
        drop(diagnostics);
        let report = Report {
            has_errors,
            diagnostics: &records,
        };
        self.json = serde_json::to_string(&report).map_err(failed)?;
        Ok(&self.json)
    }
}

#[cfg(test)]
mod tests;
