//! `xlsx-graft-fixtures-gen` binary entry point. All logic lives in the
//! library crate.

use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match xlsx_graft_fixtures_gen::run(&args, &mut io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Not eprintln!: it panics when the write to stderr fails too
            // (a closed pipe, a full disk), and the exit code already
            // carries the failure.
            let _ = writeln!(io::stderr(), "xlsx-graft-fixtures-gen: {err}");
            ExitCode::from(err.exit_status())
        }
    }
}
