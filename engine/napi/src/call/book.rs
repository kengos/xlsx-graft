//! [`Book`]: a C ABI book handle, owned, and what its calls lend copied
//! out.
//!
//! A book whose open failed holds no handle: every call on it then passes
//! the C ABI a null handle, which it refuses with `XLSX_GRAFT_ERR_NULL_ARG`,
//! as it does for a book closed. So a JavaScript object closed twice, or
//! used after its close, is refused rather than undefined behaviour.
//!
//! `XLSX_GRAFT_ERR_PANIC` is passed through like any status. What the
//! header asks after one, closing the book and every rows handle of it, is
//! the JS SDK's to do, as it is every other SDK's.

use super::rows::Rows;
use super::{Outcome, message, owned, read};
use xlsx_graft_capi::{
    XLSX_GRAFT_OK, XLSX_GRAFT_SHEET_CHARTSHEET, XLSX_GRAFT_SHEET_DIALOGSHEET,
    XLSX_GRAFT_SHEET_MACROSHEET, XLSX_GRAFT_SHEET_WORKSHEET, XLSX_GRAFT_STATE_HIDDEN,
    XLSX_GRAFT_STATE_VERY_HIDDEN, XLSX_GRAFT_STATE_VISIBLE, XlsxGraftBook, XlsxGraftSheet,
    xlsx_graft_book_close, xlsx_graft_book_diagnostics, xlsx_graft_book_open_bytes,
    xlsx_graft_book_open_path, xlsx_graft_book_sheets,
};

/// A workbook opened through the C ABI, closed when dropped.
///
/// Not [`Send`]: the handle is used on the thread JavaScript runs on, and
/// dropped there too, when the garbage collector finalizes its object.
#[derive(Debug)]
pub struct Book {
    handle: *mut XlsxGraftBook,
    /// The open's status.
    pub status: i32,
    /// Why the host failed, when `status` is `XLSX_GRAFT_ERR_IO`.
    pub message: Option<String>,
}

/// The sheets of a book, or the status refusing them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheets {
    /// The C ABI's status.
    pub status: i32,
    /// The sheets in the workbook's order, empty unless `status` is
    /// `XLSX_GRAFT_OK`.
    pub sheets: Vec<Sheet>,
}

/// One sheet of a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// Its name.
    pub name: String,
    /// `worksheet`, `chartsheet`, `dialogsheet`, `macrosheet`, or
    /// `unknown` when its part is not known.
    pub kind: &'static str,
    /// `visible`, `hidden`, `veryHidden`, or `unknown` for a state newer
    /// than this addon.
    pub state: &'static str,
}

impl Book {
    /// Opens the workbook at `path`.
    #[must_use]
    pub fn open_path(path: &str) -> Self {
        let mut handle = std::ptr::null_mut();
        // SAFETY: `path` is alive for the call, `handle` a writable slot.
        let status =
            unsafe { xlsx_graft_book_open_path(path.as_ptr(), path.len(), &raw mut handle) };
        Self::opened(status, handle)
    }

    /// Opens the workbook in `data`, which the C ABI copies.
    #[must_use]
    pub fn open_bytes(data: &[u8]) -> Self {
        let mut handle = std::ptr::null_mut();
        // SAFETY: `data` is alive for the call, `handle` a writable slot.
        let status =
            unsafe { xlsx_graft_book_open_bytes(data.as_ptr(), data.len(), &raw mut handle) };
        Self::opened(status, handle)
    }

    /// The book an open returned. `handle` is kept only on
    /// `XLSX_GRAFT_OK`: on any other status the C ABI says to read no
    /// out-parameter, so it is never closed.
    fn opened(status: i32, handle: *mut XlsxGraftBook) -> Self {
        Self {
            handle: owned(status, handle),
            status,
            message: message(status),
        }
    }

    /// The sheets, copied out.
    #[must_use]
    pub fn sheets(&self) -> Sheets {
        let (mut sheets, mut len) = (std::ptr::null(), 0);
        // SAFETY: the handle is live or null; the out pointers are
        // writable slots.
        let status = unsafe { xlsx_graft_book_sheets(self.handle, &raw mut sheets, &raw mut len) };
        // No sheets lend no pointer to read.
        if status != XLSX_GRAFT_OK || len == 0 {
            return Sheets {
                status,
                sheets: Vec::new(),
            };
        }
        // SAFETY: on OK the library lent `len` sheets, alive while the book
        // is, which `self` keeps open for the copy.
        let lent = unsafe { std::slice::from_raw_parts(sheets, len) };
        Sheets {
            status,
            // SAFETY: each sheet's name is alive while the book is.
            sheets: lent
                .iter()
                .map(|sheet| unsafe { Sheet::of(sheet) })
                .collect(),
        }
    }

    /// The rows of the sheet at `index`.
    #[must_use]
    pub fn rows(&self, index: usize) -> Rows {
        Rows::open_index(self.handle, index)
    }

    /// The rows of the sheet named `name`, letter case aside.
    #[must_use]
    pub fn rows_named(&self, name: &str) -> Rows {
        Rows::open_name(self.handle, name)
    }

    /// The diagnostics' JSON, copied out.
    #[must_use]
    pub fn diagnostics(&self) -> Outcome {
        let (mut ptr, mut len) = (std::ptr::null(), 0);
        // SAFETY: the handle is live or null; the out pointers are
        // writable slots.
        let status =
            unsafe { xlsx_graft_book_diagnostics(self.handle, &raw mut ptr, &raw mut len) };
        // SAFETY: on OK the library lent `len` UTF-8 bytes, alive until the
        // next call on the book.
        unsafe { read(status, ptr, len) }
    }

    /// Closes the book; every call after is refused as a null handle's.
    pub fn close(&mut self) {
        let handle = std::mem::replace(&mut self.handle, std::ptr::null_mut());
        // SAFETY: the handle is live or null, and nulled above so it is
        // closed once. A close cannot fail but by a panic, after which the
        // book is closed as far as it will be.
        unsafe { xlsx_graft_book_close(handle) };
    }
}

impl Drop for Book {
    fn drop(&mut self) {
        self.close();
    }
}

impl Sheet {
    /// `sheet`, copied out.
    ///
    /// # Safety
    ///
    /// `sheet.name` must point at `sheet.name_len` bytes alive for the call.
    unsafe fn of(sheet: &XlsxGraftSheet) -> Self {
        // SAFETY: the caller's contract.
        let name = unsafe { std::slice::from_raw_parts(sheet.name, sheet.name_len) };
        let kind = match sheet.kind {
            XLSX_GRAFT_SHEET_WORKSHEET => "worksheet",
            XLSX_GRAFT_SHEET_CHARTSHEET => "chartsheet",
            XLSX_GRAFT_SHEET_DIALOGSHEET => "dialogsheet",
            XLSX_GRAFT_SHEET_MACROSHEET => "macrosheet",
            _ => "unknown",
        };
        let state = match sheet.state {
            XLSX_GRAFT_STATE_VISIBLE => "visible",
            XLSX_GRAFT_STATE_HIDDEN => "hidden",
            XLSX_GRAFT_STATE_VERY_HIDDEN => "veryHidden",
            _ => "unknown",
        };
        Self {
            name: String::from_utf8_lossy(name).into_owned(),
            kind,
            state,
        }
    }
}

#[cfg(test)]
mod tests;
