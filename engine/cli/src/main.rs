//! `xlsx-graft` binary entry point. All logic lives in the library crate.

use std::io::{self, BufWriter};
use std::process::ExitCode;
use xlsx_graft_cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::try_parse_args(std::env::args_os()).unwrap_or_else(|err| err.exit());
    // `read` writes a line per row; stdout alone would write each one out.
    let mut out = BufWriter::new(io::stdout().lock());
    let result = xlsx_graft_cli::run(&cli, &mut out, &mut io::stderr());
    ExitCode::from(xlsx_graft_cli::exit_status(result, &mut io::stderr()))
}
