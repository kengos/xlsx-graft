//! End-to-end tests of the `xlsx-graft-mcp` binary over real stdio: what a
//! client reads back, and the exit code. The shared runner is here, the cases
//! in the sibling modules.
//!
//! Spawning `CARGO_BIN_EXE_xlsx-graft-mcp` keeps `main.rs` inside the
//! coverage measurement (cargo-llvm-cov instruments child processes).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

mod output;
mod session;

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn xlsx_graft_mcp() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xlsx-graft-mcp"));
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Writes `lines` to the server's stdin, one per line, closes it (which ends
/// the session) and waits for the exit.
fn talk_to(mut command: Command, lines: &[&str]) -> Output {
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for line in lines {
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin);
    child.wait_with_output().unwrap()
}

/// The response lines on stdout, each parsed as JSON.
fn responses(output: &Output) -> Vec<Value> {
    std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}
