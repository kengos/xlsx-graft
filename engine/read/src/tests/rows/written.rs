//! What `xlsx_graft_cell::write_cell` and `SharedStringsWriter` write,
//! read back: the cells `write_cell` writes for each type of value, put in
//! a row, read to the values they were written from, and the strings a
//! `SharedStringsWriter` part holds, read from the cells that point to
//! them.

use super::super::rel;
use super::super::zip::Item;
use super::reading::styles;
use super::{SHARED_STRINGS, STYLES, clean, open, read_book, row, workbook, worksheet};
use proptest::prelude::*;
use xlsx_graft_cell::{
    CellRef, CellValue, Date, DateSystem, DateTime, Elapsed, ErrorValue, SharedStringsWriter, Time,
    Written, write_cell,
};
use xlsx_graft_diagnostics::Diagnostics;

/// The cells `cells` (value and cell format, from A1 on) as a `<row r="1">`,
/// each written by `write_cell` in `system`.
fn row_of(cells: &[(CellValue<'_>, Option<u32>)], system: DateSystem) -> String {
    let mut out = b"<row r=\"1\">".to_vec();
    for (column, (value, style)) in (1..).zip(cells) {
        let at = CellRef::new(column, 1).unwrap();
        let written = write_cell(&mut out, at, *style, value, system);
        assert_eq!(written.unwrap(), Written::Whole);
    }
    out.extend_from_slice(b"</row>");
    String::from_utf8(out).unwrap()
}

/// The values of the cells of `body`, a row from A1, each as a string of
/// the form `shown` gives, in a workbook with the styles of
/// `reading::styles` (General, then the built-in date, time, date-time and
/// elapsed time formats), counting in 1904 when `date1904`.
fn styled(body: &str, date1904: bool) -> Vec<String> {
    let sheet = worksheet(body);
    let related = rel("rId9", STYLES, "styles.xml");
    let items = workbook(&[("Sheet1", &sheet)], date1904, &related, vec![styles()]);
    let mut diagnostics = Diagnostics::new();
    let book = open(items, &mut diagnostics);
    let (mut rows, seen) = read_book(&book, diagnostics);
    assert_eq!(seen, vec![]);
    rows.remove(0).1
}

#[test]
fn a_value_that_is_not_a_date_reads_back_as_the_value_it_was_written_from() {
    let cells = [
        (CellValue::Number(1.5), None),
        (CellValue::Number(-0.0), Some(0)),
        (CellValue::Number(1e21), None),
        (CellValue::Number(1.5e-7), None),
        (CellValue::Integer(-9_007_199_254_740_992), None),
        (CellValue::Bool(true), None),
        (CellValue::Bool(false), Some(1)),
        (CellValue::Error(ErrorValue::DivZero), None),
        (CellValue::Error(ErrorValue::GettingData), None),
        (CellValue::String("=A1"), None),
        (CellValue::String("00123"), None),
        (CellValue::String("2024-01-01"), None),
        (CellValue::String(" a\u{1}\r\n_x0041_ & <b> "), None),
        (CellValue::String(""), None),
        (CellValue::SharedString(1), None),
    ];
    let body = row_of(&cells, DateSystem::Compatibility1900);
    assert_eq!(
        clean(&body),
        vec![row(
            1,
            &[
                "n:1.5",
                "n:0",
                "n:1000000000000000000000",
                "n:0.00000015",
                "n:-9007199254740992",
                "b:true",
                "b:false",
                "e:#DIV/0!",
                "e:#GETTING_DATA",
                "s:=A1",
                "s:00123",
                "s:2024-01-01",
                "s: a\u{1}\r\n_x0041_ & <b> ",
                "s:",
                "s:b",
            ]
        )]
    );
}

#[test]
fn every_error_value_reads_back() {
    let errors = [
        (ErrorValue::Null, "#NULL!"),
        (ErrorValue::DivZero, "#DIV/0!"),
        (ErrorValue::Value, "#VALUE!"),
        (ErrorValue::Ref, "#REF!"),
        (ErrorValue::Name, "#NAME?"),
        (ErrorValue::Num, "#NUM!"),
        (ErrorValue::NotAvailable, "#N/A"),
        (ErrorValue::GettingData, "#GETTING_DATA"),
    ];
    let cells: Vec<_> = (errors.iter())
        .map(|&(error, _)| (CellValue::Error(error), None))
        .collect();
    let expected: Vec<String> = errors.iter().map(|(_, text)| format!("e:{text}")).collect();
    let body = row_of(&cells, DateSystem::Compatibility1900);
    assert_eq!(clean(&body)[0].1, expected);
}

#[test]
fn a_date_or_time_reads_back_under_its_format_in_each_date_system() {
    let date = Date::from_iso("2024-01-02").unwrap();
    let date_time = DateTime::from_iso("2024-01-02T03:04:05.678").unwrap();
    let time = Time::from_iso("18:00:00").unwrap();
    let elapsed = Elapsed::from_serial(1.5, DateSystem::Base1900).unwrap();
    // Cell formats 1 to 4: date, time, date-time and elapsed time.
    let cells = [
        (CellValue::Date(date), Some(1)),
        (CellValue::Time(time), Some(2)),
        (CellValue::DateTime(date_time), Some(3)),
        (CellValue::Elapsed(elapsed), Some(4)),
    ];
    for (system, date1904) in [
        (DateSystem::Compatibility1900, false),
        (DateSystem::Base1904, true),
    ] {
        assert_eq!(
            styled(&row_of(&cells, system), date1904),
            [
                "date:2024-01-02",
                "time:18:00:00.000",
                "dt:2024-01-02T03:04:05.678",
                "el:129600000",
            ],
            "{system:?}"
        );
    }
}

#[test]
fn a_date_in_compatibility_1900_reads_back_around_serial_60() {
    let cells: Vec<_> = ["1900-01-01", "1900-02-28", "1900-03-01"]
        .iter()
        .map(|iso| (CellValue::Date(Date::from_iso(iso).unwrap()), Some(1)))
        .collect();
    let body = row_of(&cells, DateSystem::Compatibility1900);
    assert_eq!(
        styled(&body, false),
        ["date:1900-01-01", "date:1900-02-28", "date:1900-03-01"]
    );
}

#[test]
fn the_strings_of_a_shared_strings_writer_read_back_from_cells_that_point_to_them() {
    let strings = [" a ", "_x0041_", "b\u{1}c", "\r\nd", "", "😀&<"];
    let mut writer = SharedStringsWriter::new(Vec::new()).unwrap();
    let mut cells = Vec::new();
    for text in strings {
        let (number, written) = writer.push(text).unwrap();
        assert_eq!(written, Written::Whole);
        cells.push((CellValue::SharedString(number), None));
    }
    let part = writer.finish().unwrap();
    let sheet = worksheet(&row_of(&cells, DateSystem::Compatibility1900));
    let related = rel("rId9", SHARED_STRINGS, "sharedStrings.xml");
    let more = vec![Item::stored("xl/sharedStrings.xml", &part)];
    let items = workbook(&[("Sheet1", &sheet)], false, &related, more);
    let mut diagnostics = Diagnostics::new();
    let book = open(items, &mut diagnostics);
    let (rows, seen) = read_book(&book, diagnostics);
    assert_eq!(seen, vec![]);
    let expected: Vec<String> = strings.iter().map(|text| format!("s:{text}")).collect();
    assert_eq!(rows[0].1, expected);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn a_string_reads_back_as_the_string_it_was_written_from(text in any::<String>()) {
        let body = row_of(&[(CellValue::String(&text), None)], DateSystem::Compatibility1900);
        prop_assert_eq!(clean(&body), vec![row(1, &[&format!("s:{text}")])]);
    }

    #[test]
    fn a_string_of_the_characters_escapes_turn_on_reads_back(
        chars in prop::collection::vec(
            prop::sample::select(&[
                '_', '_', 'x', 'X', '0', 'A', 'f', ' ', '\t', '\n', '\r', '\u{1}', '\u{fffe}',
                '\u{ffff}', '&', '<', '>', '"', '😀', 'あ',
            ][..]),
            0..30,
        ),
    ) {
        let text: String = chars.into_iter().collect();
        let body = row_of(&[(CellValue::String(&text), None)], DateSystem::Compatibility1900);
        prop_assert_eq!(clean(&body), vec![row(1, &[&format!("s:{text}")])]);
    }
}
