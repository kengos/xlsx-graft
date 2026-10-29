//! Unit tests for what `read` prints of each value, diagnostic and
//! refusal, in each format, without a workbook. What it prints of one is
//! `tests/bin/read`.

use super::json::Cell;
use super::select::Refusal;
use super::text::{Problem, Text, write_value};
use super::{End, Sink};
use crate::CliError;
use std::io::{self, Write};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit, Location};
use xlsx_graft_read::{DateSystem, FormatKind, SheetKind, Value};

/// `value` as `read --format json` prints a cell of it.
fn json(value: Value<'_>) -> String {
    serde_json::to_string(&Cell(value)).unwrap()
}

/// `value` as `read --format text` prints a cell of it.
fn text(value: Value<'_>) -> String {
    let mut line = String::new();
    write_value(&mut line, value).unwrap();
    line
}

/// Each kind of value, `t="d"` text making the dates and times, and the
/// elapsed time of 1 day, 2 hours and 3 minutes.
fn values() -> [Value<'static>; 9] {
    let elapsed = Value::from_serial(
        1.0 + 2.05 / 24.0,
        FormatKind::Duration,
        DateSystem::Base1900,
    );
    [
        Value::Empty,
        Value::String("a\tb"),
        Value::Number(3.0),
        Value::Bool(true),
        Value::from_iso("2024-01-31").unwrap(),
        Value::from_iso("08:30:00.5").unwrap(),
        Value::from_iso("2024-01-31T08:30:00").unwrap(),
        elapsed,
        Value::Error("#N/A"),
    ]
}

#[test]
fn each_value_is_a_typed_cell_in_json() {
    let cells: Vec<String> = values().into_iter().map(json).collect();
    assert_eq!(
        cells,
        [
            r#"{"type":"empty","value":null}"#,
            r#"{"type":"string","value":"a\tb"}"#,
            r#"{"type":"number","value":3.0}"#,
            r#"{"type":"boolean","value":true}"#,
            r#"{"type":"date","value":"2024-01-31"}"#,
            r#"{"type":"time","value":"08:30:00.500"}"#,
            r#"{"type":"datetime","value":"2024-01-31T08:30:00.000"}"#,
            r#"{"type":"elapsed","value":93780000}"#,
            r##"{"type":"error","value":"#N/A"}"##,
        ]
    );
}

#[test]
fn a_number_in_json_always_has_a_fraction_or_an_exponent() {
    let numbers = [
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (-42.0, "-42.0"),
        (0.1, "0.1"),
        (1e15, "1000000000000000.0"),
        (1e16, "1e+16"),
        (1.5e-7, "1.5e-7"),
        (f64::MAX, "1.7976931348623157e+308"),
        (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
    ];
    for (number, written) in numbers {
        let cell = json(Value::Number(number));
        assert_eq!(cell, format!(r#"{{"type":"number","value":{written}}}"#));
        // It reads back as the same number.
        let read: serde_json::Value = serde_json::from_str(&cell).unwrap();
        assert_eq!(read["value"].as_f64().unwrap().to_bits(), number.to_bits());
    }
}

#[test]
fn each_value_is_written_for_a_person_in_text() {
    let cells: Vec<String> = values().into_iter().map(text).collect();
    assert_eq!(
        cells,
        [
            "",
            "a\\tb",
            "3",
            "TRUE",
            "2024-01-31",
            "08:30:00.500",
            "2024-01-31T08:30:00.000",
            "26:03:00.000",
            "#N/A",
        ]
    );
    assert_eq!(text(Value::Bool(false)), "FALSE");
}

#[test]
fn a_number_in_text_takes_an_exponent_only_far_from_one() {
    let numbers = [
        (0.0, "0"),
        (-0.0, "-0"),
        (0.1, "0.1"),
        (1e-5, "0.00001"),
        (9.9e-6, "9.9e-6"),
        (1e15, "1000000000000000"),
        (-1e16, "-1e16"),
        (123_456.789, "123456.789"),
    ];
    for (number, written) in numbers {
        assert_eq!(text(Value::Number(number)), written, "{number}");
    }
}

#[test]
fn a_string_in_text_never_splits_its_line_or_its_column() {
    let value = Value::String("a\\b\tc\nd\re é");
    assert_eq!(text(value), "a\\\\b\\tc\\nd\\re é");
    assert_eq!(text(Value::Error("#\t")), "#\\t");
}

#[test]
fn a_diagnostic_in_text_says_its_severity_code_place_and_message() {
    let mut diagnostics = Diagnostics::with_limit(Limit::default_of(1));
    let missing = Diagnostic::SharedStringMissing { index: 7 };
    let sheet = Location::default().with_part("/xl/worksheets/sheet1.xml");
    diagnostics.push(missing.clone(), sheet);
    diagnostics.push(missing.clone(), Location::default().with_entry("a.xml"));
    let records = diagnostics.finish();
    let lines: Vec<String> = (records.iter())
        .map(|record| Problem(record).to_string())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        format!(
            "error shared_string_missing in /xl/worksheets/sheet1.xml: {}",
            missing.message()
        )
    );
    // The truncation notice points nowhere in the package.
    assert!(lines[1].starts_with("notice diagnostics_truncated: "));
    let rels = Location::default().with_rels_part("/_rels/.rels");
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(missing, rels.with_relationship_id("rId1"));
    let records = diagnostics.finish();
    assert!(
        Problem(&records[0])
            .to_string()
            .contains(" in /_rels/.rels: ")
    );
}

#[test]
fn each_refusal_has_its_code_and_says_why() {
    let named = |name: &str| name.to_owned();
    let refusals = [
        (
            Refusal::WorkbookUnreadable,
            "workbook_unreadable",
            "the workbook cannot be read; the diagnostics say why",
        ),
        (
            Refusal::NoSuchName(named("X")),
            "sheet_not_found",
            "no sheet is named \"X\"",
        ),
        (
            Refusal::NoSuchIndex { index: 3, count: 2 },
            "sheet_not_found",
            "there is no sheet at index 3: the workbook has 2, from index 0",
        ),
        (
            Refusal::Duplicate(named("A")),
            "sheet_ambiguous",
            "more than one sheet is named \"A\", letter case aside",
        ),
        (
            Refusal::NotWorksheet {
                name: named("C"),
                kind: SheetKind::Chartsheet,
            },
            "sheet_not_worksheet",
            "sheet \"C\" is a chartsheet, which holds no cells",
        ),
        (
            Refusal::NotWorksheet {
                name: named("D"),
                kind: SheetKind::Dialogsheet,
            },
            "sheet_not_worksheet",
            "sheet \"D\" is a dialog sheet, which holds no cells",
        ),
        (
            Refusal::NotWorksheet {
                name: named("M"),
                kind: SheetKind::Macrosheet,
            },
            "sheet_not_worksheet",
            "sheet \"M\" is a macro sheet, whose cells are not read",
        ),
        (
            Refusal::SheetUnreadable(named("S")),
            "sheet_unreadable",
            "sheet \"S\" cannot be read; the diagnostics say why",
        ),
    ];
    for (refusal, code, message) in refusals {
        assert_eq!(refusal.code(), code);
        assert_eq!(refusal.to_string(), message);
        let json = serde_json::to_value(&refusal).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": code, "message": message })
        );
    }
}

#[test]
fn text_flushes_the_rows_before_the_problems_and_passes_a_closed_stderr_over() {
    let mut diagnostics = Diagnostics::new();
    let missing = Diagnostic::SharedStringMissing { index: 7 };
    diagnostics.push(missing, Location::default());
    let end = End {
        error: Some(Refusal::WorkbookUnreadable),
        diagnostics: diagnostics.finish(),
        ..End::default()
    };
    let mut out = io::BufWriter::new(Vec::new());
    out.write_all(b"1\n").unwrap();
    // A stderr that takes nothing, as a closed one or a full disk.
    let mut refusing: &mut [u8] = &mut [];
    Text::new(&mut out, &mut refusing).end(&end).unwrap();
    assert_eq!(out.buffer(), b"");
    assert_eq!(out.get_ref(), b"1\n");
    // A stdout that refuses the rows is still an error.
    let mut full = io::BufWriter::new(&mut [][..]);
    full.write_all(b"1\n").unwrap();
    let err = Text::new(&mut full, &mut io::sink()).end(&end).unwrap_err();
    assert!(matches!(err, CliError::Write(e) if e.kind() == io::ErrorKind::WriteZero));
}
