//! Tests for `strings.rs`: the shared strings table, read before the
//! sheets, through `verify`: the length of each string, and how many there
//! are, which a cell's index is checked against. What Excel does with each
//! is Excel for Mac 16.114.3's, opening the workbook.

use super::zip::Item;
use super::{Book, WORKSHEET, check, run};
use crate::{Limits, Rules, VerifyError};
use xlsx_graft_diagnostics::Diagnostic;

/// The part the tables are in.
const PART: &str = "/xl/sharedStrings.xml";
/// The Transitional SpreadsheetML namespace.
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// A workbook whose shared strings table is `xml`.
fn with_table(xml: &str) -> Book {
    let media = "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";
    (Book::new().part(PART, media, xml)).rel("/xl/workbook.xml", "sharedStrings", PART)
}

/// A workbook whose shared strings are `items`, each an `si`'s content.
fn with_items(items: &[String]) -> Book {
    let body: String = items
        .iter()
        .map(|item| format!("<si>{item}</si>"))
        .collect();
    with_table(&format!(r#"<sst xmlns="{MAIN}">{body}</sst>"#))
}

/// This workbook, with a sheet whose `A1` is the shared string `index`.
fn with_cell(book: Book, index: u64) -> Book {
    let sheet = "/xl/worksheets/sheet1.xml";
    let xml = format!(
        r#"<worksheet xmlns="{MAIN}"><sheetData><row r="1"><c r="A1" t="s"><v>{index}</v></c></row></sheetData></worksheet>"#
    );
    let from_workbook =
        (book.relationships.iter()).filter(|(source, _)| source == "/xl/workbook.xml");
    let id = 1 + from_workbook.count();
    let workbook = format!(
        r#"<workbook xmlns="{MAIN}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="S" sheetId="1" r:id="rId{id}"/></sheets></workbook>"#
    );
    (book.main("application/xml", &workbook))
        .part(sheet, WORKSHEET, &xml)
        .rel("/xl/workbook.xml", "worksheet", sheet)
}

/// `shared_string_too_long` of the string `index`, `length` long.
fn too_long(index: u64, length: u64) -> (Diagnostic, String) {
    (
        Diagnostic::SharedStringTooLong { index, length },
        PART.to_owned(),
    )
}

/// `shared_string_missing` of `index`.
fn missing(index: u64) -> (Diagnostic, String) {
    (
        Diagnostic::SharedStringMissing { index },
        "/xl/worksheets/sheet1.xml".to_owned(),
    )
}

/// A `t` holding `text`.
fn t(text: &str) -> String {
    format!("<t>{text}</t>")
}

/// A run whose `t` holds `text`.
fn rich(text: &str) -> String {
    format!("<r><rPr><b/></rPr>{}</r>", t(text))
}

/// A phonetic run over the first character, whose `t` holds `text`.
fn phonetic(text: &str) -> String {
    format!(r#"<rPh sb="0" eb="1">{}</rPh>"#, t(text))
}

/// The strings at and past the bound, each as Excel for Mac opens it, and
/// the diagnostics each past it raises, in the table's order.
fn cases() -> (Vec<String>, Vec<(Diagnostic, String)>) {
    let a = |n| "a".repeat(n);
    let items = vec![
        t("short"),
        // At the bound: Excel opens it as it is.
        t(&a(32_767)),
        // Past it: Excel for Mac asks to repair, and cuts it to 32,767.
        t(&a(32_768)),
        // 16,384 characters past the Basic Multilingual Plane are 32,768
        // code units: Excel for Mac asks to repair, and cuts the last pair
        // in two.
        t(&"\u{1F600}".repeat(16_384)),
        // 16,383 of them, 32,766 code units: Excel opens it as it is.
        t(&"\u{1F600}".repeat(16_383)),
        // One more code unit makes 32,767.
        t(&("\u{1F600}".repeat(16_383) + "a")),
        // 32,767 code units in 98,301 bytes of UTF-8.
        t(&"\u{30A2}".repeat(32_767)),
        // Runs together past the bound: Excel for Mac asks to repair.
        rich(&a(16_384)) + &rich(&a(16_384)),
        // Runs together at it: Excel opens it as it is.
        rich(&a(16_384)) + &rich(&a(16_383)),
        // A phonetic run is not counted with the text: Excel opens it as it
        // is.
        t(&a(32_767)) + &phonetic(&"\u{30A2}".repeat(10)),
        // An escape is one character, `_x005F_` one `_`.
        t(&"_x005F_".repeat(32_768)),
        // An unpaired surrogate's escape is one code unit, as Excel counts
        // it: Excel for Mac opens 4,682 of them, 32,774 characters as
        // written, as it is, and asks to repair 32,768 of them.
        t(&"_xD83D_".repeat(4_682)),
        t(&"_xD83D_".repeat(32_768)),
        // An escape held back at the end of a `t` counts as written.
        rich(&a(32_764)) + &rich("_x00"),
    ];
    let found = vec![
        too_long(2, 32_768),
        too_long(3, 32_768),
        too_long(7, 32_768),
        too_long(10, 32_768),
        too_long(12, 32_768),
        too_long(13, 32_768),
    ];
    (items, found)
}

#[test]
fn every_string_past_the_bound_is_reported_with_its_index() {
    let (items, found) = cases();
    assert_eq!(check(with_items(&items)), found);
}

#[test]
fn only_the_text_of_the_table_counts() {
    let a = "a".repeat(32_768);
    let xml = format!(
        r#"<sst xmlns="{MAIN}" xmlns:o="urn:other"><si><t>a</t><o:t>{a}</o:t><extLst><t>{a}</t></extLst></si><o:si><t>{a}</t></o:si><si><r><rPr><t>{a}</t></rPr><t>b</t></r></si><si/><si><t/></si></sst>"#
    );
    // Four strings, the last two empty: index 3 is the last.
    assert_eq!(check(with_cell(with_table(&xml), 3)), []);
    assert_eq!(check(with_cell(with_table(&xml), 4)), [missing(4)]);
}

#[test]
fn text_counts_however_it_is_written() {
    // CDATA and a reference are text as any other, and an element in no
    // namespace is passed over.
    let a = "a".repeat(32_766);
    let xml = format!(
        r#"<?xml version="1.0"?><sst xmlns="{MAIN}"><si><t><![CDATA[{a}]]>&amp;<x xmlns="">{a}</x></t></si><si><t><![CDATA[{a}]]>&amp;&#97;</t></si></sst>"#
    );
    assert_eq!(check(with_table(&xml)), [too_long(1, 32_768)]);
}

#[test]
fn a_cell_is_checked_against_the_strings_there_are() {
    // No table: there is no string at all.
    assert_eq!(check(with_cell(Book::new(), 0)), [missing(0)]);
    let table = || with_items(&[t("a"), t("b")]);
    assert_eq!(check(with_cell(table(), 1)), []);
    assert_eq!(check(with_cell(table(), 2)), [missing(2)]);
}

#[test]
fn a_table_not_read_to_its_end_is_not_counted() {
    // Not SpreadsheetML's `sst`, or not well-formed: how many strings it
    // holds is not known, and no index is checked against it.
    let other = r#"<sst xmlns="urn:other"><si><t>a</t></si></sst>"#;
    assert_eq!(check(with_cell(with_table(other), 3)), []);
    let broken = format!(r#"<sst xmlns="{MAIN}"><si><t>a</t></si>"#);
    let seen = check(with_cell(with_table(&broken), 3));
    assert_eq!(seen.len(), 1);
    assert!(matches!(seen[0].0, Diagnostic::XmlMalformed { .. }));
    assert_eq!(seen[0].1, PART);
    // Two tables: which one is meant is not known, which is reported once.
    let two = with_items(&[t("a")]).rel("/xl/workbook.xml", "sharedStrings", "/xl/other.xml");
    let seen = check(with_cell(
        two.part("/xl/other.xml", "application/xml", "<sst/>"),
        3,
    ));
    assert_eq!(seen.len(), 1);
    assert!(matches!(
        seen[0].0,
        Diagnostic::RelationshipTypeDuplicate { .. }
    ));
    assert_eq!(seen[0].1, "/xl/_rels/workbook.xml.rels#rId2");
}

#[test]
fn a_table_whose_entry_is_damaged_or_refused_is_not_counted() {
    let xml = format!(r#"<sst xmlns="{MAIN}"><si><t>a</t></si></sst>"#);
    // Its text changed, so it is read to its end as XML.
    let at = xml.find("a</t>").unwrap();
    let damaged = Item::stored(&PART[1..], xml.as_bytes()).damaged(|data| data[at] = b'b');
    let seen = check(with_cell(with_table(&xml).replace(damaged), 3));
    assert_eq!(seen.len(), 1);
    assert!(matches!(seen[0].0, Diagnostic::ZipCrcMismatch { .. }));
    let refused = Item::stored(&PART[1..], xml.as_bytes()).with_method(99);
    let seen = check(with_cell(with_table(&xml).replace(refused), 3));
    assert_eq!(seen.len(), 1);
    assert!(matches!(
        seen[0].0,
        Diagnostic::ZipUnsupportedCompression { method: 99 }
    ));
}

#[test]
fn a_failure_of_the_host_while_reading_the_table_is_an_error() {
    let zip = with_table(&format!(
        r#"<sst xmlns="{MAIN}"><si><t>MARK</t></si></sst>"#
    ))
    .zip();
    let mark = zip.windows(4).position(|w| w == b"MARK").unwrap() as u64;
    let start = mark - format!(r#"<sst xmlns="{MAIN}"><si><t>"#).len() as u64;
    let error = run(
        zip,
        &Limits::default(),
        &Rules::default(),
        (start, start + 1),
    )
    .unwrap_err();
    assert!(matches!(error, VerifyError::Package(_)), "{error:?}");
}
