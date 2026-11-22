//! End-to-end tests of the `xlsx-graft` binary: what a script sees on stdout,
//! stderr and the exit code. The shared runner is here, the cases in the
//! sibling modules.
//!
//! Spawning `CARGO_BIN_EXE_xlsx-graft` keeps `main.rs` inside the coverage
//! measurement (cargo-llvm-cov instruments child processes).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

mod describe_function;
mod describe_sheet;
mod describe_sheet_failures;
mod output;
mod read;
mod read_failures;
mod sheets;
mod tree;
mod usage;
mod verify;
mod version;
mod workbook;

use std::process::{Command, Output};

fn xlsx_graft() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xlsx-graft"))
}

fn run(args: &[&str]) -> Output {
    xlsx_graft().args(args).output().unwrap()
}

/// The path of the fixture `name`.
fn fixture(name: &str) -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    dir.join(name).to_str().unwrap().to_owned()
}

/// The view `output` printed, which is one line of JSON.
fn printed(output: &Output) -> serde_json::Value {
    assert_eq!(stdout(output).lines().count(), 1);
    serde_json::from_str(stdout(output)).unwrap()
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}
