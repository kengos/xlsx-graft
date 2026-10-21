//! `xlsx-graft` binary entry point. All logic lives in the library crate.

use clap::Parser;
use std::io::{self, Write};
use std::process::ExitCode;
use xlsx_graft_cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match xlsx_graft_cli::run(&cli, &mut io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Not eprintln!: it panics when stderr refuses the write too (a
            // closed pipe, a full disk), and the exit code already carries
            // the failure.
            let _ = writeln!(io::stderr(), "xlsx-graft: {err}");
            ExitCode::FAILURE
        }
    }
}
