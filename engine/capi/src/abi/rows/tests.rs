//! Unit tests for the rows handle's entry points, driven as a binding
//! drives them: opening by index and by name, reading to the end, closing
//! in either order, and what each refuses.

use super::*;
use crate::abi::book::{xlsx_graft_book_close, xlsx_graft_book_open_bytes};
use crate::status::*;
use crate::tests::workbook::{worksheet, xlsx};
use crate::value::XLSX_GRAFT_VALUE_STRING;
use std::ptr;

/// The handle of a book of one worksheet, `Data`, of rows 2 and 4.
fn book() -> *mut XlsxGraftBook {
    let rows = r#"<row r="2"><c r="B2" t="s"><v>1</v></c></row><row r="4"/>"#;
    let bytes = xlsx(&[worksheet("Data", rows)]);
    let mut book = ptr::null_mut();
    // SAFETY: `bytes` is alive for the call, `book` a writable slot.
    let status = unsafe { xlsx_graft_book_open_bytes(bytes.as_ptr(), bytes.len(), &raw mut book) };
    assert_eq!(status, XLSX_GRAFT_OK);
    book
}

/// The status opening the rows of the sheet named `name` returns, and the
/// handle it wrote, if any.
fn open_name(book: *mut XlsxGraftBook, name: &[u8]) -> (i32, *mut XlsxGraftRows) {
    let mut rows = ptr::null_mut();
    // SAFETY: `book` is a live handle, `name` alive for the call, `rows` a
    // writable slot.
    let status =
        unsafe { xlsx_graft_rows_open_name(book, name.as_ptr(), name.len(), &raw mut rows) };
    (status, rows)
}

/// The next row's number and the kinds and texts of its values.
fn next(rows: *mut XlsxGraftRows) -> (u32, Vec<(u32, String)>) {
    let (mut number, mut values, mut len) = (u32::MAX, ptr::dangling(), usize::MAX);
    // SAFETY: `rows` is a live handle; the out pointers are writable slots.
    let status =
        unsafe { xlsx_graft_rows_next(rows, &raw mut number, &raw mut values, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    if number == 0 {
        assert_eq!((values, len), (ptr::null(), 0));
        return (0, Vec::new());
    }
    // SAFETY: on OK the values are `len` of them, alive until the next call.
    let values = unsafe { std::slice::from_raw_parts(values, len) };
    let values = (values.iter())
        .map(|value| {
            let text = if value.text.is_null() {
                &[][..]
            } else {
                // SAFETY: a value's text is alive until the next call.
                unsafe { std::slice::from_raw_parts(value.text, value.text_len) }
            };
            (value.kind, String::from_utf8(text.to_vec()).unwrap())
        })
        .collect();
    (number, values)
}

/// Closes `rows`, asserting the status.
fn close(rows: *mut XlsxGraftRows) {
    // SAFETY: `rows` is a live handle, or null.
    assert_eq!(unsafe { xlsx_graft_rows_close(rows) }, XLSX_GRAFT_OK);
}

#[test]
fn the_rows_are_read_to_the_end_then_say_so_again() {
    let book = book();
    let mut rows = ptr::null_mut();
    // SAFETY: `book` is a live handle, `rows` a writable slot.
    let status = unsafe { xlsx_graft_rows_open(book, 0, &raw mut rows) };
    assert_eq!(status, XLSX_GRAFT_OK);
    let b = (XLSX_GRAFT_VALUE_STRING, "b".to_owned());
    assert_eq!(next(rows), (2, vec![(0, String::new()), b]));
    assert_eq!(next(rows), (4, Vec::new()));
    assert_eq!(next(rows), (0, Vec::new()));
    assert_eq!(next(rows), (0, Vec::new()));
    close(rows);
    // SAFETY: `book` is a live handle.
    assert_eq!(unsafe { xlsx_graft_book_close(book) }, XLSX_GRAFT_OK);
}

#[test]
fn the_book_may_be_closed_before_its_rows() {
    let book = book();
    let (status, rows) = open_name(book, "DATA".as_bytes());
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: `book` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_book_close(book) }, XLSX_GRAFT_OK);
    assert_eq!(next(rows).0, 2);
    close(rows);
}

#[test]
fn a_refused_sheet_writes_no_handle() {
    let book = book();
    let mut rows = ptr::null_mut();
    // SAFETY: `book` is a live handle, `rows` a writable slot.
    let status = unsafe { xlsx_graft_rows_open(book, 1, &raw mut rows) };
    assert_eq!(
        (status, rows),
        (XLSX_GRAFT_ERR_SHEET_NOT_FOUND, ptr::null_mut())
    );
    // SAFETY: `book` is a live handle; a null name of no bytes is the
    // empty name, and `rows` a writable slot.
    let empty = unsafe { xlsx_graft_rows_open_name(book, ptr::null(), 0, &raw mut rows) };
    assert_eq!(
        (empty, rows),
        (XLSX_GRAFT_ERR_SHEET_NOT_FOUND, ptr::null_mut())
    );
    let refusals = [open_name(book, b"Nope"), open_name(book, b"Data\xff")];
    let expected = [
        (XLSX_GRAFT_ERR_SHEET_NOT_FOUND, ptr::null_mut()),
        (XLSX_GRAFT_ERR_INVALID_ARG, ptr::null_mut()),
    ];
    assert_eq!(refusals, expected);
    // SAFETY: `book` is a live handle.
    assert_eq!(unsafe { xlsx_graft_book_close(book) }, XLSX_GRAFT_OK);
}

#[test]
fn every_null_pointer_is_refused_and_writes_nothing() {
    let book = book();
    let (mut rows, null_book) = (ptr::null_mut(), ptr::null_mut());
    let name = b"Data";
    let (mut number, mut values, mut len) = (u32::MAX, ptr::null(), usize::MAX);
    // SAFETY: every pointer is null or a writable slot of its type; the
    // nulls are what is tested.
    let statuses = unsafe {
        [
            xlsx_graft_rows_open(null_book, 0, &raw mut rows),
            xlsx_graft_rows_open(book, 0, ptr::null_mut()),
            xlsx_graft_rows_open_name(null_book, name.as_ptr(), 4, &raw mut rows),
            xlsx_graft_rows_open_name(book, ptr::null(), 4, &raw mut rows),
            xlsx_graft_rows_open_name(book, name.as_ptr(), 4, ptr::null_mut()),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_ERR_NULL_ARG; 5]);
    assert_eq!(rows, ptr::null_mut());
    let (status, open) = open_name(book, name);
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: as above.
    let statuses = unsafe {
        [
            xlsx_graft_rows_next(
                ptr::null_mut(),
                &raw mut number,
                &raw mut values,
                &raw mut len,
            ),
            xlsx_graft_rows_next(open, ptr::null_mut(), &raw mut values, &raw mut len),
            xlsx_graft_rows_next(open, &raw mut number, ptr::null_mut(), &raw mut len),
            xlsx_graft_rows_next(open, &raw mut number, &raw mut values, ptr::null_mut()),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_ERR_NULL_ARG; 4]);
    assert_eq!((number, values, len), (u32::MAX, ptr::null(), usize::MAX));
    assert_eq!(next(open).0, 2, "a refused call reads no row");
    close(open);
    close(ptr::null_mut());
    // SAFETY: `book` is a live handle.
    assert_eq!(unsafe { xlsx_graft_book_close(book) }, XLSX_GRAFT_OK);
}
