//! Every workbook in the repository's `fixtures/` folder, made in Excel,
//! read by `xlsx-graft-read` and verified by `verify` with the engine's
//! default rules, function table and targets: what each gives is written in
//! `expected.rs`, one entry per workbook, and `render.rs` turns the
//! results into the lines compared.
//!
//! `fixtures/` is outside the crate: the tests read the workbooks from
//! disk when they run, and do not embed them in the test binary, so the
//! crate's library builds without the repository's `fixtures/` folder.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

mod expected;
mod render;

use std::path::{Path, PathBuf};

/// The repository's `fixtures/` folder.
fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The file names of the workbooks in `fixtures/` (`.xlsx` and `.xlsm`),
/// sorted.
fn workbooks() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".xlsx") || name.ends_with(".xlsm"))
        .collect();
    names.sort();
    names
}

/// The bytes of the workbook `name` in `fixtures/`.
fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(folder().join(name)).unwrap()
}

/// Compares the file names in `EXPECTED`, in the order written there,
/// with the workbooks' file names sorted, so an entry out of file-name
/// order fails as well.
#[test]
fn every_workbook_in_fixtures_has_an_expected_entry_and_every_entry_a_workbook() {
    let listed: Vec<String> = expected::EXPECTED
        .iter()
        .map(|e| e.file.to_owned())
        .collect();
    assert_eq!(workbooks(), listed);
}

/// Panics when `actual(file)` differs from `expected(entry)` for any
/// entry, listing every workbook that differs with its lines as they now
/// are, written as `expected.rs` writes them.
fn compare(
    what: &str,
    expected: impl Fn(&expected::Expected) -> &'static [&'static str],
    actual: impl Fn(&str) -> Vec<String>,
) {
    let mut differ = String::new();
    for entry in expected::EXPECTED {
        let lines = actual(entry.file);
        if lines != expected(entry) {
            differ += &format!("fixtures/{} ({what}) now gives:\n", entry.file);
            for line in lines {
                differ += &format!("    {line:?},\n");
            }
        }
    }
    assert!(differ.is_empty(), "{differ}");
}

#[test]
fn verify_gives_each_workbook_the_diagnostics_expected_rs_lists() {
    compare("verify", |e| e.verify, |file| render::verify(&bytes(file)));
}

#[test]
fn reading_each_workbook_gives_the_sheets_rows_and_diagnostics_expected_rs_lists() {
    compare("read", |e| e.read, |file| render::read(&bytes(file)));
}
