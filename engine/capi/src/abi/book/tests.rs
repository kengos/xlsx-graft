//! Unit tests for the book handle's entry points, driven as a binding
//! drives them: what each writes, and what each refuses.

use super::*;
use crate::message::lent;
use crate::status::*;
use crate::tests::workbook::{worksheet, write, xlsx};
use std::ptr;

/// The handle of a book opened from `bytes`.
fn open_bytes(bytes: &[u8]) -> *mut XlsxGraftBook {
    let mut book = ptr::null_mut();
    // SAFETY: `bytes` is alive for the call, `book` a writable slot.
    let status = unsafe { xlsx_graft_book_open_bytes(bytes.as_ptr(), bytes.len(), &raw mut book) };
    assert_eq!(status, XLSX_GRAFT_OK);
    book
}

/// The status opening `path` returns, and the handle it wrote, if any.
fn open_path(path: &[u8]) -> (i32, *mut XlsxGraftBook) {
    let mut book = ptr::null_mut();
    // SAFETY: `path` is alive for the call, `book` a writable slot.
    let status = unsafe { xlsx_graft_book_open_path(path.as_ptr(), path.len(), &raw mut book) };
    (status, book)
}

/// Closes `book`, asserting the status.
fn close(book: *mut XlsxGraftBook) {
    // SAFETY: `book` is a live handle, or null.
    assert_eq!(unsafe { xlsx_graft_book_close(book) }, XLSX_GRAFT_OK);
}

/// The sheets' count `book` lists, or the status refusing them.
fn sheet_count(book: *mut XlsxGraftBook) -> Result<usize, i32> {
    let (mut sheets, mut len) = (ptr::null(), usize::MAX);
    // SAFETY: `book` is a live handle; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_book_sheets(book, &raw mut sheets, &raw mut len) };
    if status != XLSX_GRAFT_OK {
        assert_eq!(len, usize::MAX, "a refusal writes nothing");
        return Err(status);
    }
    Ok(len)
}

#[test]
fn a_workbook_opens_from_a_path_and_from_bytes() {
    let bytes = xlsx(&[worksheet("Data", ""), worksheet("More", "")]);
    let (_dir, path) = write(&bytes);
    let (status, from_path) = open_path(path.to_str().unwrap().as_bytes());
    assert_eq!(status, XLSX_GRAFT_OK);
    let from_bytes = open_bytes(&bytes);
    assert_eq!(
        (sheet_count(from_path), sheet_count(from_bytes)),
        (Ok(2), Ok(2))
    );
    close(from_path);
    close(from_bytes);
}

#[test]
fn a_file_that_cannot_be_opened_is_the_host_failing_with_why() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.xlsx");
    let (status, book) = open_path(missing.to_str().unwrap().as_bytes());
    assert_eq!((status, book), (XLSX_GRAFT_ERR_IO, ptr::null_mut()));
    let (ptr, len) = lent();
    // SAFETY: the message is alive until the next failure on this thread.
    let message = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(!message.is_empty());
}

#[test]
fn a_directory_is_the_host_failing_too() {
    let dir = tempfile::tempdir().unwrap();
    let (status, book) = open_path(dir.path().to_str().unwrap().as_bytes());
    assert_eq!((status, book), (XLSX_GRAFT_ERR_IO, ptr::null_mut()));
}

#[test]
fn a_path_that_is_not_utf8_is_an_invalid_argument() {
    let (status, book) = open_path(b"book\xff.xlsx");
    assert_eq!(
        (status, book),
        (XLSX_GRAFT_ERR_INVALID_ARG, ptr::null_mut())
    );
}

#[test]
fn bytes_that_hold_no_workbook_open_a_book_that_refuses_its_sheets() {
    let book = open_bytes(b"");
    assert_eq!(sheet_count(book), Err(XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE));
    close(book);
}

#[test]
fn the_diagnostics_are_lent_as_json() {
    let book = open_bytes(&xlsx(&[worksheet("Data", "")]));
    let (mut json, mut len) = (ptr::null(), 0);
    // SAFETY: `book` is a live handle; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_book_diagnostics(book, &raw mut json, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the JSON is `len` bytes, alive until the next call.
    let json = unsafe { std::slice::from_raw_parts(json, len) };
    assert_eq!(
        json,
        br#"{"has_errors":false,"error":null,"diagnostics":[]}"#
    );
    close(book);
}

#[test]
fn every_null_pointer_is_refused_and_writes_nothing() {
    let book = open_bytes(&xlsx(&[worksheet("Data", "")]));
    let data = b"x";
    let mut out = ptr::null_mut();
    let (mut lent, mut len) = (ptr::null(), usize::MAX);
    let (mut sheets, null_book) = (ptr::null(), ptr::null_mut());
    // SAFETY: every pointer is null or a writable slot of its type; the
    // nulls are what is tested.
    let statuses = unsafe {
        [
            xlsx_graft_book_open_path(ptr::null(), 1, &raw mut out),
            xlsx_graft_book_open_path(data.as_ptr(), 1, ptr::null_mut()),
            xlsx_graft_book_open_bytes(ptr::null(), 1, &raw mut out),
            xlsx_graft_book_open_bytes(data.as_ptr(), 1, ptr::null_mut()),
            xlsx_graft_book_sheets(null_book, &raw mut sheets, &raw mut len),
            xlsx_graft_book_sheets(book, ptr::null_mut(), &raw mut len),
            xlsx_graft_book_sheets(book, &raw mut sheets, ptr::null_mut()),
            xlsx_graft_book_diagnostics(null_book, &raw mut lent, &raw mut len),
            xlsx_graft_book_diagnostics(book, ptr::null_mut(), &raw mut len),
            xlsx_graft_book_diagnostics(book, &raw mut lent, ptr::null_mut()),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_ERR_NULL_ARG; 10]);
    assert_eq!(
        (out, sheets, lent, len),
        (ptr::null_mut(), ptr::null(), ptr::null(), usize::MAX)
    );
    close(book);
}

#[test]
fn a_null_pointer_of_no_bytes_is_an_empty_input() {
    let mut book = ptr::null_mut();
    // SAFETY: a null pointer of no bytes is read as none; `book` is a
    // writable slot.
    let status = unsafe { xlsx_graft_book_open_bytes(ptr::null(), 0, &raw mut book) };
    assert_eq!(status, XLSX_GRAFT_OK);
    assert_eq!(sheet_count(book), Err(XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE));
    close(book);
    let mut none = ptr::null_mut();
    // SAFETY: as above; the empty path names no file.
    let status = unsafe { xlsx_graft_book_open_path(ptr::null(), 0, &raw mut none) };
    assert_eq!((status, none), (XLSX_GRAFT_ERR_IO, ptr::null_mut()));
}

#[test]
fn the_message_of_a_file_that_will_not_open_names_its_path() {
    let (status, _) = open_path(b"/no/such/book.xlsx");
    assert_eq!(status, XLSX_GRAFT_ERR_IO);
    let (ptr, len) = lent();
    // SAFETY: the message is alive until the next failure on this thread.
    let message = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(String::from_utf8_lossy(message).starts_with("/no/such/book.xlsx: "));
}

#[test]
fn closing_a_null_book_does_nothing() {
    close(ptr::null_mut());
}
