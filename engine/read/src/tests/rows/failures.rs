//! What stops a sheet's reading besides where its rows and cells stand: a
//! value its type does not take, a shared string not there, a damaged or
//! refused entry, a book that cannot be opened, and the failures of the
//! host: the package's reader and the spool.

use super::super::rel;
use super::super::zip::Item;
use super::{
    Shown, book_of, briefs, on_sheet, open, open_with, read, row, rows, workbook, worksheet,
};
use crate::{Book, ReadError, ReadLimits, Row};
use std::cell::Cell;
use std::io::{self, Cursor};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit};
use xlsx_graft_package::{Limits, PackageError};
use xlsx_graft_spool::{MemorySpools, Spool, SpoolError, SpoolFactory};

/// The rows before the one diagnostic reading a sheet of `body` stops at,
/// and that diagnostic [`briefs`](super::briefs) it.
fn stops(body: &str) -> (Vec<Shown>, Vec<(String, String)>) {
    let (rows, seen) = rows(body);
    (rows, briefs(&seen))
}

#[test]
fn a_value_its_type_does_not_take_is_refused() {
    let invalid = || vec![on_sheet("schema:InvalidValue:v")];
    for cell in [
        "<c><v>one</v></c>",
        "<c><v></v></c>",
        "<c><v>inf</v></c>",
        r#"<c t="n"><v>NaN</v></c>"#,
        r#"<c t="d"><v> 2024-01-02 </v></c>"#,
        r#"<c t="b"><v>2</v></c>"#,
        r#"<c t="s"><v>-1</v></c>"#,
        r#"<c t="d"><v>2024-13-01</v></c>"#,
        r#"<c t="d"><v>noon</v></c>"#,
        r#"<c t="e"><v></v></c>"#,
    ] {
        let body = format!("<row><c><v>1</v></c></row><row>{cell}</row>");
        assert_eq!(stops(&body), (vec![row(1, &["n:1"])], invalid()), "{cell}");
    }
}

#[test]
fn a_style_or_type_its_type_does_not_take_is_refused_where_it_is_read() {
    assert_eq!(
        stops(r#"<row><c s="x"><v>1</v></c></row>"#),
        (vec![], vec![on_sheet("schema:InvalidValue:s")])
    );
    assert_eq!(
        stops(r#"<row><c t="x"><v>1</v></c></row>"#),
        (vec![], vec![on_sheet("schema:InvalidValue:t")])
    );
    assert_eq!(
        stops(r#"<row r="x"/>"#),
        (vec![], vec![on_sheet("schema:InvalidValue:r")])
    );
    // A string's `s` is not read.
    assert_eq!(
        stops(r#"<row><c s="x" t="str"><v>1</v></c></row>"#),
        (vec![row(1, &["s:1"])], vec![])
    );
}

#[test]
fn a_shared_string_not_there_is_missing() {
    let missing = Diagnostic::SharedStringMissing { index: 2 };
    let (rows, seen) = rows(r#"<row><c t="s"><v>2</v></c></row>"#);
    assert_eq!(
        (rows, seen),
        (vec![], vec![(missing.clone(), super::SHEET.to_owned())])
    );
    // A workbook without a table has none.
    let sheet = worksheet(r#"<row><c t="s"><v>2</v></c></row>"#);
    let (rows, seen) = read(workbook(&[("S", &sheet)], false, "", vec![]));
    assert_eq!(
        (rows, seen),
        (vec![], vec![(missing, super::SHEET.to_owned())])
    );
}

#[test]
fn a_shared_string_of_a_table_not_known_stops_without_a_diagnostic_of_its_own() {
    let sheet = worksheet(r#"<row><c><v>1</v></c></row><row><c t="s"><v>0</v></c></row>"#);
    let related = rel("rId9", super::super::SHARED_STRINGS, "sharedStrings.xml");
    let table = Item::stored("xl/sharedStrings.xml", b"<sst/>");
    let (rows, seen) = read(workbook(&[("S", &sheet)], false, &related, vec![table]));
    assert_eq!(rows, vec![row(1, &["n:1"])]);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].1, "/xl/sharedStrings.xml");
}

/// A book of one sheet of `body`, its stored bytes changed by `damage`
/// after the CRC-32 was taken.
fn damaged(body: &str, damage: impl FnOnce(&mut Vec<u8>)) -> Vec<Item> {
    let mut items = book_of(&worksheet(body));
    let at = items.len() - 2;
    items[at] = items[at].clone().damaged(damage);
    items
}

/// Replaces the first `from` in `data` with `to`, of the same length.
fn replace(data: &mut [u8], from: &[u8], to: &[u8]) {
    let at = data.windows(from.len()).position(|w| w == from).unwrap();
    data[at..at + to.len()].copy_from_slice(to);
}

#[test]
fn a_damaged_entry_reports_its_crc_alone_in_place_of_a_problem_found() {
    let body = r#"<row r="1"><c><v>1</v></c></row><row r="2"><c><v>2</v></c></row>"#;
    let crc = vec![(
        "zip_crc_mismatch".to_owned(),
        "xl/worksheets/sheet1.xml".to_owned(),
    )];
    // A row out of order, and XML that is malformed.
    for (from, to) in [(&b"r=\"2\""[..], &b"r=\"1\""[..]), (b">2</v>", b">2</x>")] {
        let (rows, seen) = read(damaged(body, |data| replace(data, from, to)));
        assert_eq!((rows, briefs(&seen)), (vec![row(1, &["n:1"])], crc.clone()));
    }
    // And with no problem found, after every row.
    let (rows, seen) = read(damaged(body, |data| replace(data, b">2<", b">3<")));
    let read = vec![row(1, &["n:1"]), row(2, &["n:3"])];
    assert_eq!((rows, briefs(&seen)), (read, crc));
}

#[test]
fn a_sheet_the_zip_refused_has_no_rows() {
    let mut items = book_of(&worksheet(""));
    let at = items.len() - 2;
    items[at] = items[at].clone().with_method(99);
    let mut diagnostics = Diagnostics::new();
    let book = open(items, &mut diagnostics);
    assert!(book.rows(0).is_none());
    let seen = super::super::seen(diagnostics);
    assert!(
        matches!(
            seen[..],
            [(Diagnostic::ZipUnsupportedCompression { method: 99 }, _)]
        ),
        "{seen:?}"
    );
}

/// The rows of a sheet whose `sheetData` holds `body`, a row's text held
/// to `max` bytes, and what was found.
fn rows_within(body: &str, max: u64) -> (Vec<Shown>, super::Seen) {
    let limits = ReadLimits {
        max_row_bytes: Limit::default_of(max),
        ..ReadLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let book = open_with(
        book_of(&worksheet(body)),
        &limits,
        Rc::default(),
        &mut diagnostics,
    );
    super::read_book(&book.unwrap(), diagnostics)
}

#[test]
fn a_row_of_more_text_than_max_row_bytes_is_refused() {
    // Strings and error values count, and a value's text while it is read:
    // a number's, and a shared string's index.
    let body = concat!(
        r#"<row r="4"><c t="inlineStr"><is><t>abc</t></is></c><c t="s"><v>0</v></c>"#,
        r#"<c t="e"><v>#N/A</v></c><c><v>123456</v></c></row>"#,
    );
    let read = vec![row(4, &["s:abc", "s:a", "e:#N/A", "n:123456"])];
    assert_eq!(rows_within(body, 14), (read, vec![]));
    let too_large = |size| {
        let diagnostic = Diagnostic::RowTooLarge {
            row: 4,
            size,
            limit: Limit::default_of(size - 1),
        };
        (vec![], vec![(diagnostic, super::SHEET.to_owned())])
    };
    // At the number, the error value, the index and the inline string.
    assert_eq!(rows_within(body, 13), too_large(14));
    assert_eq!(rows_within(body, 7), too_large(8));
    assert_eq!(rows_within(body, 3), too_large(4));
    assert_eq!(rows_within(body, 2), too_large(3));
    // Each row starts again from none.
    let rows = format!("{body}{}", body.replace("r=\"4\"", "r=\"5\""));
    assert_eq!(rows_within(&rows, 14).0.len(), 2);
}

#[test]
fn a_book_without_a_workbook_to_read_does_not_open() {
    let open_none = |items: Vec<Item>| {
        let mut diagnostics = Diagnostics::new();
        let book = open_with(
            items,
            &ReadLimits::default(),
            Rc::default(),
            &mut diagnostics,
        );
        (book.is_none(), diagnostics.has_errors())
    };
    // No workbook part, a workbook part refused, and no package.
    let mut items = book_of(&worksheet(""));
    items.remove(1);
    assert_eq!(open_none(items), (true, true));
    let mut items = book_of(&worksheet(""));
    items[2] = Item::stored("xl/workbook.xml", b"<workbook/>");
    assert_eq!(open_none(items), (true, true));
    let mut diagnostics = Diagnostics::new();
    let reader = super::super::Reader {
        inner: Cursor::new(b"not a zip".to_vec()),
        poisoned: Rc::default(),
    };
    let (package, limits) = (Limits::default(), ReadLimits::default());
    let book = Book::open(
        reader,
        &package,
        &limits,
        &mut MemorySpools,
        &mut diagnostics,
    );
    assert!(book.unwrap().is_none());
}

/// Reads the first sheet of the book of `items` with the reader failing
/// from `from` bytes into the sheet entry's data to the end of the file,
/// `from` 0 failing the seek to it too.
fn read_failing(items: Vec<Item>, from: u64) -> Result<Vec<Shown>, ReadError> {
    // The same zip, opened as a package, says where the sheet's data is.
    let package = super::super::open(items.clone(), &mut Diagnostics::new());
    let part = package.part(super::SHEET).unwrap().entry();
    let range = package.archive().entries()[part].data_range();
    let mut diagnostics = Diagnostics::new();
    let poisoned = Rc::new(Cell::new((0, 0)));
    let limits = ReadLimits::default();
    let book = open_with(items, &limits, Rc::clone(&poisoned), &mut diagnostics).unwrap();
    let mut rows = book.rows(0).unwrap();
    poisoned.set((range.start + from, u64::MAX));
    let mut row = Row::new();
    let mut read = Vec::new();
    while rows.next_row(&mut row, &mut diagnostics)? {
        read.push(super::shown(&row));
    }
    Ok(read)
}

/// Whether `result` is a failure of the package's reader.
fn is_host_failure(result: &Result<Vec<Shown>, ReadError>) -> bool {
    matches!(result, Err(ReadError::Package(PackageError::Io(_))))
}

/// A sheet of about 300 KB whose first row is `first`.
fn large(first: &str) -> Vec<Item> {
    let rest = "<row><c><v>0123456789</v></c></row>".repeat(8_000);
    book_of(&worksheet(&format!("{first}{rest}")))
}

#[test]
fn a_host_failure_reading_the_sheet_is_an_error() {
    assert!(is_host_failure(&read_failing(large(""), 0)));
    assert!(is_host_failure(&read_failing(large(""), 100_000)));
}

#[test]
fn a_host_failure_reading_past_a_problem_is_an_error() {
    assert!(is_host_failure(&read_failing(large("<x/>"), 200_000)));
}

/// Spools that take what is written and fail every read.
struct Unreadable;

/// A spool that fails every read.
struct Unread(Vec<u8>);

impl Spool for Unread {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0.extend(bytes);
        Ok(())
    }

    fn read_exact_at(&mut self, _: u64, _: &mut [u8]) -> io::Result<()> {
        Err(io::Error::other("the spool went away"))
    }
}

impl SpoolFactory for Unreadable {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Ok(Box::new(Unread(Vec::new())))
    }
}

#[test]
fn a_spool_that_fails_a_lookup_is_an_error() {
    let limits = ReadLimits {
        shared_strings_spill_bytes: 0,
        ..ReadLimits::default()
    };
    let items = book_of(&worksheet(r#"<row><c t="s"><v>0</v></c></row>"#));
    let reader = super::super::Reader {
        inner: Cursor::new(super::super::zip::build(items)),
        poisoned: Rc::default(),
    };
    let mut diagnostics = Diagnostics::new();
    let package = Limits::default();
    let book = Book::open(reader, &package, &limits, &mut Unreadable, &mut diagnostics);
    let book = book.unwrap().unwrap();
    let mut rows = book.rows(0).unwrap();
    let read = rows.next_row(&mut Row::new(), &mut diagnostics);
    assert!(matches!(read, Err(ReadError::Spool(SpoolError::Io(_)))));
    // Nothing more comes after.
    assert!(!rows.next_row(&mut Row::new(), &mut diagnostics).unwrap());
}
