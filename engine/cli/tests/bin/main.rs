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

mod output;
mod usage;
mod version;

use std::process::{Command, Output};

fn xlsx_graft() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xlsx-graft"))
}

fn run(args: &[&str]) -> Output {
    xlsx_graft().args(args).output().unwrap()
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}
