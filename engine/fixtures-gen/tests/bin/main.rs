//! End-to-end tests of the `xlsx-graft-fixtures-gen` binary: what it prints
//! and the exit status, for a command line it writes workbooks for and
//! ones it refuses.
//!
//! Spawning `CARGO_BIN_EXE_xlsx-graft-fixtures-gen` keeps `main.rs` inside
//! the coverage measurement (cargo-llvm-cov instruments child processes).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

use std::process::{Command, Output};

fn fixtures_gen(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xlsx-graft-fixtures-gen"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

#[test]
fn the_binary_prints_the_path_of_each_workbook_written_and_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated");
    let output = fixtures_gen(&[out_dir.to_str().unwrap(), "rows-3"]);
    let path = out_dir.join("rows-3.xlsx");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stdout), format!("{}\n", path.display()));
    assert_eq!(text(&output.stderr), "");
    assert!(path.is_file());
}

#[test]
fn the_binary_exits_2_with_the_usage_for_a_command_line_without_a_directory() {
    let output = fixtures_gen(&[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(text(&output.stdout), "");
    assert_eq!(
        text(&output.stderr),
        "xlsx-graft-fixtures-gen: usage: xlsx-graft-fixtures-gen <dir> [rows-<n>|shared-<n>...] | repack <fixtures-dir> <dir> | cells <dir> | blank <dir> | styles <fixtures-dir> <dir> | structure <fixtures-dir> <dir>\n"
    );
}

#[test]
fn the_binary_exits_1_when_the_directory_is_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file");
    std::fs::write(&file, b"").unwrap();
    let output = fixtures_gen(&[file.to_str().unwrap(), "rows-1"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(text(&output.stdout), "");
    assert!(
        text(&output.stderr).starts_with(&format!("xlsx-graft-fixtures-gen: {}: ", file.display())),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn the_binary_repacks_the_workbooks_of_a_directory_and_prints_each_path_written() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    std::fs::create_dir(&from).unwrap();
    let table = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/table.xlsx");
    std::fs::copy(table, from.join("table.xlsx")).unwrap();
    let output = fixtures_gen(&["repack", from.to_str().unwrap(), to.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        format!(
            "{}\n{}\n",
            to.join("table.xlsx").display(),
            to.join("table-app.xlsx").display()
        )
    );
    assert_eq!(text(&output.stderr), "");
    assert!(to.join("table.xlsx").is_file() && to.join("table-app.xlsx").is_file());
}

#[test]
fn the_binary_writes_the_cells_workbooks_and_prints_each_path_written() {
    let dir = tempfile::tempdir().unwrap();
    let output = fixtures_gen(&["cells", dir.path().to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    let expected: String = [
        "cells-1900.xlsx",
        "cells-1900-strict.xlsx",
        "cells-1904.xlsx",
    ]
    .iter()
    .map(|name| format!("{}\n", dir.path().join(name).display()))
    .collect();
    assert_eq!(text(&output.stdout), expected);
    assert_eq!(text(&output.stderr), "");
}

#[test]
fn the_binary_writes_the_blank_workbooks_and_prints_each_path_written() {
    let dir = tempfile::tempdir().unwrap();
    let output = fixtures_gen(&["blank", dir.path().to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    let expected: String = [
        "blank.xlsx",
        "blank-created.xlsx",
        "blank-font.xlsx",
        "blank-row-height.xlsx",
    ]
    .iter()
    .map(|name| format!("{}\n", dir.path().join(name).display()))
    .collect();
    assert_eq!(text(&output.stdout), expected);
    assert_eq!(text(&output.stderr), "");
}

#[test]
fn the_binary_gives_styles_to_the_workbooks_of_a_directory_and_prints_each_path_written() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    std::fs::create_dir(&from).unwrap();
    let table = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/table.xlsx");
    std::fs::copy(table, from.join("table.xlsx")).unwrap();
    let output = fixtures_gen(&["styles", from.to_str().unwrap(), to.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        format!("{}\n", to.join("table.xlsx").display())
    );
    assert_eq!(text(&output.stderr), "");
    assert!(to.join("table.xlsx").is_file());
}
