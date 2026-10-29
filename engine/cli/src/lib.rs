//! xlsx-graft CLI: the engine on the command line.
//!
//! A thin host over [`xlsx_graft_core`] and [`xlsx_graft_read`], like the C
//! ABI and the MCP server. `main.rs` parses [`Cli`]
//! ([`Cli::try_parse_args`]), hands it to [`run`] and exits with the
//! [`exit_status`] of what it returns; everything testable lives here.
//! There are `--version` and the `read` command.
//!
//! Every command's output is printed as text or as JSON by `--format`:
//! `--version` through [`emit`], one value; `read` a line at a time as it
//! reads ([`ReadArgs`]). The JSON is a contract: its keys and their types
//! do not change once released, so a script can read them without parsing
//! the text. The help is not a command's output: it is text whatever
//! `--format` says, as clap's own `-h` / `--help` are.

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand};
use std::ffi::OsString;
use std::io::{self, Write};

mod error;
mod format;
mod read;
mod version;

pub use error::CliError;
pub use format::{Format, Report, emit};
pub use read::ReadArgs;
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
    // global: a subcommand takes it too, before or after its name.
    #[arg(long, value_enum, global = true, default_value = "text")]
    format: Format,
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    /// Parses `args`, the program's name first, as clap does, and refuses
    /// `--version` beside a command, which it would not print.
    /// `args_conflicts_with_subcommands` would refuse `--format` before the
    /// command's name too.
    ///
    /// # Errors
    ///
    /// The usage error, which exits 2.
    pub fn try_parse_args<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let cli = Self::try_parse_from(args)?;
        if cli.version && cli.command.is_some() {
            let message = "the argument '--version' cannot be used with a command";
            return Err(Self::command().error(ErrorKind::ArgumentConflict, message));
        }
        Ok(cli)
    }
}

/// A command.
#[derive(Debug, Subcommand)]
enum Command {
    /// Print a sheet's rows, one line each, as they are read.
    #[command(long_about = read::LONG_ABOUT, after_long_help = read::AFTER_LONG_HELP)]
    Read(ReadArgs),
}

/// How a run that printed what it had to ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It did what was asked: exit 0.
    Success,
    /// It printed why it did not: exit 1.
    Failure,
}

/// Runs the parsed command line, writing its output to `out` and, in
/// text, what went wrong to `err`.
///
/// With nothing to do (no flag, no subcommand) it prints the help, as text
/// even under `--format json`.
///
/// # Errors
///
/// [`CliError`] when the workbook cannot be opened or read on the host, or
/// the output cannot be serialized or written.
pub fn run(cli: &Cli, out: &mut dyn Write, err: &mut dyn Write) -> Result<Outcome, CliError> {
    let outcome = match &cli.command {
        Some(Command::Read(args)) => read::run(args, cli.format, out, err)?,
        None if cli.version => {
            emit(cli.format, &VersionReport::current(), out)?;
            Outcome::Success
        }
        None => {
            write!(out, "{}", Cli::command().render_long_help()).map_err(CliError::Write)?;
            Outcome::Success
        }
    };
    out.flush().map_err(CliError::Write)?;
    Ok(outcome)
}

/// The exit status of a run that ended in `result`, writing an error's
/// reason to `err`: 0 for [`Outcome::Success`], 1 for a failure.
///
/// A closed stdout (a pipe whose reader has gone, as `| head` leaves it)
/// is 0, with nothing written: whoever closed it has what they wanted, and
/// the run stopped at the write that found it gone.
pub fn exit_status(result: Result<Outcome, CliError>, err: &mut dyn Write) -> u8 {
    match result {
        Ok(Outcome::Success) => 0,
        Ok(Outcome::Failure) => 1,
        Err(CliError::Write(e)) if e.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(e) => {
            // Not eprintln!: it panics when stderr refuses the write too (a
            // closed pipe, a full disk), and the status already carries
            // the failure.
            let _ = writeln!(err, "xlsx-graft: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests;
