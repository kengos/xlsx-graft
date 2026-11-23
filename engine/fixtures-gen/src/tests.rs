//! The workbooks `run`, `write_rows_book` and `write_shared_book` write,
//! read back with `xlsx-graft-read`, and the command lines and failures
//! `run` returns an error for.

use crate::{
    BookSpec, DEFAULT_BOOKS, FixturesGenError, Kind, MAX_ROWS, parse, run, write_rows_book,
    write_shared_book,
};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::Limits as PackageLimits;
use xlsx_graft_read::{Book, ReadLimits, Row, Value};
use xlsx_graft_spool::TempFiles;

fn args(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// A cell's value as one string: a string in quotes, a number as Rust
/// prints an `f64`, a boolean as `true` or `false`, a date in ISO 8601.
fn cell(value: &Value) -> String {
    match value {
        Value::String(text) => format!("{text:?}"),
        Value::Number(number) => format!("{number}"),
        Value::Bool(flag) => format!("{flag}"),
        Value::Date(date) => format!("{date}"),
        other => format!("unexpected {other:?}"),
    }
}

/// What reading the workbook at `path` gives: its date system, each
/// sheet's name, each row of each sheet as `<sheet>!<row>: <cell> | ...`,
/// then a line per diagnostic.
fn read_back(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let mut diagnostics = Diagnostics::new();
    let book = Book::open(
        Cursor::new(bytes),
        &PackageLimits::default(),
        &ReadLimits::default(),
        &mut TempFiles::new(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    let mut lines = vec![format!("date system {:?}", book.date_system())];
    for (index, sheet) in book.sheets().iter().enumerate() {
        lines.push(format!("sheet {:?}", sheet.name()));
        let mut rows = book.rows(index).unwrap();
        let mut row = Row::new();
        while rows.next_row(&mut row, &mut diagnostics).unwrap() {
            let cells: Vec<String> = row.values().map(|value| cell(&value)).collect();
            lines.push(format!(
                "{}!{}: {}",
                sheet.name(),
                row.number(),
                cells.join(" | ")
            ));
        }
    }
    let records = diagnostics.records();
    lines.extend(
        records
            .iter()
            .map(|r| format!("diagnostic {}", r.diagnostic.code().as_str())),
    );
    lines
}

/// Whether the workbook at `path` has a shared strings part: a zip stores
/// each entry's name uncompressed, so the name is in the file's bytes.
fn has_shared_strings(path: &Path) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let name = b"xl/sharedStrings.xml";
    bytes.windows(name.len()).any(|window| window == name)
}

#[test]
fn write_rows_book_writes_the_header_and_each_data_row_the_reader_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book.xlsx");
    write_rows_book(&path, 3).unwrap();
    assert_eq!(
        read_back(&path),
        [
            "date system Compatibility1900",
            "sheet \"Data\"",
            "Data!1: \"number\" | \"fraction\" | \"text\" | \"flag\" | \"date\"",
            "Data!2: 1 | 0.25 | \"text 1\" | false | 2024-01-01",
            "Data!3: 2 | 0.5 | \"text 2\" | true | 2024-01-02",
            "Data!4: 3 | 0.75 | \"text 3\" | false | 2024-01-03",
        ]
    );
}

#[test]
fn write_rows_book_gives_data_row_367_the_date_of_data_row_1() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book.xlsx");
    write_rows_book(&path, 367).unwrap();
    let lines = read_back(&path);
    assert_eq!(lines.len(), 2 + 1 + 367);
    assert!(
        lines[2 + 366].ends_with(" | 2024-12-31"),
        "{}",
        lines[2 + 366]
    );
    assert!(
        lines[2 + 367].ends_with(" | 2024-01-01"),
        "{}",
        lines[2 + 367]
    );
}

#[test]
fn write_shared_book_writes_the_header_and_each_data_row_the_reader_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book.xlsx");
    write_shared_book(&path, 3).unwrap();
    assert_eq!(
        read_back(&path),
        [
            "date system Compatibility1900",
            "sheet \"Data\"",
            "Data!1: \"number\" | \"first\" | \"second\"",
            "Data!2: 1 | \"row 0000001: the first of two strings in the row\" | \"row 0000001: the second of two strings in the row\"",
            "Data!3: 2 | \"row 0000002: the first of two strings in the row\" | \"row 0000002: the second of two strings in the row\"",
            "Data!4: 3 | \"row 0000003: the first of two strings in the row\" | \"row 0000003: the second of two strings in the row\"",
        ]
    );
}

#[test]
fn a_shared_book_has_a_shared_strings_part_and_a_rows_book_has_none() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path().join("shared.xlsx");
    let rows = dir.path().join("rows.xlsx");
    write_shared_book(&shared, 2).unwrap();
    write_rows_book(&rows, 2).unwrap();
    assert!(has_shared_strings(&shared));
    assert!(!has_shared_strings(&rows));
}

#[test]
fn write_shared_book_returns_an_error_for_a_path_in_a_directory_that_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing").join("book.xlsx");
    assert!(write_shared_book(&path, 1).is_err());
}

#[test]
fn write_rows_book_returns_an_error_for_a_path_in_a_directory_that_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing").join("book.xlsx");
    assert!(write_rows_book(&path, 1).is_err());
}

#[test]
fn run_writes_a_workbook_per_book_given_and_prints_each_path() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated");
    let mut out = Vec::new();
    let words = [out_dir.to_str().unwrap(), "shared-2", "rows-1"];
    run(&args(&words), &mut out).unwrap();
    let two = out_dir.join("shared-2.xlsx");
    let one = out_dir.join("rows-1.xlsx");
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!("{}\n{}\n", two.display(), one.display())
    );
    assert_eq!(read_back(&two).len(), 2 + 1 + 2);
    assert_eq!(read_back(&one).len(), 2 + 1 + 1);
    assert!(has_shared_strings(&two));
    assert!(!has_shared_strings(&one));
    let mut names: Vec<String> = std::fs::read_dir(&out_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["rows-1.xlsx", "shared-2.xlsx"]);
}

#[test]
fn parse_gives_the_default_books_for_a_directory_alone() {
    let (dir, books) = parse(&args(&["out"])).unwrap();
    assert_eq!(dir, PathBuf::from("out"));
    assert_eq!(books, DEFAULT_BOOKS);
    let names: Vec<String> = DEFAULT_BOOKS.iter().map(BookSpec::file_name).collect();
    assert_eq!(
        names,
        [
            "rows-100000.xlsx",
            "rows-1000000.xlsx",
            "shared-100000.xlsx",
            "shared-1000000.xlsx"
        ]
    );
}

#[test]
fn parse_takes_books_with_1_to_max_rows_and_refuses_others() {
    let max = format!("shared-{MAX_ROWS}");
    let (_, books) = parse(&args(&["out", "rows-1", &max])).unwrap();
    assert_eq!(
        books,
        [
            BookSpec {
                kind: Kind::Rows,
                rows: 1
            },
            BookSpec {
                kind: Kind::Shared,
                rows: MAX_ROWS
            }
        ]
    );
    let past = format!("shared-{}", MAX_ROWS + 1);
    for word in [
        "shared-0",
        past.as_str(),
        "rows-",
        "cells-1",
        "1",
        "shared-1.5",
        "shared--1",
        "rows-+1",
        "",
    ] {
        let err = parse(&args(&["out", word])).unwrap_err();
        assert!(
            matches!(&err, FixturesGenError::Book(arg) if arg == word),
            "{word}"
        );
        assert_eq!(err.exit_status(), 2);
        assert_eq!(
            err.to_string(),
            format!("{word:?} is not rows-<n> or shared-<n> with <n> from 1 to 1048575")
        );
    }
}

#[cfg(unix)]
#[test]
fn parse_refuses_a_book_that_is_not_utf_8() {
    use std::os::unix::ffi::OsStringExt;
    let words = vec![OsString::from("out"), OsString::from_vec(vec![0xff])];
    let err = parse(&words).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Book(arg) if arg == "\u{fffd}"));
}

#[test]
fn book_spec_parse_reads_the_kind_and_the_rows() {
    let spec = BookSpec::parse(OsStr::new("shared-12")).unwrap();
    assert_eq!(spec.kind, Kind::Shared);
    assert_eq!(spec.rows, 12);
    assert_eq!(spec.file_name(), "shared-12.xlsx");
}

#[test]
fn run_refuses_a_command_line_without_a_directory() {
    let err = run(&[], &mut Vec::new()).unwrap_err();
    assert!(matches!(err, FixturesGenError::Usage));
    assert_eq!(err.exit_status(), 2);
    assert_eq!(
        err.to_string(),
        "usage: xlsx-graft-fixtures-gen <dir> [rows-<n>|shared-<n>...] | repack <fixtures-dir> <dir>"
    );
}

#[test]
fn run_returns_an_io_error_when_the_directory_is_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file");
    std::fs::write(&file, b"").unwrap();
    let err = run(&args(&[file.to_str().unwrap(), "rows-1"]), &mut Vec::new()).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Io { path, .. } if *path == file));
    assert_eq!(err.exit_status(), 1);
    assert!(
        err.to_string()
            .starts_with(&format!("{}: ", file.display()))
    );
}

#[test]
fn run_returns_an_io_error_when_a_directory_is_at_the_workbook_path() {
    let dir = tempfile::tempdir().unwrap();
    let taken = dir.path().join("rows-1.xlsx");
    std::fs::create_dir(&taken).unwrap();
    std::fs::write(taken.join("keep"), b"").unwrap();
    let err = run(
        &args(&[dir.path().to_str().unwrap(), "rows-1"]),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(&err, FixturesGenError::Io { path, .. } if *path == taken));
    assert_eq!(err.exit_status(), 1);
}

#[test]
fn run_returns_an_xlsx_error_when_the_workbook_cannot_be_written() {
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("rows-1.xlsx.part");
    std::fs::create_dir(&part).unwrap();
    let err = run(
        &args(&[dir.path().to_str().unwrap(), "rows-1"]),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(&err, FixturesGenError::Xlsx { path, .. } if *path == part));
    assert_eq!(err.exit_status(), 1);
    assert!(
        err.to_string()
            .starts_with(&format!("{}: ", part.display()))
    );
}

/// An output whose every write returns an error.
struct Refuses;

impl Write for Refuses {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("refused"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn run_returns_an_output_error_when_writing_the_path_to_the_output_fails() {
    let dir = tempfile::tempdir().unwrap();
    let err = run(
        &args(&[dir.path().to_str().unwrap(), "rows-1"]),
        &mut Refuses,
    )
    .unwrap_err();
    assert!(matches!(err, FixturesGenError::Output(_)));
    assert_eq!(err.exit_status(), 1);
    assert_eq!(err.to_string(), "writing to the output failed: refused");
    assert!(dir.path().join("rows-1.xlsx").exists());
}
