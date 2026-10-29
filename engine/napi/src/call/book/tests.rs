//! Unit tests for the book: opening from a path and from bytes, its
//! sheets copied out, its diagnostics, and every call after its close.

use super::{Book, Sheet};
use crate::call::owned;
use crate::workbook::{chartsheet, dialogsheet, macrosheet, unrelated, worksheet, write, xlsx};
use xlsx_graft_capi::{
    XLSX_GRAFT_ERR_IO, XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_PANIC,
    XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE, XLSX_GRAFT_OK, XlsxGraftSheet,
};

/// `(name, kind, state)` of each sheet `book` lists.
fn sheets(book: &Book) -> Vec<(String, &'static str, &'static str)> {
    let sheets = book.sheets();
    assert_eq!(sheets.status, XLSX_GRAFT_OK);
    (sheets.sheets.into_iter())
        .map(|sheet| (sheet.name, sheet.kind, sheet.state))
        .collect()
}

#[test]
fn the_sheets_are_copied_out_with_their_kinds_and_states() {
    let (mut hidden, mut very) = (worksheet("Hidden", ""), chartsheet("Chart"));
    hidden.state = "hidden";
    very.state = "veryHidden";
    let bytes = xlsx(&[
        worksheet("Data", ""),
        hidden,
        very,
        dialogsheet("Dialog"),
        macrosheet("Macro"),
        unrelated("Lost"),
    ]);
    let book = Book::open_bytes(&bytes);
    assert_eq!(
        (book.status, book.message.as_deref()),
        (XLSX_GRAFT_OK, None)
    );
    let expected = [
        ("Data", "worksheet", "visible"),
        ("Hidden", "worksheet", "hidden"),
        ("Chart", "chartsheet", "veryHidden"),
        ("Dialog", "dialogsheet", "visible"),
        ("Macro", "macrosheet", "visible"),
        ("Lost", "unknown", "visible"),
    ];
    let expected = expected.map(|(name, kind, state)| (name.to_owned(), kind, state));
    assert_eq!(sheets(&book), expected);
}

#[test]
fn a_workbook_opens_from_its_path() {
    let (_dir, path) = write(&xlsx(&[worksheet("Data", "")]));
    let book = Book::open_path(path.to_str().unwrap());
    assert_eq!(book.status, XLSX_GRAFT_OK);
    assert_eq!(sheets(&book).len(), 1);
}

#[test]
fn a_file_that_will_not_open_is_the_host_failing_with_why() {
    let book = Book::open_path("/no/such/book.xlsx");
    assert_eq!(book.status, XLSX_GRAFT_ERR_IO);
    let message = book.message.as_deref().unwrap();
    assert!(message.starts_with("/no/such/book.xlsx: "), "{message}");
    assert_eq!(book.sheets().status, XLSX_GRAFT_ERR_NULL_ARG);
}

#[test]
fn bytes_that_hold_no_workbook_say_so_in_the_diagnostics() {
    let book = Book::open_bytes(b"");
    assert_eq!(book.status, XLSX_GRAFT_OK);
    let sheets = book.sheets();
    assert_eq!(
        (sheets.status, sheets.sheets),
        (XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE, vec![])
    );
    let diagnostics = book.diagnostics();
    assert_eq!(diagnostics.status, XLSX_GRAFT_OK);
    assert!(
        diagnostics
            .value
            .unwrap()
            .starts_with(r#"{"has_errors":true,"#)
    );
}

#[test]
fn every_call_after_the_close_is_refused_as_a_null_handles() {
    let mut book = Book::open_bytes(&xlsx(&[worksheet("Data", "")]));
    book.close();
    book.close();
    assert_eq!(book.sheets().status, XLSX_GRAFT_ERR_NULL_ARG);
    assert_eq!(book.diagnostics().status, XLSX_GRAFT_ERR_NULL_ARG);
    assert_eq!(book.rows(0).status, XLSX_GRAFT_ERR_NULL_ARG);
    assert_eq!(book.rows_named("Data").status, XLSX_GRAFT_ERR_NULL_ARG);
}

#[test]
fn a_handle_is_owned_only_on_ok() {
    let mut slot = 0_u8;
    let handle: *mut u8 = &raw mut slot;
    assert_eq!(owned(XLSX_GRAFT_OK, handle), handle);
    assert!(owned(XLSX_GRAFT_ERR_PANIC, handle).is_null());
}

#[test]
fn a_state_newer_than_the_addon_is_unknown() {
    let sheet = XlsxGraftSheet {
        name: b"New".as_ptr(),
        name_len: 3,
        kind: 99,
        state: 99,
    };
    // SAFETY: the name is a static string of `name_len` bytes.
    let sheet = unsafe { Sheet::of(&sheet) };
    assert_eq!((sheet.kind, sheet.state), ("unknown", "unknown"));
}
