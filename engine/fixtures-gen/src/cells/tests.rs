//! The workbooks `cells` writes, read back with `xlsx-graft-read`: each
//! row's value against the sample it was written from, the date system of
//! each workbook, and the command lines and failures `run` returns an error
//! for.

use super::rows::{SHARED, Sample, rows};
use crate::tests::Refuses;
use crate::{FixturesGenError, run};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::Path;
use xlsx_graft_cell::{DateSystem, ErrorValue};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits as PackageLimits, Package};
use xlsx_graft_read::{Book, ReadLimits, Row as ReadRow, Value};
use xlsx_graft_spool::TempFiles;

/// The workbooks `cells` writes and the date system each is in.
const BOOKS: [(&str, DateSystem); 3] = [
    ("cells-1900.xlsx", DateSystem::Compatibility1900),
    ("cells-1900-strict.xlsx", DateSystem::Base1900),
    ("cells-1904.xlsx", DateSystem::Base1904),
];

fn args(words: &[&Path]) -> Vec<OsString> {
    words
        .iter()
        .map(|word| word.as_os_str().to_owned())
        .collect()
}

/// Runs `cells` into `dir`, returning what it printed.
fn write_books(dir: &Path) -> String {
    let mut out = Vec::new();
    run(&args(&[Path::new("cells"), dir]), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// What reading the workbook at `path` gives: its date system, its sheets'
/// names, each row's column A and B as `(row number, A, B)` of the first
/// sheet with the values as `Debug` prints them, and the codes of the
/// diagnostics.
struct ReadBack {
    date_system: DateSystem,
    sheets: Vec<String>,
    rows: Vec<(u32, String, String)>,
    diagnostics: Vec<String>,
}

fn read_back(path: &Path) -> ReadBack {
    let bytes = fs::read(path).unwrap();
    let mut diagnostics = Diagnostics::new();
    let book = Book::open(
        Cursor::new(bytes),
        &PackageLimits::default(),
        &ReadLimits::default(),
        &mut TempFiles::new(),
        &mut diagnostics,
    )
    .unwrap();
    let Some(book) = book else {
        let codes: Vec<_> = (diagnostics.records().iter())
            .map(|r| r.diagnostic.code().as_str())
            .collect();
        panic!("{}: the book is not read: {codes:?}", path.display());
    };
    let mut read_rows = book.rows(0).unwrap();
    let mut row = ReadRow::new();
    let mut rows = Vec::new();
    while read_rows.next_row(&mut row, &mut diagnostics).unwrap() {
        let mut values = row.values();
        let label = format!("{:?}", values.next().unwrap());
        let value = format!("{:?}", values.next().unwrap_or(Value::Empty));
        rows.push((row.number(), label, value));
    }
    ReadBack {
        date_system: book.date_system(),
        sheets: book.sheets().iter().map(|s| s.name().to_owned()).collect(),
        rows,
        diagnostics: (diagnostics.records().iter())
            .map(|r| r.diagnostic.code().as_str().to_owned())
            .collect(),
    }
}

/// `text` cut to `units` UTF-16 code units, at a character boundary.
fn cut(text: &str, units: usize) -> String {
    let mut kept = String::new();
    let mut length = 0;
    for c in text.chars() {
        length += c.len_utf16();
        if length > units {
            break;
        }
        kept.push(c);
    }
    kept
}

/// The text of an error value, written as `<v>`.
fn error_text(error: ErrorValue) -> &'static str {
    match error {
        ErrorValue::Null => "#NULL!",
        ErrorValue::DivZero => "#DIV/0!",
        ErrorValue::Value => "#VALUE!",
        ErrorValue::Ref => "#REF!",
        ErrorValue::Name => "#NAME?",
        ErrorValue::Num => "#NUM!",
        ErrorValue::NotAvailable => "#N/A",
        ErrorValue::GettingData => "#GETTING_DATA",
        _ => "an error value the tests do not know",
    }
}

/// The rows of a workbook in `system` that `write_cell` refuses, as the
/// description in column A and the text in column B: `refused: ` and why.
/// The texts are written by hand, not taken from `CellError`'s `Display`.
fn refused(system: DateSystem) -> Vec<(&'static str, &'static str)> {
    let number = "refused: the number is not finite";
    let integer = "refused: the integer is past 2^53";
    let date = "refused: the date or time has no serial date-time in the date system";
    let mut rows = vec![
        ("number NaN", number),
        ("number inf", number),
        ("integer 9007199254740993", integer),
    ];
    let before_1900 = ["date 1899-12-30", "date 1899-12-31"];
    let before_1904 = [
        "date 1899-12-30",
        "date 1899-12-31",
        "date 1900-01-01",
        "date 1900-02-28",
        "date 1900-03-01",
        "date-time 1900-01-01T00:00:00",
        "date-time 1900-02-28T23:59:59.999",
        "date-time 1900-03-01T00:00:00",
    ];
    let dates: &[&str] = match system {
        DateSystem::Compatibility1900 => &before_1900,
        DateSystem::Base1904 => &before_1904,
        _ => &[],
    };
    rows.extend(dates.iter().map(|label| (*label, date)));
    rows
}

/// The value the reader gives for `sample`, a row that `write_cell` does not
/// refuse, as `Debug` prints it.
fn expected(sample: &Sample) -> String {
    let text;
    let value = match sample {
        Sample::Text(string) => {
            text = cut(string, 32_767);
            Value::String(&text)
        }
        Sample::Number(number) => Value::Number(if *number == 0.0 { 0.0 } else { *number }),
        Sample::Integer(integer) => Value::Number(*integer as f64),
        Sample::Bool(flag) => Value::Bool(*flag),
        Sample::Error(error) => Value::Error(error_text(*error)),
        Sample::Date(date) => Value::Date(*date),
        Sample::Time(time) => Value::Time(*time),
        Sample::DateTime(date_time) => Value::DateTime(*date_time),
        Sample::Elapsed(elapsed) => Value::Elapsed(*elapsed),
        Sample::Shared(index) => Value::String(SHARED[usize::try_from(*index).unwrap()]),
        Sample::Empty => Value::Empty,
    };
    format!("{value:?}")
}

#[test]
fn each_workbook_reads_back_in_its_date_system_with_every_row_as_its_sample_expects() {
    let dir = tempfile::tempdir().unwrap();
    write_books(&dir.path().join("nested").join("cells"));
    for (name, system) in BOOKS {
        let path = dir.path().join("nested").join("cells").join(name);
        let read = read_back(&path);
        assert_eq!(read.date_system, system, "{name}");
        assert_eq!(read.sheets, ["Cells"], "{name}");
        assert!(
            read.diagnostics.is_empty(),
            "{name}: {:?}",
            read.diagnostics
        );
        let samples = rows(system).unwrap();
        assert_eq!(read.rows.len(), samples.len() + 1, "{name}");
        let header = ("String(\"description\")", "String(\"value\")");
        assert_eq!((read.rows[0].1.as_str(), read.rows[0].2.as_str()), header);
        for (row, (number, label, value)) in samples.iter().zip(&read.rows[1..]) {
            let at = format!("{name} row {number}");
            assert_eq!(*label, format!("{:?}", Value::String(&row.label)), "{at}");
            let refusal = refused(system).into_iter().find(|(l, _)| *l == row.label);
            let want = match refusal {
                Some((_, why)) => format!("{:?}", Value::String(why)),
                None => expected(&row.sample),
            };
            assert_eq!(*value, want, "{at}: {}", row.label);
        }
        let numbers: Vec<u32> = read.rows.iter().map(|r| r.0).collect();
        assert_eq!(numbers, (1..=samples.len() as u32 + 1).collect::<Vec<_>>());
    }
}

#[test]
fn run_prints_the_path_of_each_workbook_written_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let printed = write_books(dir.path());
    let expected: String = BOOKS
        .iter()
        .map(|(name, _)| format!("{}\n", dir.path().join(name).display()))
        .collect();
    assert_eq!(printed, expected);
}

#[test]
fn long_strings_are_cut_to_32767_code_units_without_splitting_a_surrogate_pair() {
    let dir = tempfile::tempdir().unwrap();
    write_books(dir.path());
    let read = read_back(&dir.path().join("cells-1900.xlsx"));
    let units = |label: &str| -> usize {
        let row = read.rows.iter().find(|r| r.1.contains(label)).unwrap();
        let text = &row.2["String(\"".len()..row.2.len() - "\")".len()];
        text.chars().map(char::len_utf16).sum()
    };
    assert_eq!(units("40,000 x"), 32_767);
    assert_eq!(units("a and 16,384"), 32_767);
    assert_eq!(units("of 16,384 emoji, 32,768"), 32_766);
}

/// An input of 32,767 code units or fewer is written whole, so a sample one
/// emoji shorter would no longer test the cut. The unit at index 32,766 is
/// a high surrogate in 16,384 emoji and a low surrogate in "a" and 16,384
/// emoji.
#[test]
fn the_long_string_samples_are_40000_32769_and_32768_code_units_with_a_surrogate_pair_around_unit_32767()
 {
    let samples = rows(DateSystem::Base1900).unwrap();
    let units = |label: &str| -> Vec<u16> {
        let row = samples.iter().find(|r| r.label.contains(label)).unwrap();
        let Sample::Text(text) = &row.sample else {
            panic!("{label}: not a string sample");
        };
        text.encode_utf16().collect()
    };
    let x = units("40,000 x");
    assert_eq!(x.len(), 40_000);
    let a_emoji = units("a and 16,384 emoji");
    assert_eq!(a_emoji.len(), 32_769);
    assert!((0xDC00..=0xDFFF).contains(&a_emoji[32_766]));
    let emoji = units("of 16,384 emoji, 32,768");
    assert_eq!(emoji.len(), 32_768);
    assert!((0xD800..=0xDBFF).contains(&emoji[32_766]));
}

#[test]
fn the_rows_a_date_system_has_no_serial_date_time_for_are_exactly_those_refused_lists() {
    let dir = tempfile::tempdir().unwrap();
    write_books(dir.path());
    for (name, system) in BOOKS {
        let read = read_back(&dir.path().join(name));
        let mut found: Vec<String> = (read.rows.iter())
            .filter(|row| row.2.starts_with("String(\"refused: "))
            .map(|row| row.1.clone())
            .collect();
        let mut table: Vec<String> = (refused(system).iter())
            .map(|(label, _)| format!("{:?}", Value::String(label)))
            .collect();
        found.sort();
        table.sort();
        assert_eq!(found, table, "{name}");
    }
}

#[test]
fn the_samples_have_the_number_of_rows_of_each_kind_in_every_date_system() {
    for (_, system) in BOOKS {
        let samples = rows(system).unwrap();
        let count = |kind: fn(&Sample) -> bool| samples.iter().filter(|r| kind(&r.sample)).count();
        let counts = [
            count(|s| matches!(s, Sample::Text(_))),
            count(|s| matches!(s, Sample::Number(_))),
            count(|s| matches!(s, Sample::Integer(_))),
            count(|s| matches!(s, Sample::Bool(_))),
            count(|s| matches!(s, Sample::Error(_))),
            count(|s| matches!(s, Sample::Empty)),
            count(|s| matches!(s, Sample::Date(_))),
            count(|s| matches!(s, Sample::Time(_))),
            count(|s| matches!(s, Sample::DateTime(_))),
            count(|s| matches!(s, Sample::Elapsed(_))),
            count(|s| matches!(s, Sample::Shared(_))),
        ];
        assert_eq!(counts, [24, 10, 5, 2, 8, 1, 8, 3, 5, 7, 5], "{system:?}");
        assert_eq!(samples.len(), 78, "{system:?}");
    }
}

#[test]
fn the_number_and_integer_samples_hold_these_values() {
    let samples = rows(DateSystem::Base1900).unwrap();
    let numbers: Vec<f64> = (samples.iter())
        .filter_map(|r| {
            if let Sample::Number(n) = r.sample {
                Some(n)
            } else {
                None
            }
        })
        .collect();
    assert!(numbers[1] == 0.0 && numbers[1].is_sign_negative());
    assert_eq!(numbers[0], 0.1);
    assert_eq!(numbers[2], f64::MAX);
    assert_eq!(numbers[3], 5E-324);
    assert_eq!(numbers[4], 1E-7);
    assert_eq!(numbers[5], 1.5E-7);
    assert_eq!(numbers[6], 1E+21);
    assert_eq!(numbers[7], 123_456_789_012_345_680_000.0);
    assert!(numbers[8].is_nan());
    assert_eq!(numbers[9], f64::INFINITY);
    let integers: Vec<i64> = (samples.iter())
        .filter_map(|r| {
            if let Sample::Integer(n) = r.sample {
                Some(n)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        integers,
        [
            0,
            -1,
            9_007_199_254_740_992,
            -9_007_199_254_740_992,
            9_007_199_254_740_993
        ]
    );
}

#[test]
fn the_elapsed_samples_hold_these_milliseconds_in_each_date_system() {
    let day = 86_400_000_u64;
    let latest_1900 = 2_958_465 * day + 86_399_999;
    let latest = [
        (DateSystem::Compatibility1900, latest_1900),
        (DateSystem::Base1900, latest_1900),
        (DateSystem::Base1904, latest_1900 - 1_462 * day),
    ];
    for (system, latest) in latest {
        let samples = rows(system).unwrap();
        let elapsed: Vec<(&str, u64)> = (samples.iter())
            .filter_map(|r| match &r.sample {
                Sample::Elapsed(e) => Some((r.label.as_str(), e.milliseconds())),
                _ => None,
            })
            .collect();
        assert_eq!(
            elapsed,
            [
                ("elapsed time 0.5 days", 43_200_000),
                ("elapsed time 1.5 days", 129_600_000),
                ("elapsed time 59.5 days", 5_140_800_000),
                ("elapsed time 60 days", 5_184_000_000),
                ("elapsed time 61 days", 5_270_400_000),
                ("elapsed time 1000.25 days", 86_421_600_000),
                ("elapsed time up to 9999-12-31T23:59:59.999", latest),
            ],
            "{system:?}"
        );
    }
}

#[test]
fn run_without_exactly_one_directory_returns_the_usage_error() {
    let mut out = Vec::new();
    for words in [
        &[Path::new("cells")][..],
        &[Path::new("cells"), Path::new("a"), Path::new("b")],
    ] {
        let error = run(&args(words), &mut out).unwrap_err();
        assert!(matches!(error, FixturesGenError::Usage), "{error}");
        assert_eq!(error.exit_status(), 2);
    }
    assert!(out.is_empty());
}

#[test]
fn run_returns_an_io_error_for_a_directory_that_is_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file");
    File::create(&file).unwrap();
    let error = run(&args(&[Path::new("cells"), &file]), &mut Vec::new()).unwrap_err();
    assert!(matches!(&error, FixturesGenError::Io { path, .. } if *path == file));
    assert_eq!(error.exit_status(), 1);
    assert!(
        error
            .to_string()
            .starts_with(&format!("{}: ", file.display()))
    );
}

#[test]
fn run_returns_an_output_error_when_writing_the_path_to_the_output_fails() {
    let dir = tempfile::tempdir().unwrap();
    let error = run(&args(&[Path::new("cells"), dir.path()]), &mut Refuses).unwrap_err();
    assert!(matches!(error, FixturesGenError::Output(_)));
    assert_eq!(error.exit_status(), 1);
    assert_eq!(error.to_string(), "writing to the output failed: refused");
    assert!(dir.path().join("cells-1900.xlsx").exists());
}

#[test]
fn run_returns_a_cells_error_when_a_workbook_cannot_be_renamed_into_place() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("cells-1900.xlsx")).unwrap();
    let error = run(&args(&[Path::new("cells"), dir.path()]), &mut Vec::new()).unwrap_err();
    let FixturesGenError::Cells { path, .. } = &error else {
        panic!("{error}");
    };
    let taken = dir.path().join("cells-1900.xlsx");
    assert_eq!(path, &taken);
    assert_eq!(error.exit_status(), 1);
    let text = error.to_string();
    let detail = text.strip_prefix(&format!("{}: ", taken.display()));
    assert!(detail.is_some_and(|detail| !detail.is_empty()), "{text}");
}

/// The description in column A of each row of a workbook in
/// `Compatibility1900`, written by hand. A string's, a date's, a time's, a
/// date-time's and an error value's description names the sample's value, so
/// when a sample's value is changed, its description no longer equals the one
/// in this list and the test fails.
const LABELS: [&str; 78] = [
    r##"string "plain""##,
    r##"string """##,
    r##"string "\u{1}\u{8}\u{1f}""##,
    r##"string "\u{fffe}""##,
    r##"string "_x0041_""##,
    r##"string "_xABCD\u{1}""##,
    r##"string "a\rb""##,
    r##"string "a\r\nb""##,
    r##"string "line1\nline2""##,
    r##"string " leading""##,
    r##"string "trailing ""##,
    r##"string "\ttab""##,
    r##"string "=1+2""##,
    r##"string "=SUM(A1:A2)""##,
    r##"string "00123""##,
    r##"string "2024-01-01""##,
    r##"string "TRUE""##,
    r##"string "#N/A""##,
    r##"string "<&>\"'""##,
    r##"string "日本語""##,
    r##"string "😀""##,
    "string of 40,000 x, cut to 32,767 UTF-16 code units",
    "string of a and 16,384 emoji, 32,769 code units, cut to 32,767",
    "string of 16,384 emoji, 32,768 code units, cut to 32,766 so that a surrogate pair is not split",
    "number 0.1",
    "number -0.0",
    "number 1.7976931348623157e308",
    "number 5e-324",
    "number 1e-7",
    "number 1.5e-7",
    "number 1e21",
    "number 1.2345678901234568e20",
    "number NaN",
    "number inf",
    "integer 0",
    "integer -1",
    "integer 9007199254740992",
    "integer -9007199254740992",
    "integer 9007199254740993",
    "boolean true",
    "boolean false",
    "error Null",
    "error DivZero",
    "error Value",
    "error Ref",
    "error Name",
    "error Num",
    "error NotAvailable",
    "error GettingData",
    "empty cell",
    "date 1899-12-30",
    "date 1899-12-31",
    "date 1900-01-01",
    "date 1900-02-28",
    "date 1900-03-01",
    "date 1904-01-01",
    "date 2024-02-29",
    "date 9999-12-31",
    "time 00:00:00",
    "time 12:34:56.789",
    "time 23:59:59.999",
    "date-time 1900-01-01T00:00:00",
    "date-time 1900-02-28T23:59:59.999",
    "date-time 1900-03-01T00:00:00",
    "date-time 2024-02-29T12:34:56.789",
    "date-time 9999-12-31T23:59:59.999",
    "elapsed time 0.5 days",
    "elapsed time 1.5 days",
    "elapsed time 59.5 days",
    "elapsed time 60 days",
    "elapsed time 61 days",
    "elapsed time 1000.25 days",
    "elapsed time up to 9999-12-31T23:59:59.999",
    r##"shared string 0 "shared plain""##,
    r##"shared string 1 "shared _x0041_""##,
    r##"shared string 2 "shared \u{1}""##,
    r##"shared string 3 " shared preserve ""##,
    r##"shared string 4 "shared a\rb""##,
];

#[test]
fn the_descriptions_of_the_rows_are_these_in_order() {
    let samples = rows(DateSystem::Compatibility1900).unwrap();
    let labels: Vec<&str> = samples.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, LABELS);
}

/// The text of `xl/worksheets/sheet1.xml` of the workbook at `path`.
fn worksheet(path: &Path) -> String {
    let mut diagnostics = Diagnostics::new();
    let file = File::open(path).unwrap();
    let package = Package::open(file, &PackageLimits::default(), &mut diagnostics);
    let mut archive = package.unwrap().unwrap().into_archive();
    let index = archive.position(b"xl/worksheets/sheet1.xml").unwrap();
    let mut entry = archive
        .open_entry(index, &mut diagnostics)
        .unwrap()
        .unwrap();
    let mut xml = String::new();
    entry.read_to_string(&mut xml).unwrap();
    xml
}

#[test]
fn a_cell_written_as_refused_has_no_cell_format_though_its_value_had_one() {
    let dir = tempfile::tempdir().unwrap();
    write_books(dir.path());
    for (name, system) in BOOKS {
        let xml = worksheet(&dir.path().join(name));
        let refused_cells: Vec<&str> = (xml.split("<c ").skip(1))
            .filter(|cell| cell.contains("refused: "))
            .collect();
        let dates = refused(system)
            .iter()
            .filter(|(l, _)| l.starts_with("date"))
            .count();
        assert_eq!(refused_cells.len(), refused(system).len(), "{name}");
        assert!(dates > 0 || system == DateSystem::Base1900, "{name}");
        for cell in refused_cells {
            let start_tag = &cell[..cell.find('>').unwrap()];
            assert!(!start_tag.contains(" s=\""), "{name}: {cell}");
        }
    }
}
