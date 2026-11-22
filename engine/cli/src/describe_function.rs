//! `describe-function`: what the compatibility table says of one function,
//! through `xlsx_graft_host`, the same JSON the MCP server gives. The long
//! help in `help.rs` says what each format prints.
//!
//! The table answers every name, so `describe-function` prints its
//! description and exits 0 for a function with a row of versions, a baseline
//! function and a name the table does not list. Only a command line clap
//! refuses, and an output that cannot be written, print no description.

mod help;

use crate::{CliError, Format, Outcome, Report, emit};
use clap::Args;
pub(crate) use help::{AFTER_LONG_HELP, LONG_ABOUT};
use std::io::{self, Write};
use xlsx_graft_host::{FunctionDescription, FunctionText, describe_function};

/// The arguments of `describe-function`.
#[derive(Debug, Args)]
pub struct DescribeFunctionArgs {
    /// The function's name as a formula writes it, such as SUM or
    /// _xlfn.SEQUENCE. A leading _xlfn. and _xlws. and the letter case are
    /// ignored.
    name: String,
}

impl Report for FunctionDescription {
    fn write_text(&self, out: &mut dyn Write) -> io::Result<()> {
        write!(out, "{}", FunctionText(self))
    }
}

/// Looks up the function `args` names, printing the description in `format`
/// to `out`.
pub(crate) fn run(
    args: &DescribeFunctionArgs,
    format: Format,
    out: &mut dyn Write,
) -> Result<Outcome, CliError> {
    emit(format, &describe_function(&args.name), out)?;
    Ok(Outcome::Success)
}
