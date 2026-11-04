//! Tests for `strings.rs`: the length of each string of a shared strings
//! table, read by `xlsx-graft-read` from a package the tests build. What
//! Excel does with each is Excel for Mac 16.114.3's, opening the workbook.

use super::{Book, seen};
use crate::strings::check;
use std::io::Cursor;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_read::{ReadLimits, SharedStrings, Workbook};
use xlsx_graft_spool::MemorySpools;

/// The part the tables are in.
const PART: &str = "/xl/sharedStrings.xml";

/// The shared strings table whose strings are `items`, each an `si`'s
/// content, read from a package, spilled to a spool once its text passes
/// `spill` bytes.
fn table(items: &[String], spill: u64) -> SharedStrings {
    let ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let body: String = items
        .iter()
        .map(|item| format!("<si>{item}</si>"))
        .collect();
    let xml = format!(r#"<sst xmlns="{ns}">{body}</sst>"#);
    let media = "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";
    let zip = (Book::new().part(PART, media, &xml))
        .rel("/xl/workbook.xml", "sharedStrings", PART)
        .zip();
    let mut diagnostics = Diagnostics::new();
    let reader = Cursor::new(zip);
    let mut package = Package::open(reader, &Limits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let limits = ReadLimits {
        shared_strings_spill_bytes: spill,
        ..ReadLimits::default()
    };
    let strings = SharedStrings::read(
        &mut package,
        &workbook,
        &limits,
        &mut MemorySpools,
        &mut diagnostics,
    );
    assert_eq!(seen(diagnostics), []);
    strings.unwrap()
}

/// What checking `strings` finds.
fn checked(mut strings: SharedStrings) -> Vec<(Diagnostic, String)> {
    let mut diagnostics = Diagnostics::new();
    check(&mut strings, PART, &mut diagnostics).unwrap();
    seen(diagnostics)
}

/// `shared_string_too_long` of the string `index`, `length` long.
fn too_long(index: u64, length: u64) -> (Diagnostic, String) {
    (
        Diagnostic::SharedStringTooLong { index, length },
        PART.to_owned(),
    )
}

/// A `t` holding `text`.
fn t(text: &str) -> String {
    format!("<t>{text}</t>")
}

/// A run whose `t` holds `text`.
fn run(text: &str) -> String {
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
        run(&a(16_384)) + &run(&a(16_384)),
        // Runs together at it: Excel opens it as it is.
        run(&a(16_384)) + &run(&a(16_383)),
        // A phonetic run is not counted with the text: Excel opens it as it
        // is.
        t(&a(32_767)) + &phonetic(&"\u{30A2}".repeat(10)),
        // An escape is one character, `_x005F_` one `_`.
        t(&"_x005F_".repeat(32_768)),
        // An unpaired surrogate's escape is kept as written, seven
        // characters, where Excel counts one: Excel for Mac opens it as it
        // is, and this reports it all the same.
        t(&"_xD83D_".repeat(4_682)),
    ];
    let found = vec![
        too_long(2, 32_768),
        too_long(3, 32_768),
        too_long(7, 32_768),
        too_long(10, 32_768),
        too_long(11, 32_774),
    ];
    (items, found)
}

#[test]
fn every_string_past_the_bound_is_reported_with_its_index() {
    let (items, found) = cases();
    assert_eq!(checked(table(&items, u64::MAX)), found);
}

#[test]
fn a_table_spilled_to_a_spool_is_checked_the_same() {
    let (items, found) = cases();
    let strings = table(&items, 1_024);
    assert!(matches!(&strings, SharedStrings::Table(inner) if inner.is_spilled()));
    assert_eq!(checked(strings), found);
}

#[test]
fn a_table_absent_or_unknown_holds_nothing_to_check() {
    assert_eq!(checked(SharedStrings::Absent), []);
    assert_eq!(checked(SharedStrings::Unknown), []);
    assert_eq!(checked(table(&[], u64::MAX)), []);
}
