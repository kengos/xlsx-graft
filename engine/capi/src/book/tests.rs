//! Unit tests for the book: what opening finds, which sheet a choice
//! opens or why none, and the diagnostics' JSON.

use super::{Choice, Reader, XlsxGraftBook};
use crate::message::lent;
use crate::sheet::*;
use crate::status::*;
use crate::tests::workbook::{
    Sheet, chartsheet, dialogsheet, macrosheet, unrelated, worksheet, xlsx,
};
use serde_json::Value;
use std::io::{self, Cursor, Read, Seek, SeekFrom};

/// The book of the workbook of `sheets`.
fn open(sheets: &[Sheet<'_>]) -> XlsxGraftBook {
    XlsxGraftBook::open(Box::new(Cursor::new(xlsx(sheets)))).unwrap()
}

/// The diagnostics of `book`, parsed.
fn diagnostics(book: &mut XlsxGraftBook) -> Value {
    serde_json::from_str(book.diagnostics().unwrap()).unwrap()
}

/// The status `choice` opens the rows of `book` with: `OK` when it opens
/// them.
fn status(book: &XlsxGraftBook, choice: Choice<'_>) -> i32 {
    book.rows(choice)
        .map_or_else(|status| status, |_| XLSX_GRAFT_OK)
}

#[test]
fn the_sheets_are_listed_with_their_names_kinds_and_states() {
    let mut hidden = worksheet("Hidden", "");
    hidden.state = "hidden";
    let mut very = chartsheet("Chart");
    very.state = "veryHidden";
    let book = open(&[
        worksheet("Data", ""),
        hidden,
        very,
        dialogsheet("Dialog"),
        macrosheet("Macro"),
        unrelated("Lost"),
    ]);
    let sheets: Vec<(String, u32, u32)> = (book.sheets().unwrap().iter())
        .map(|sheet| {
            // SAFETY: a sheet's name points at `name_len` bytes of the book,
            // which is alive for the call.
            let name = unsafe { std::slice::from_raw_parts(sheet.name, sheet.name_len) };
            let name = String::from_utf8(name.to_vec()).unwrap();
            (name, sheet.kind, sheet.state)
        })
        .collect();
    let expected = [
        ("Data", XLSX_GRAFT_SHEET_WORKSHEET, XLSX_GRAFT_STATE_VISIBLE),
        (
            "Hidden",
            XLSX_GRAFT_SHEET_WORKSHEET,
            XLSX_GRAFT_STATE_HIDDEN,
        ),
        (
            "Chart",
            XLSX_GRAFT_SHEET_CHARTSHEET,
            XLSX_GRAFT_STATE_VERY_HIDDEN,
        ),
        (
            "Dialog",
            XLSX_GRAFT_SHEET_DIALOGSHEET,
            XLSX_GRAFT_STATE_VISIBLE,
        ),
        (
            "Macro",
            XLSX_GRAFT_SHEET_MACROSHEET,
            XLSX_GRAFT_STATE_VISIBLE,
        ),
        ("Lost", XLSX_GRAFT_SHEET_UNKNOWN, XLSX_GRAFT_STATE_VISIBLE),
    ];
    let expected = expected.map(|(name, kind, state)| (name.to_owned(), kind, state));
    assert_eq!(sheets, expected);
}

#[test]
fn a_sheet_is_opened_by_index_or_by_name_letter_case_aside() {
    let book = open(&[worksheet("Data", ""), worksheet("Más", "")]);
    for choice in [Choice::Index(1), Choice::Name("MÁS"), Choice::Name("Data")] {
        assert_eq!(status(&book, choice), XLSX_GRAFT_OK, "{choice:?}");
    }
}

#[test]
fn each_sheet_that_cannot_be_opened_says_why() {
    let book = open(&[
        worksheet("Data", ""),
        worksheet("data", ""),
        chartsheet("Chart"),
        unrelated("Lost"),
        macrosheet("Macro"),
    ]);
    let cases = [
        (Choice::Index(5), XLSX_GRAFT_ERR_SHEET_NOT_FOUND),
        (Choice::Index(4), XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET),
        (Choice::Name("Nope"), XLSX_GRAFT_ERR_SHEET_NOT_FOUND),
        (Choice::Name("DATA"), XLSX_GRAFT_ERR_SHEET_AMBIGUOUS),
        (Choice::Name("chart"), XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET),
        (Choice::Index(3), XLSX_GRAFT_ERR_SHEET_UNREADABLE),
        (Choice::Index(0), XLSX_GRAFT_OK),
    ];
    for (choice, expected) in cases {
        assert_eq!(status(&book, choice), expected, "{choice:?}");
    }
}

#[test]
fn a_workbook_that_cannot_be_read_still_opens_and_says_why() {
    let mut book = XlsxGraftBook::open(Box::new(Cursor::new(b"not a zip".to_vec()))).unwrap();
    assert_eq!(book.sheets(), Err(XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE));
    let status = status(&book, Choice::Index(0));
    assert_eq!(status, XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE);
    let report = diagnostics(&mut book);
    assert_eq!(report["has_errors"], true);
    assert!(!report["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn a_clean_workbook_has_no_diagnostics() {
    let mut book = open(&[worksheet("Data", "")]);
    let report = diagnostics(&mut book);
    assert_eq!(
        report,
        serde_json::json!({"has_errors": false, "diagnostics": []})
    );
}

#[test]
fn the_diagnostics_are_the_ones_every_host_prints() {
    let mut book = open(&[worksheet("Data", ""), worksheet("DATA", "")]);
    let report = diagnostics(&mut book);
    assert_eq!(report["diagnostics"][0]["code"], "sheet_name_duplicate");
    assert_eq!(report["diagnostics"].as_array().unwrap().len(), 1);
}

/// A reader whose every read and seek fails.
struct Failing;

impl Read for Failing {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("the disk is gone"))
    }
}

impl Seek for Failing {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::other("the disk is gone"))
    }
}

#[test]
fn the_host_failing_makes_no_book_and_keeps_why() {
    let reader: Reader = Box::new(Failing);
    let status = XlsxGraftBook::open(reader).map(|_| ()).unwrap_err();
    assert_eq!(status, XLSX_GRAFT_ERR_IO);
    let (ptr, len) = lent();
    // SAFETY: the message is alive until the next failure on this thread.
    let message = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(String::from_utf8_lossy(message).contains("the disk is gone"));
}
