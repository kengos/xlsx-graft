//! xlsx-graft CLI: the engine on the command line.
//!
//! A thin host over [`xlsx_graft_core`], like the C ABI and the MCP server.
//! `main.rs` parses [`Cli`] and hands it to [`run`]; everything testable
//! lives here. For now the one thing to ask is `--version`.
//!
//! Every command's output goes through [`emit`], which prints it as text or as
//! JSON by `--format`. The JSON is a contract: its keys and their types do not
//! change once released, so a script can read them without parsing the text.
//! The help is not a command's output: it is text whatever `--format` says,
//! as clap's own `-h` / `--help` are.

use clap::{CommandFactory, Parser};
use std::io::Write;

mod error;
mod format;
mod version;

pub use error::CliError;
pub use format::{Format, Report, emit};
pub use version::VersionReport;

/// The command line.
#[derive(Debug, Parser)]
// clap's own `--version` prints from inside the parser and never sees
// `--format`; the flag below goes through `emit` like every command's output.
// clap adds its own only when `version` is set on the command, so this
// disable is a guard against that being added later.
#[command(
    name = "xlsx-graft",
    about = "Read, verify, create and edit xlsx workbooks",
    disable_version_flag = true
)]
pub struct Cli {
    /// Print the engine version.
    #[arg(short = 'V', long)]
    version: bool,
    /// How to print the output. json keeps its keys and their types stable.
    // global: a subcommand added later takes it too, before or after its name.
    #[arg(long, value_enum, global = true, default_value = "text")]
    format: Format,
}

/// Runs the parsed command line, writing its output to `out`.
///
/// With nothing to do (no flag, no subcommand) it prints the help, as text
/// even under `--format json`.
///
/// # Errors
///
/// [`CliError`] when the output cannot be serialized or written.
pub fn run(cli: &Cli, out: &mut dyn Write) -> Result<(), CliError> {
    if cli.version {
        emit(cli.format, &VersionReport::current(), out)?;
    } else {
        write!(out, "{}", Cli::command().render_long_help()).map_err(CliError::Write)?;
    }
    out.flush().map_err(CliError::Write)
}

#[cfg(test)]
mod tests;
