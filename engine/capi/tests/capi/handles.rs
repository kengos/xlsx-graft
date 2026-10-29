//! The book and rows handles, driven end to end as a binding drives them:
//! every entry point once, through the library as its dependents link it.

use crate::workbook::{worksheet, write, xlsx};
use std::ptr;
use xlsx_graft_capi::*;

/// The `len` bytes at `ptr`, as text.
///
/// # Safety
///
/// `ptr` must point at `len` readable bytes of UTF-8.
unsafe fn text(ptr: *const u8, len: usize) -> String {
    // SAFETY: the caller's contract.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The next row of `rows`: its number, and its values' kinds.
fn next(rows: *mut XlsxGraftRows) -> (u32, Vec<u32>) {
    let (mut number, mut values, mut len) = (0, ptr::null(), 0);
    // SAFETY: `rows` is a live handle; the out pointers are writable slots.
    let status =
        unsafe { xlsx_graft_rows_next(rows, &raw mut number, &raw mut values, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    if len == 0 {
        return (number, Vec::new());
    }
    // SAFETY: on OK the values are `len` of them, alive until the next call.
    let values = unsafe { std::slice::from_raw_parts(values, len) };
    (number, values.iter().map(|value| value.kind).collect())
}

#[test]
fn a_workbook_is_read_from_its_path_by_index_and_by_name() {
    let rows = r#"<row r="1"><c r="B1"><v>2</v></c></row>"#;
    let (_dir, path) = write(&xlsx(&[worksheet("Data", rows)]));
    let path = path.to_str().unwrap();
    let mut book = ptr::null_mut();
    // SAFETY: `path` is alive for the call, `book` a writable slot.
    let status = unsafe { xlsx_graft_book_open_path(path.as_ptr(), path.len(), &raw mut book) };
    assert_eq!(status, XLSX_GRAFT_OK);

    let (mut sheets, mut count) = (ptr::null(), 0);
    // SAFETY: `book` is live; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_book_sheets(book, &raw mut sheets, &raw mut count) };
    assert_eq!((status, count), (XLSX_GRAFT_OK, 1));
    // SAFETY: on OK `sheets` points at `count` sheets of the live book.
    let sheet = unsafe { *sheets };
    // SAFETY: a sheet's name is `name_len` bytes of the live book.
    let name = unsafe { text(sheet.name, sheet.name_len) };
    assert_eq!(
        (name.as_str(), sheet.kind),
        ("Data", XLSX_GRAFT_SHEET_WORKSHEET)
    );

    let (mut by_index, mut by_name) = (ptr::null_mut(), ptr::null_mut());
    // SAFETY: `book` is live; the names and slots are alive for the calls.
    let statuses = unsafe {
        [
            xlsx_graft_rows_open(book, 0, &raw mut by_index),
            xlsx_graft_rows_open_name(book, b"data".as_ptr(), 4, &raw mut by_name),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_OK; 2]);
    let empty_then_number = vec![XLSX_GRAFT_VALUE_EMPTY, XLSX_GRAFT_VALUE_NUMBER];
    assert_eq!(next(by_index), (1, empty_then_number));
    assert_eq!(next(by_index), (0, Vec::new()));

    let (mut json, mut len) = (ptr::null(), 0);
    // SAFETY: `book` is live; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_book_diagnostics(book, &raw mut json, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the JSON is `len` bytes of the live book.
    let json = unsafe { text(json, len) };
    assert_eq!(json, r#"{"has_errors":false,"diagnostics":[]}"#);

    // SAFETY: each handle is live, and closed once; the nulls do nothing.
    let statuses = unsafe {
        [
            xlsx_graft_book_close(book),
            xlsx_graft_rows_close(by_index),
            xlsx_graft_rows_close(by_name),
            xlsx_graft_book_close(ptr::null_mut()),
            xlsx_graft_rows_close(ptr::null_mut()),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_OK; 5]);
}

#[test]
fn bytes_that_hold_no_workbook_make_a_book_that_says_so() {
    let mut book = ptr::null_mut();
    // SAFETY: the bytes are alive for the call, `book` a writable slot.
    let status = unsafe { xlsx_graft_book_open_bytes(b"x".as_ptr(), 1, &raw mut book) };
    assert_eq!(status, XLSX_GRAFT_OK);
    let mut rows = ptr::null_mut();
    // SAFETY: `book` is live, `rows` a writable slot.
    let status = unsafe { xlsx_graft_rows_open(book, 0, &raw mut rows) };
    assert_eq!(status, XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE);
    // SAFETY: `book` is live, and closed once.
    assert_eq!(unsafe { xlsx_graft_book_close(book) }, XLSX_GRAFT_OK);
}

#[test]
fn a_file_that_will_not_open_says_why_on_the_same_thread() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.xlsx");
    let missing = missing.to_str().unwrap();
    let mut book = ptr::null_mut();
    // SAFETY: the path is alive for the call, `book` a writable slot.
    let status =
        unsafe { xlsx_graft_book_open_path(missing.as_ptr(), missing.len(), &raw mut book) };
    assert_eq!((status, book), (XLSX_GRAFT_ERR_IO, ptr::null_mut()));
    let (mut message, mut len) = (ptr::null(), 0);
    // SAFETY: the out pointers are writable slots.
    let status = unsafe { xlsx_graft_error_message(&raw mut message, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the message is `len` bytes, alive until the next failure.
    assert!(!unsafe { text(message, len) }.is_empty());
}
