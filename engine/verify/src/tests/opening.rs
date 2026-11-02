//! What opening and reading the package refuses, reached through `verify`:
//! the zip bombs and a DOCTYPE, and what each XML part's reading leaves.

use super::zip::{self, Item};
use super::{Book, WORKSHEET, check, denied, run};
use crate::{Limits, Rules, VerifyError};
use xlsx_graft_diagnostics::{Diagnostic, Limit, PartKind};

/// The worksheet each case puts its problem in.
const SHEET: &str = "/xl/worksheets/sheet1.xml";

/// A workbook whose worksheet holds `xml`.
fn with_sheet(xml: &str) -> Book {
    Book::new()
        .part(SHEET, WORKSHEET, xml)
        .rel("/xl/workbook.xml", "worksheet", SHEET)
}

/// The limits, with the zip's set by `set`.
fn limits(set: impl FnOnce(&mut xlsx_graft_package::ZipLimits)) -> Limits {
    let mut limits = Limits::default();
    set(&mut limits.package.zip);
    limits
}

/// Verifies `book` under `limits`.
fn check_under(book: Book, limits: &Limits) -> super::Seen {
    run(book.zip(), limits, &Rules::default(), (0, 0)).unwrap()
}

/// `(diagnostic, place)`.
fn at(diagnostic: Diagnostic, place: &str) -> (Diagnostic, String) {
    (diagnostic, place.to_owned())
}

#[test]
fn an_entry_that_inflates_past_the_ratio_limit_is_refused() {
    let book = with_sheet("").replace(Item::deflated("xl/worksheets/sheet1.xml", b"x", 100));
    let limits = limits(|zip| {
        zip.max_compression_ratio = Limit::default_of(10);
        zip.min_ratio_checked_bytes = Limit::default_of(1);
    });
    assert_eq!(
        check_under(book, &limits),
        vec![at(
            Diagnostic::ZipRatioExceeded {
                size: 100,
                compressed_size: 1,
                limit: Limit::default_of(10),
            },
            "xl/worksheets/sheet1.xml"
        )]
    );
}

#[test]
fn an_entry_larger_than_the_entry_limit_is_refused() {
    let book = with_sheet("").replace(Item::deflated("xl/worksheets/sheet1.xml", b"x", 10_000));
    let limits = limits(|zip| zip.max_entry_bytes = Limit::default_of(9_999));
    assert_eq!(
        check_under(book, &limits),
        vec![at(
            Diagnostic::ZipEntryTooLarge {
                size: 10_000,
                limit: Limit::default_of(9_999),
            },
            "xl/worksheets/sheet1.xml"
        )]
    );
}

#[test]
fn entries_larger_than_the_total_limit_are_refused() {
    let book = with_sheet("<worksheet/>");
    let total = (book.items().iter()).map(Item::size).sum::<u64>();
    let book = with_sheet("<worksheet/>");
    let limits = limits(|zip| zip.max_total_bytes = Limit::default_of(total - 1));
    assert_eq!(
        check_under(book, &limits),
        vec![at(
            Diagnostic::ZipTotalTooLarge {
                size: total,
                limit: Limit::default_of(total - 1),
            },
            "xl/worksheets/sheet1.xml"
        )]
    );
}

#[test]
fn a_doctype_in_a_part_read_is_refused_and_what_came_before_it_counts() {
    let xml = r#"<worksheet><webPublishItems><webPublishItem/></webPublishItems><!DOCTYPE x [<!ENTITY e SYSTEM "file:///etc/passwd">]></worksheet>"#;
    let seen = check(with_sheet(xml));
    assert_eq!(seen.len(), 2);
    assert!(matches!(seen[0].0, Diagnostic::XmlDoctype { .. }));
    assert_eq!(seen[0].1, SHEET);
    assert_eq!(seen[1], denied(PartKind::WebPublish, SHEET));
}

#[test]
fn a_comment_too_large_before_the_first_element_is_refused_in_any_part() {
    let xml = format!(
        "<!--{}--><worksheet><webPublishItem/></worksheet>",
        "x".repeat(300)
    );
    let book = Book::new()
        .part(
            "/xl/worksheets/sheet1.bin",
            "application/octet-stream",
            &xml,
        )
        .rel("/xl/workbook.xml", "worksheet", "/xl/worksheets/sheet1.bin");
    let mut limits = Limits::default();
    limits.package.xml.max_event_bytes = Limit::default_of(256);
    let seen = check_under(book, &limits);
    assert_eq!(seen.len(), 1);
    assert!(matches!(
        seen[0].0,
        Diagnostic::XmlEventTooLarge { offset: 0, .. }
    ));
    assert_eq!(seen[0].1, "/xl/worksheets/sheet1.bin");
}

#[test]
fn a_doctype_in_the_content_types_stream_is_refused() {
    let mut items = Book::new().items();
    let types = concat!(
        r#"<?xml version="1.0"?><!DOCTYPE Types [<!ENTITY e SYSTEM "file:///etc/passwd">]>"#,
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#,
    );
    items[0] = Item::stored("[Content_Types].xml", types.as_bytes());
    let seen = run(
        zip::build(items),
        &Limits::default(),
        &Rules::default(),
        (0, 0),
    )
    .unwrap();
    assert_eq!(seen.len(), 1);
    assert!(matches!(seen[0].0, Diagnostic::XmlDoctype { offset: 21 }));
    assert_eq!(seen[0].1, "[Content_Types].xml");
}

#[test]
fn a_part_whose_data_is_damaged_reports_the_damage_alone() {
    let item =
        Item::stored("xl/worksheets/sheet1.xml", b"<worksheet/>").damaged(|data| data[1] = b'W');
    let book = with_sheet("<worksheet/>").replace(item);
    assert_eq!(
        check(book),
        vec![at(
            Diagnostic::ZipCrcMismatch {
                expected: crc32("<worksheet/>"),
                actual: crc32("<Worksheet/>"),
            },
            "xl/worksheets/sheet1.xml"
        )]
    );
}

#[test]
fn a_part_whose_entry_is_refused_is_not_read() {
    let item = Item::stored("xl/worksheets/sheet1.xml", b"<webPublishItem/>").with_method(99);
    let seen = check(with_sheet("").replace(item));
    assert_eq!(seen.len(), 1);
    assert!(matches!(
        seen[0].0,
        Diagnostic::ZipUnsupportedCompression { method: 99 }
    ));
}

#[test]
fn a_file_that_is_not_a_zip_is_refused_alone() {
    let seen = run(
        b"not a zip".to_vec(),
        &Limits::default(),
        &Rules::default(),
        (0, 0),
    )
    .unwrap();
    assert_eq!(seen.len(), 1);
    assert!(matches!(seen[0].0, Diagnostic::NotAZip {}));
}

#[test]
fn a_failure_of_the_host_while_reading_a_part_is_an_error() {
    let zip = with_sheet("<worksheet>MARK</worksheet>").zip();
    let mark = zip.windows(4).position(|w| w == b"MARK").unwrap() as u64;
    let start = mark - "<worksheet>".len() as u64;
    let error = run(
        zip,
        &Limits::default(),
        &Rules::default(),
        (start, start + 1),
    )
    .unwrap_err();
    assert!(matches!(error, VerifyError::Package(_)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "reading the package failed: the disk went away"
    );
}

/// The CRC-32 of `text`.
fn crc32(text: &str) -> u64 {
    u64::from(crc32fast::hash(text.as_bytes()))
}
