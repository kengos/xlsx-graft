//! `xlsx-graft-fixtures-gen layout` over every workbook of `fixtures/`, read
//! back: each result is checked by `xlsx-graft-verify` (the error
//! diagnostics it reports are the ones it reports for the workbook before the
//! edit), the first worksheet holds what `layout` sets, and the sheets are
//! visible, hidden and very hidden as it says.
//!
//! The test is here and not in the crate's unit tests because it reads the
//! results with the verifier, a dev-dependency that the crate's own tests
//! reach too, but the checks are of the written workbooks, which this folder
//! holds.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

use std::ffi::OsString;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::{Diagnostics, Severity};
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_read::{ReadLimits, SheetKind, Workbook, WorkbookPart};
use xlsx_graft_verify::{DefaultFunctions, Rules, Targets};
use xlsx_graft_write::WriteLimits;
use xlsx_graft_write::structure::model;

/// The repository's `fixtures/` folder.
fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The paths of the workbooks in `fixtures/` (`.xlsx` and `.xlsm`), sorted.
fn workbooks() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("xlsx" | "xlsm")
            )
        })
        .collect();
    paths.sort();
    paths
}

/// What `layout` printed and where it wrote.
fn layout() -> (String, tempfile::TempDir) {
    let to = tempfile::tempdir().unwrap();
    let args: Vec<OsString> = vec!["layout".into(), folder().into(), to.path().into()];
    let mut out = Vec::new();
    xlsx_graft_fixtures_gen::run(&args, &mut out).unwrap();
    (String::from_utf8(out).unwrap(), to)
}

fn open(bytes: &[u8]) -> Package<Cursor<Vec<u8>>> {
    Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap()
}

/// The sheets' names and states, as the reader reads them, and the text of
/// the first sheet's part when that is a worksheet.
fn read(bytes: &[u8]) -> (Vec<(String, String)>, Option<String>) {
    let mut package = open(bytes);
    let mut diagnostics = Diagnostics::new();
    let found = Workbook::find(&package, &mut diagnostics).unwrap();
    let limits = ReadLimits::default();
    let read = WorkbookPart::read(&mut package, &found, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    assert!(
        diagnostics.records().is_empty(),
        "{:?}",
        diagnostics.records()
    );
    let sheets = (read.sheets().iter())
        .map(|sheet| (sheet.name().to_owned(), format!("{:?}", sheet.state())))
        .collect();
    let first = read.sheets().first().and_then(|sheet| sheet.part());
    let first = first.filter(|part| part.kind() == SheetKind::Worksheet);
    let text = first.map(|part| {
        let limit = xlsx_graft_diagnostics::Limit::default_of(1 << 28);
        let text = package
            .read_xml_part(part.index(), limit, &mut diagnostics)
            .unwrap()
            .unwrap();
        String::from_utf8(text).unwrap()
    });
    (sheets, text)
}

/// The sheet the first workbook view makes active.
fn active(bytes: &[u8]) -> u32 {
    let mut package = open(bytes);
    let book = model::Workbook::read(
        &mut package,
        &WriteLimits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    book.views()
        .first()
        .and_then(|view| view.active_tab())
        .unwrap_or(0)
}

/// The codes of the diagnostics of error severity `verify` pushes for
/// `bytes`, sorted.
fn errors(bytes: &[u8]) -> Vec<String> {
    let mut diagnostics = Diagnostics::new();
    xlsx_graft_verify::verify(
        Cursor::new(bytes),
        &xlsx_graft_verify::Limits::default(),
        &Rules::default(),
        &DefaultFunctions,
        &Targets::default(),
        &mut diagnostics,
    )
    .unwrap();
    let mut codes: Vec<String> = (diagnostics.records().iter())
        .filter(|record| record.diagnostic.severity() == Severity::Error)
        .map(|record| record.diagnostic.code().as_str().to_owned())
        .collect();
    codes.sort();
    codes
}

/// The state each sheet has after `layout` when the workbook has `count`
/// sheets that are all visible.
fn states(count: usize) -> Vec<&'static str> {
    (0..count)
        .map(|at| match at {
            1 if count >= 3 => "Hidden",
            2 if count >= 4 => "VeryHidden",
            _ => "Visible",
        })
        .collect()
}

#[test]
fn each_workbook_of_fixtures_is_written_with_its_first_sheet_laid_out_and_verifies() {
    let (out, to) = layout();
    assert!(!out.contains("skipped"), "{out}");
    let (mut laid, mut hidden, mut very_hidden, mut moved) = (0, 0, 0, 0);
    for path in workbooks() {
        let name = path.file_name().unwrap();
        let result = std::fs::read(to.path().join(name)).unwrap();
        let original = std::fs::read(&path).unwrap();
        let label = name.to_string_lossy();
        assert!(out.contains(&format!("{}\n", to.path().join(name).display())));
        assert_eq!(errors(&result), errors(&original), "{label}");
        let (before, _) = read(&original);
        let (sheets, first) = read(&result);
        let names = |sheets: &[(String, String)]| -> Vec<String> {
            sheets.iter().map(|(name, _)| name.clone()).collect()
        };
        assert_eq!(names(&sheets), names(&before), "{label}");
        // The fixtures' sheets are all visible.
        let expected = states(before.len());
        let got: Vec<&str> = sheets.iter().map(|(_, state)| state.as_str()).collect();
        assert_eq!(got, expected, "{label}");
        let last = u32::try_from(before.len() - 1).unwrap();
        if before.len() >= 2 {
            assert_eq!(active(&result), last, "{label}");
            moved += 1;
        }
        hidden += usize::from(before.len() >= 3);
        very_hidden += usize::from(before.len() >= 4);
        let Some(first) = first else { continue };
        laid += 1;
        let merged =
            first.contains(r#"ref="AA1000:AB1001""#) || first.contains(r#"ref="AD2000:AE2001""#);
        assert!(merged, "{label}: {first}");
        assert!(first.contains("<pane "), "{label}");
        assert!(first.contains(r#"state="frozen""#), "{label}");
        assert!(first.contains("<sheetProtection "), "{label}");
        assert!(first.contains(r#"algorithmName="SHA-512""#), "{label}");
        assert!(first.contains(r#"spinCount="100000""#), "{label}");
        let row = first
            .find(r#"<row r="500""#)
            .unwrap_or_else(|| panic!("{label}"));
        let row = &first[row..first[row..].find('>').unwrap() + row];
        assert!(
            row.contains(r#"ht="24""#) && row.contains(r#"customHeight="1""#),
            "{label}"
        );
        assert!(first.contains(r#"width="20""#), "{label}");
        assert!(first.contains(r#"width="30""#), "{label}");
    }
    assert!(laid >= 10, "{laid}");
    assert!(
        moved > 0 && hidden > 0 && very_hidden > 0,
        "{moved} {hidden} {very_hidden}"
    );
}

#[test]
fn the_merged_and_table_workbooks_and_a_workbook_of_several_sheets_verify_without_an_error() {
    let (_, to) = layout();
    let mut several = 0;
    for name in ["merged.xlsx", "table.xlsx"] {
        let result = std::fs::read(to.path().join(name)).unwrap();
        assert_eq!(errors(&result), Vec::<String>::new(), "{name}");
    }
    for path in workbooks() {
        let result = std::fs::read(to.path().join(path.file_name().unwrap())).unwrap();
        if read(&result).0.len() >= 2 && errors(&result).is_empty() {
            several += 1;
        }
    }
    assert!(several > 0);
}
