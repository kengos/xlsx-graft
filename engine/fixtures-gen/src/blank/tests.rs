//! The workbooks `blank` writes, read back with `xlsx-graft-read`, and the
//! command lines and failures `run` returns an error for: the sheets and
//! values of each workbook, the `sheetFormatPr` of each, and the order of
//! the zip's entries.

use super::create;
use crate::tests::Refuses;
use crate::{FixturesGenError, run};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::Path;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits as PackageLimits, ZipArchive, ZipLimits};
use xlsx_graft_read::{Book, ReadLimits, Row as ReadRow, Value};
use xlsx_graft_spool::{MemorySpools, TempFiles};
use xlsx_graft_write::blank::{self, Defaults};
use xlsx_graft_write::structure::StructureEdit;
use xlsx_graft_write::{PackageEdit, WriteLimits};

/// The workbooks `blank` writes, and the `sheetFormatPr` of each first
/// worksheet.
const BOOKS: [(&str, &str); 4] = [
    ("blank.xlsx", r#"<sheetFormatPr defaultRowHeight="15"/>"#),
    (
        "blank-created.xlsx",
        r#"<sheetFormatPr defaultRowHeight="15"/>"#,
    ),
    (
        "blank-font.xlsx",
        r#"<sheetFormatPr defaultRowHeight="15" customHeight="1"/>"#,
    ),
    (
        "blank-row-height.xlsx",
        r#"<sheetFormatPr defaultRowHeight="20" customHeight="1"/>"#,
    ),
];

fn args(words: &[&Path]) -> Vec<OsString> {
    words
        .iter()
        .map(|word| word.as_os_str().to_owned())
        .collect()
}

/// Runs `blank` into `dir`, returning what it printed.
fn write_books(dir: &Path) -> String {
    let mut out = Vec::new();
    run(&args(&[Path::new("blank"), dir]), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// The sheets' names and, of the first sheet, each row's columns A and B as
/// `Debug` prints them, with the codes of the diagnostics.
fn read_back(path: &Path) -> (Vec<String>, Vec<(String, String)>, Vec<String>) {
    let bytes = fs::read(path).unwrap();
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
    let mut read_rows = book.rows(0).unwrap();
    let mut row = ReadRow::new();
    let mut rows = Vec::new();
    while read_rows.next_row(&mut row, &mut diagnostics).unwrap() {
        let mut values = row.values();
        let a = format!("{:?}", values.next().unwrap());
        let b = format!("{:?}", values.next().unwrap_or(Value::Empty));
        rows.push((a, b));
    }
    let sheets = book.sheets().iter().map(|s| s.name().to_owned()).collect();
    let codes = (diagnostics.records().iter())
        .map(|r| r.diagnostic.code().as_str().to_owned())
        .collect();
    (sheets, rows, codes)
}

/// The names of the entries of the workbook at `path`, and the text of each
/// entry whose name starts with `xl/worksheets/`.
fn entries(path: &Path) -> (Vec<String>, Vec<String>) {
    let bytes = fs::read(path).unwrap();
    let mut diagnostics = Diagnostics::new();
    let mut archive = ZipArchive::open(Cursor::new(bytes), &ZipLimits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let names: Vec<String> = (archive.entries().iter())
        .map(|entry| String::from_utf8(entry.name().to_vec()).unwrap())
        .collect();
    let mut sheets = Vec::new();
    for (index, name) in names.iter().enumerate() {
        if name.starts_with("xl/worksheets/") && name.ends_with(".xml") {
            let mut entry = archive
                .open_entry(index, &mut diagnostics)
                .unwrap()
                .unwrap();
            let mut text = String::new();
            entry.read_to_string(&mut text).unwrap();
            sheets.push(text);
        }
    }
    assert!(diagnostics.records().is_empty());
    (names, sheets)
}

#[test]
fn run_prints_the_path_of_each_workbook_written_in_order_into_a_directory_it_creates() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested").join("blank");
    let printed = write_books(&nested);
    let expected: String = BOOKS
        .iter()
        .map(|(name, _)| format!("{}\n", nested.join(name).display()))
        .collect();
    assert_eq!(printed, expected);
}

#[test]
fn the_built_in_workbook_is_written_as_it_is_with_one_empty_sheet_named_sheet1() {
    let dir = tempfile::tempdir().unwrap();
    write_books(dir.path());
    let path = dir.path().join("blank.xlsx");
    let (sheets, rows, codes) = read_back(&path);
    assert_eq!(sheets, ["Sheet1"]);
    assert!(rows.is_empty());
    assert!(codes.is_empty(), "{codes:?}");
    let (names, _) = entries(&path);
    assert_eq!(names[0], "[Content_Types].xml");
}

#[test]
fn each_created_workbook_has_the_sheets_sheet1_and_data_and_the_rows_of_the_first() {
    let dir = tempfile::tempdir().unwrap();
    write_books(dir.path());
    for (name, _) in &BOOKS[1..] {
        let path = dir.path().join(name);
        let (sheets, rows, codes) = read_back(&path);
        assert_eq!(sheets, ["Sheet1", "Data"], "{name}");
        assert!(codes.is_empty(), "{name}: {codes:?}");
        let text = |a: &str, b: Value| (format!("{:?}", Value::String(a)), format!("{b:?}"));
        assert_eq!(
            rows,
            [
                text("name", Value::String("value")),
                text("a string", Value::String("text")),
                text("a number", Value::Number(12.5)),
                text("a boolean", Value::Bool(true)),
            ],
            "{name}"
        );
        let (names, _) = entries(&path);
        assert_eq!(names[0], "[Content_Types].xml", "{name}");
    }
}

#[test]
fn each_workbook_has_the_sheet_format_pr_of_its_defaults_on_its_first_worksheet_alone() {
    let dir = tempfile::tempdir().unwrap();
    write_books(dir.path());
    for (name, element) in BOOKS {
        let (_, sheets) = entries(&dir.path().join(name));
        let with: Vec<&String> = sheets.iter().filter(|s| s.contains(element)).collect();
        assert_eq!(with.len(), 1, "{name}: {sheets:?}");
        let any = sheets
            .iter()
            .filter(|s| s.contains("sheetFormatPr"))
            .count();
        assert_eq!(any, 1, "{name}");
    }
}

#[test]
fn run_without_exactly_one_directory_returns_the_usage_error() {
    let mut out = Vec::new();
    for words in [
        &[Path::new("blank")][..],
        &[Path::new("blank"), Path::new("a"), Path::new("b")],
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
    let error = run(&args(&[Path::new("blank"), &file]), &mut Vec::new()).unwrap_err();
    assert!(matches!(&error, FixturesGenError::Io { path, .. } if *path == file));
    assert_eq!(error.exit_status(), 1);
}

#[test]
fn run_returns_a_blank_error_for_a_workbook_that_cannot_be_written_to_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let taken = dir.path().join("blank.xlsx");
    fs::create_dir(&taken).unwrap();
    fs::write(taken.join("file"), b"x").unwrap();
    let error = run(&args(&[Path::new("blank"), dir.path()]), &mut Vec::new()).unwrap_err();
    assert!(matches!(&error, FixturesGenError::Blank { path, .. } if *path == taken));
    assert_eq!(error.exit_status(), 1);
    assert!(
        error
            .to_string()
            .starts_with(&format!("{}: ", taken.display()))
    );
}

#[test]
fn run_returns_an_output_error_when_writing_a_path_to_the_output_fails() {
    let dir = tempfile::tempdir().unwrap();
    let error = run(&args(&[Path::new("blank"), dir.path()]), &mut Refuses).unwrap_err();
    assert!(matches!(error, FixturesGenError::Output(_)));
    assert_eq!(error.exit_status(), 1);
}

#[test]
fn the_create_procedure_gives_the_first_name_and_data_whether_or_not_it_collides() {
    for first in ["Sheet1", "sheet1", "Other"] {
        let defaults = Defaults::default();
        let limits = WriteLimits::default();
        let mut diagnostics = Diagnostics::new();
        let mut package = blank::package(&defaults).unwrap();
        let mut structure = StructureEdit::open(&mut package, &limits, &mut diagnostics)
            .unwrap()
            .unwrap();
        create(&mut structure, &defaults, first).unwrap();
        let mut edit = PackageEdit::new(package);
        structure.apply(&mut edit).unwrap();
        let finished = edit.finish(Vec::new(), &mut MemorySpools, &limits, &mut diagnostics);
        let bytes = finished.unwrap().unwrap();
        let book = Book::open(
            Cursor::new(bytes),
            &PackageLimits::default(),
            &ReadLimits::default(),
            &mut TempFiles::new(),
            &mut diagnostics,
        )
        .unwrap()
        .unwrap();
        let names: Vec<&str> = book.sheets().iter().map(|s| s.name()).collect();
        assert_eq!(names, [first, "Data"]);
    }
}

#[test]
fn each_workbook_has_the_font_of_its_defaults_in_its_styles_part() {
    let dir = tempfile::tempdir().unwrap();
    write_books(dir.path());
    let fonts = [
        ("blank.xlsx", "Calibri"),
        ("blank-created.xlsx", "Calibri"),
        ("blank-font.xlsx", "Yu Gothic"),
        ("blank-row-height.xlsx", "Yu Gothic"),
    ];
    for (name, font) in fonts {
        let bytes = fs::read(dir.path().join(name)).unwrap();
        let mut diagnostics = Diagnostics::new();
        let mut archive =
            ZipArchive::open(Cursor::new(bytes), &ZipLimits::default(), &mut diagnostics)
                .unwrap()
                .unwrap();
        let index = archive.position(b"xl/styles.xml").unwrap();
        let mut entry = archive
            .open_entry(index, &mut diagnostics)
            .unwrap()
            .unwrap();
        let mut text = String::new();
        entry.read_to_string(&mut text).unwrap();
        let element = format!(r#"<font><sz val="11"/><name val="{font}"/></font>"#);
        assert!(text.contains(&element), "{name}: {text}");
    }
}
