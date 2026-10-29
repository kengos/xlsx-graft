//! Unit tests for reading rows: each row's number and values, the end,
//! a problem that stops the reading, the host failing, and that the book
//! may go first.

use crate::book::{Choice, XlsxGraftBook};
use crate::status::XLSX_GRAFT_ERR_IO;
use crate::tests::workbook::{worksheet, xlsx};
use crate::value::{
    XLSX_GRAFT_VALUE_DATE, XLSX_GRAFT_VALUE_EMPTY, XLSX_GRAFT_VALUE_NUMBER,
    XLSX_GRAFT_VALUE_STRING, XlsxGraftValue,
};
use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

/// The book of one worksheet, `Data`, whose `sheetData` holds `rows`.
fn book(rows: &str) -> XlsxGraftBook {
    let bytes = xlsx(&[worksheet("Data", rows)]);
    XlsxGraftBook::open(Box::new(Cursor::new(bytes))).unwrap()
}

/// What a test reads of a value: its kind, its number or year, and its
/// text.
fn read(value: &XlsxGraftValue) -> (u32, f64, Option<String>) {
    let text = (!value.text.is_null()).then(|| {
        // SAFETY: a value's text is alive until the next row is read.
        let bytes = unsafe { std::slice::from_raw_parts(value.text, value.text_len) };
        String::from_utf8(bytes.to_vec()).unwrap()
    });
    let number = if value.kind == XLSX_GRAFT_VALUE_DATE {
        f64::from(value.year)
    } else {
        value.number
    };
    (value.kind, number, text)
}

#[test]
fn each_row_comes_with_its_number_and_its_values_from_column_a() {
    let book = book(concat!(
        r#"<row r="1"><c r="A1" t="s"><v>1</v></c><c r="C1"><v>3</v></c></row>"#,
        r#"<row r="3"><c r="B3" s="1"><v>45322</v></c></row>"#,
    ));
    let mut rows = book.rows(Choice::Index(0)).unwrap();
    let (number, values) = rows.next().unwrap().unwrap();
    let values: Vec<_> = values.iter().map(read).collect();
    let string = (XLSX_GRAFT_VALUE_STRING, 0.0, Some("b".to_owned()));
    let empty = (XLSX_GRAFT_VALUE_EMPTY, 0.0, None);
    let three = (XLSX_GRAFT_VALUE_NUMBER, 3.0, None);
    assert_eq!((number, values), (1, vec![string, empty.clone(), three]));
    let (number, values) = rows.next().unwrap().unwrap();
    let values: Vec<_> = values.iter().map(read).collect();
    let date = (XLSX_GRAFT_VALUE_DATE, 2024.0, None);
    assert_eq!((number, values), (3, vec![empty, date]));
    assert_eq!(rows.next(), Ok(None));
    assert_eq!(rows.next(), Ok(None));
}

#[test]
fn a_problem_ends_the_rows_and_lands_in_the_books_diagnostics() {
    let mut book = book(r#"<row r="1"><c r="A1"><v>1</v></c></row><row r="1"/>"#);
    let mut rows = book.rows(Choice::Index(0)).unwrap();
    assert_eq!(rows.next().unwrap().map(|(number, _)| number), Some(1));
    assert_eq!(rows.next(), Ok(None));
    let report: serde_json::Value = serde_json::from_str(book.diagnostics().unwrap()).unwrap();
    assert_eq!(report["has_errors"], true);
    assert_eq!(report["diagnostics"][0]["code"], "cell_order_invalid");
}

#[test]
fn the_rows_read_on_after_the_book_is_dropped() {
    let book = book(r#"<row r="2"><c r="A2" t="s"><v>0</v></c></row>"#);
    let mut rows = book.rows(Choice::Index(0)).unwrap();
    drop(book);
    let (number, values) = rows.next().unwrap().unwrap();
    let values: Vec<_> = values.iter().map(read).collect();
    let string = (XLSX_GRAFT_VALUE_STRING, 0.0, Some("a".to_owned()));
    assert_eq!((number, values), (2, vec![string]));
}

#[test]
fn two_sheets_of_one_book_read_in_turns() {
    let bytes = xlsx(&[
        worksheet("A", r#"<row r="1"/><row r="2"/>"#),
        worksheet("B", r#"<row r="5"/><row r="6"/>"#),
    ]);
    let book = XlsxGraftBook::open(Box::new(Cursor::new(bytes))).unwrap();
    let mut first = book.rows(Choice::Index(0)).unwrap();
    let mut second = book.rows(Choice::Index(1)).unwrap();
    let mut numbers = Vec::new();
    for _ in 0..2 {
        numbers.push(first.next().unwrap().map(|(number, _)| number));
        numbers.push(second.next().unwrap().map(|(number, _)| number));
    }
    assert_eq!(numbers, [Some(1), Some(5), Some(2), Some(6)]);
}

/// A workbook's bytes, read until the test pulls the plug.
struct Unplugged {
    bytes: Cursor<Vec<u8>>,
    unplugged: Rc<Cell<bool>>,
}

impl Read for Unplugged {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.unplugged.get() {
            return Err(io::Error::other("unplugged"));
        }
        self.bytes.read(out)
    }
}

impl Seek for Unplugged {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.bytes.seek(to)
    }
}

#[test]
fn the_host_failing_mid_sheet_is_io_and_no_row_comes_after() {
    let unplugged = Rc::new(Cell::new(false));
    let bytes = xlsx(&[worksheet("Data", r#"<row r="1"/>"#)]);
    let reader = Unplugged {
        bytes: Cursor::new(bytes),
        unplugged: Rc::clone(&unplugged),
    };
    let book = XlsxGraftBook::open(Box::new(reader)).unwrap();
    let mut rows = book.rows(Choice::Index(0)).unwrap();
    unplugged.set(true);
    assert_eq!(rows.next(), Err(XLSX_GRAFT_ERR_IO));
    unplugged.set(false);
    assert_eq!(rows.next(), Ok(None));
}
