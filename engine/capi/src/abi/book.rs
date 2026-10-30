//! The book handle's entry points: open a workbook from a path or bytes,
//! list its sheets, lend its diagnostics, close it.

use super::{bytes, missing, poisoning, refuse_null, run, run_on, text};
use crate::book::XlsxGraftBook;
use crate::message::failed;
use crate::shared::Shared;
use crate::sheet::XlsxGraftSheet;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::rc::Rc;

/// Opens the workbook at the UTF-8 path `path` (`path_len` bytes, not
/// NUL-terminated) and writes its book handle into `*out_book`.
///
/// Returns [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK) whenever the engine read
/// the file, even when it found no workbook in it: the book's sheets then
/// say [`XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE`](crate::XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE)
/// and its diagnostics why. [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO)
/// when the file cannot be opened or read, or a temporary file written,
/// [`XLSX_GRAFT_ERR_INVALID_ARG`](crate::XLSX_GRAFT_ERR_INVALID_ARG) when
/// the path is not UTF-8, [`XLSX_GRAFT_ERR_NULL_ARG`](crate::XLSX_GRAFT_ERR_NULL_ARG)
/// when a pointer is null; none of them writes `*out_book`.
///
/// # Safety
///
/// `path` must be null or point at `path_len` readable bytes (a null
/// `path` is no bytes when `path_len` is 0, and refused otherwise), and
/// `out_book` be null or point at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_book_open_path(
    path: *const u8,
    path_len: usize,
    out_book: *mut *mut XlsxGraftBook,
) -> i32 {
    run(|| {
        refuse_null(missing(path, path_len) || out_book.is_null())?;
        // SAFETY: `path` is not missing, and the caller's contract gives it
        // `path_len` bytes for the call.
        let path = unsafe { text(path, path_len) }?;
        let file = File::open(path).map_err(|error| failed(format!("{path}: {error}")))?;
        let book = XlsxGraftBook::open(Box::new(BufReader::new(file)))?;
        // SAFETY: `out_book` is non-null, and a writable slot by contract.
        unsafe { out_book.write(Box::into_raw(Box::new(book))) };
        Ok(())
    })
}

/// Opens the workbook in the `len` bytes at `data`, copied first, and
/// writes its book handle into `*out_book`. As
/// [`xlsx_graft_book_open_path`], but for the path.
///
/// # Safety
///
/// `data` must be null or point at `len` readable bytes (a null `data` is
/// no bytes when `len` is 0, and refused otherwise), and `out_book` be null
/// or point at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_book_open_bytes(
    data: *const u8,
    len: usize,
    out_book: *mut *mut XlsxGraftBook,
) -> i32 {
    run(|| {
        refuse_null(missing(data, len) || out_book.is_null())?;
        // SAFETY: `data` is not missing, and the caller's contract gives it
        // `len` bytes for the call.
        let copy = unsafe { bytes(data, len) }.to_vec();
        let book = XlsxGraftBook::open(Box::new(Cursor::new(copy)))?;
        // SAFETY: `out_book` is non-null, and a writable slot by contract.
        unsafe { out_book.write(Box::into_raw(Box::new(book))) };
        Ok(())
    })
}

/// Writes the book's sheets, in the order the workbook lists them: a
/// pointer to the first into `*out_sheets` and their count into
/// `*out_len`. The array and the names belong to the book and live until it
/// is closed.
///
/// [`XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE`](crate::XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE)
/// when the workbook cannot be read.
///
/// # Safety
///
/// `book` must be null or a book handle not yet closed, and each out
/// pointer null or at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_book_sheets(
    book: *mut XlsxGraftBook,
    out_sheets: *mut *const XlsxGraftSheet,
    out_len: *mut usize,
) -> i32 {
    // SAFETY: `book` is null or a live handle by contract.
    let shared = unsafe { shared(book) };
    run_on(shared, || {
        refuse_null(book.is_null() || out_sheets.is_null() || out_len.is_null())?;
        // SAFETY: `book` is non-null, and a live handle by contract.
        let sheets = unsafe { &*book }.sheets()?;
        // SAFETY: the out pointers are non-null, and writable by contract.
        unsafe {
            out_sheets.write(sheets.as_ptr());
            out_len.write(sheets.len());
        }
        Ok(())
    })
}

/// Writes every problem found so far, in the book and in the rows read
/// from it, as UTF-8 JSON: a pointer into `*out_ptr` and the length into
/// `*out_len`. The bytes are not NUL-terminated, belong to the book, and
/// live until the next call of this function on it, or its close.
///
/// # Safety
///
/// `book` must be null or a book handle not yet closed, and each out
/// pointer null or at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_book_diagnostics(
    book: *mut XlsxGraftBook,
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    // SAFETY: `book` is null or a live handle by contract.
    let shared = unsafe { shared(book) };
    run_on(shared, || {
        refuse_null(book.is_null() || out_ptr.is_null() || out_len.is_null())?;
        // SAFETY: `book` is non-null, and a live handle by contract.
        let json = unsafe { &mut *book }.diagnostics()?;
        // SAFETY: the out pointers are non-null, and writable by contract.
        unsafe {
            out_ptr.write(json.as_ptr());
            out_len.write(json.len());
        }
        Ok(())
    })
}

/// Closes the book, and the rows of every rows handle opened from it,
/// which answer [`XLSX_GRAFT_ERR_CLOSED`](crate::XLSX_GRAFT_ERR_CLOSED)
/// after: everything they lent is gone. A null `book` does nothing; either
/// way the call returns [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK).
///
/// # Safety
///
/// `book` must be null or a book handle not yet closed. It is closed after
/// the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_book_close(book: *mut XlsxGraftBook) -> i32 {
    // SAFETY: `book` is null or a live handle by contract.
    let shared = unsafe { shared(book) };
    let status = run(|| {
        if !book.is_null() {
            // SAFETY: a live handle is a `Box` the library leaked, and the
            // caller gives it back only once.
            drop(unsafe { Box::from_raw(book) });
        }
        Ok(())
    });
    poisoning(shared, status)
}

/// What `book` shares with its rows, or `None` for a null `book`.
///
/// # Safety
///
/// `book` must be null or a book handle not yet closed.
pub(super) unsafe fn shared(book: *mut XlsxGraftBook) -> Option<Rc<Shared>> {
    // SAFETY: the caller's contract.
    (!book.is_null()).then(|| Rc::clone(unsafe { &*book }.shared()))
}

#[cfg(test)]
mod tests;
