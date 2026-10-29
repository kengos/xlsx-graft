//! The rows handle's entry points: open a sheet's rows by index or name,
//! read the next row, close it.

use super::{missing, refuse_null, run, text};
use crate::book::{Choice, XlsxGraftBook};
use crate::rows::XlsxGraftRows;
use crate::value::XlsxGraftValue;

/// Opens the rows of the sheet at `index` of the book's list (0 is the
/// first, hidden sheets and chartsheets counted) and writes the rows
/// handle into `*out_rows`.
///
/// Refuses, writing nothing: [`XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE`](crate::XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE),
/// [`XLSX_GRAFT_ERR_SHEET_NOT_FOUND`](crate::XLSX_GRAFT_ERR_SHEET_NOT_FOUND)
/// (no sheet at `index`), [`XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET`](crate::XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET)
/// and [`XLSX_GRAFT_ERR_SHEET_UNREADABLE`](crate::XLSX_GRAFT_ERR_SHEET_UNREADABLE).
///
/// # Safety
///
/// `book` must be null or a book handle not yet closed, and `out_rows`
/// null or at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_rows_open(
    book: *mut XlsxGraftBook,
    index: usize,
    out_rows: *mut *mut XlsxGraftRows,
) -> i32 {
    run(|| {
        refuse_null(book.is_null() || out_rows.is_null())?;
        // SAFETY: `book` is non-null, and a live handle by contract.
        let rows = unsafe { &*book }.rows(Choice::Index(index))?;
        // SAFETY: `out_rows` is non-null, and a writable slot by contract.
        unsafe { out_rows.write(Box::into_raw(Box::new(rows))) };
        Ok(())
    })
}

/// Opens the rows of the sheet named by the UTF-8 `name` (`name_len`
/// bytes), compared as Excel compares sheet names, letter case aside. As
/// [`xlsx_graft_rows_open`], and refuses too with
/// [`XLSX_GRAFT_ERR_SHEET_AMBIGUOUS`](crate::XLSX_GRAFT_ERR_SHEET_AMBIGUOUS)
/// when more than one sheet has the name, and
/// [`XLSX_GRAFT_ERR_INVALID_ARG`](crate::XLSX_GRAFT_ERR_INVALID_ARG) when it
/// is not UTF-8.
///
/// # Safety
///
/// `book` must be null or a book handle not yet closed, `name` null or at
/// `name_len` readable bytes (a null `name` is no bytes when `name_len` is
/// 0, and refused otherwise), and `out_rows` null or at one writable,
/// aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_rows_open_name(
    book: *mut XlsxGraftBook,
    name: *const u8,
    name_len: usize,
    out_rows: *mut *mut XlsxGraftRows,
) -> i32 {
    run(|| {
        refuse_null(book.is_null() || missing(name, name_len) || out_rows.is_null())?;
        // SAFETY: `name` is not missing, and the caller's contract gives it
        // `name_len` bytes for the call.
        let name = unsafe { text(name, name_len) }?;
        // SAFETY: `book` is non-null, and a live handle by contract.
        let rows = unsafe { &*book }.rows(Choice::Name(name))?;
        // SAFETY: `out_rows` is non-null, and a writable slot by contract.
        unsafe { out_rows.write(Box::into_raw(Box::new(rows))) };
        Ok(())
    })
}

/// Reads the next row: writes its number (from 1) into `*out_number`, a
/// pointer to its values from column A to its last cell into
/// `*out_values`, and their count into `*out_len`. The values, and the text
/// they point at, belong to the rows handle and live until the next call
/// on it, or its close.
///
/// At the end of the sheet, or when a problem stopped it (the book's
/// diagnostics say which), it writes 0, null and 0, and does so on every
/// call after. The end is the number 0: a row the sheet writes empty has
/// its number and no values. [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) when the
/// host fails, writing nothing; no row comes after it either.
///
/// # Safety
///
/// `rows` must be null or a rows handle not yet closed, and each out
/// pointer null or at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_rows_next(
    rows: *mut XlsxGraftRows,
    out_number: *mut u32,
    out_values: *mut *const XlsxGraftValue,
    out_len: *mut usize,
) -> i32 {
    run(|| {
        let null = out_number.is_null() || out_values.is_null() || out_len.is_null();
        refuse_null(rows.is_null() || null)?;
        // SAFETY: `rows` is non-null, and a live handle by contract.
        let (number, values, len) = match unsafe { &mut *rows }.next()? {
            Some((number, values)) => (number, values.as_ptr(), values.len()),
            None => (0, std::ptr::null(), 0),
        };
        // SAFETY: the out pointers are non-null, and writable by contract.
        unsafe {
            out_number.write(number);
            out_values.write(values);
            out_len.write(len);
        }
        Ok(())
    })
}

/// Closes the rows handle: the values it lent are gone. A null `rows` does
/// nothing; either way the call returns [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK).
///
/// # Safety
///
/// `rows` must be null or a rows handle not yet closed. It is closed after
/// the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_rows_close(rows: *mut XlsxGraftRows) -> i32 {
    run(|| {
        if !rows.is_null() {
            // SAFETY: a live handle is a `Box` the library leaked, and the
            // caller gives it back only once.
            drop(unsafe { Box::from_raw(rows) });
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests;
