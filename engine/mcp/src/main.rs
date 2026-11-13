//! `xlsx-graft-mcp` binary entry point. All logic lives in the library crate.

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use xlsx_graft_mcp::StartError;

/// Exit 2: the command line or the policy is wrong; exit 1: a file or the
/// stdio could not be read or written. The same split as the CLI's.
fn main() -> ExitCode {
    let context = match xlsx_graft_mcp::start(env::args_os().skip(1)) {
        Ok(context) => context,
        Err(error) => {
            report(&error);
            return match error {
                StartError::Usage(_) | StartError::PolicyInvalid { .. } => ExitCode::from(2),
                StartError::PolicyUnreadable { .. } => ExitCode::FAILURE,
            };
        }
    };
    match xlsx_graft_mcp::serve(&mut io::stdin().lock(), &mut io::stdout().lock(), &context) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report(&err);
            ExitCode::FAILURE
        }
    }
}

/// Writes `error` to stderr.
fn report(error: &dyn std::fmt::Display) {
    // Not eprintln!: it panics when stderr refuses the write too (a
    // closed pipe, a full disk), and the exit code already carries
    // the failure.
    let _ = writeln!(
        io::stderr(),
        "xlsx-graft-mcp: {}",
        error.to_string().trim_end()
    );
}
