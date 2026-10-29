//! Unit tests for the rows: each row copied out, the end, every kind of
//! cell, and every call after the close.

use super::Cell;
use crate::call::Book;
use crate::workbook::{chartsheet, worksheet, xlsx};
use xlsx_graft_capi::{
    XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_SHEET_AMBIGUOUS, XLSX_GRAFT_ERR_SHEET_NOT_FOUND,
    XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET, XLSX_GRAFT_OK, XlsxGraftValue,
};

/// A value of `kind` and nothing else.
fn value(kind: u32) -> XlsxGraftValue {
    XlsxGraftValue {
        kind,
        year: 2024,
        month: 1,
        day: 31,
        hour: 8,
        minute: 30,
        second: 59,
        boolean: 1,
        millisecond: 500,
        reserved: 0,
        number: -0.5,
        elapsed: 93_780_000,
        text: c"#N/A".as_ptr().cast(),
        text_len: 4,
    }
}

#[test]
fn each_kind_of_cell_keeps_only_its_own_fields() {
    let (date, time) = (Some([2024, 1, 31]), Some([8, 30, 59, 500]));
    let text = Some("#N/A".to_owned());
    let expected = [
        Cell {
            kind: "empty",
            ..Cell::default()
        },
        Cell {
            kind: "string",
            text: text.clone(),
            ..Cell::default()
        },
        Cell {
            kind: "number",
            number: Some(-0.5),
            ..Cell::default()
        },
        Cell {
            kind: "boolean",
            boolean: Some(true),
            ..Cell::default()
        },
        Cell {
            kind: "date",
            date,
            ..Cell::default()
        },
        Cell {
            kind: "time",
            time,
            ..Cell::default()
        },
        Cell {
            kind: "datetime",
            date,
            time,
            ..Cell::default()
        },
        Cell {
            kind: "elapsed",
            milliseconds: Some(93_780_000.0),
            ..Cell::default()
        },
        Cell {
            kind: "error",
            text,
            ..Cell::default()
        },
        Cell {
            kind: "unknown",
            ..Cell::default()
        },
    ];
    for (kind, expected) in (0..).zip(expected) {
        // SAFETY: the text is a static string of `text_len` bytes.
        assert_eq!(unsafe { Cell::of(&value(kind)) }, expected);
    }
    let mut no_text = value(1);
    no_text.text = std::ptr::null();
    // SAFETY: a null text is not read.
    assert_eq!(unsafe { Cell::of(&no_text) }.text, None);
}

#[test]
fn the_rows_are_copied_out_to_the_end() {
    let rows = r#"<row r="2"><c r="B2" t="s"><v>1</v></c></row><row r="4"/>"#;
    let book = Book::open_bytes(&xlsx(&[worksheet("Data", rows)]));
    let mut rows = book.rows_named("data");
    assert_eq!(rows.status, XLSX_GRAFT_OK);
    drop(book);
    let first = rows.next_row().row.unwrap();
    let kinds: Vec<_> = first.cells.iter().map(|cell| cell.kind).collect();
    assert_eq!((first.number, kinds), (2, vec!["empty", "string"]));
    assert_eq!(first.cells[1].text.as_deref(), Some("b"));
    let empty = rows.next_row().row.unwrap();
    assert_eq!((empty.number, empty.cells), (4, vec![]));
    let end = rows.next_row();
    assert_eq!(
        (end.status, end.message, end.row),
        (XLSX_GRAFT_OK, None, None)
    );
}

#[test]
fn a_refused_sheet_and_a_closed_one_read_no_row() {
    let book = Book::open_bytes(&xlsx(&[worksheet("A", ""), worksheet("a", "")]));
    let mut ambiguous = book.rows_named("A");
    assert_eq!(ambiguous.status, XLSX_GRAFT_ERR_SHEET_AMBIGUOUS);
    assert_eq!(ambiguous.next_row().status, XLSX_GRAFT_ERR_NULL_ARG);
    let mut rows = book.rows(0);
    rows.close();
    rows.close();
    assert_eq!(
        (rows.next_row().status, rows.next_row().row),
        (XLSX_GRAFT_ERR_NULL_ARG, None)
    );
}

#[test]
fn a_sheet_is_opened_by_index_or_refused_with_its_status() {
    let rows = r#"<row r="3"/>"#;
    let book = Book::open_bytes(&xlsx(&[chartsheet("Chart"), worksheet("Data", rows)]));
    let mut data = book.rows(1);
    assert_eq!(data.status, XLSX_GRAFT_OK);
    assert_eq!(data.next_row().row.map(|row| row.number), Some(3));
    let statuses = [book.rows(0).status, book.rows(2).status];
    assert_eq!(
        statuses,
        [
            XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET,
            XLSX_GRAFT_ERR_SHEET_NOT_FOUND
        ]
    );
}
