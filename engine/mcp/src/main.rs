//! `xlsx-graft-mcp` binary entry point. All logic lives in the library crate.

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match xlsx_graft_mcp::serve(&mut io::stdin().lock(), &mut io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Not eprintln!: it panics when stderr refuses the write too (a
            // closed pipe, a full disk), and the exit code already carries
            // the failure.
            let _ = writeln!(io::stderr(), "xlsx-graft-mcp: {err}");
            ExitCode::FAILURE
        }
    }
}
