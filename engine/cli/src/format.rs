//! `--format` and the one place every output is printed from.
//!
//! A command returns a value implementing [`Report`] instead of printing, and
//! [`emit`] prints it in the chosen format. `Report` needs both `Serialize`
//! and a text form, so a command cannot be added with only one of them.

use crate::CliError;
use clap::ValueEnum;
use serde::Serialize;
use std::io::{self, Write};

/// The output format, chosen by `--format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// For a person reading a terminal. Not a contract: it may change.
    Text,
    /// One JSON value per command run, or one a line as `read` reads
    /// (JSON Lines). Its keys and their types are a contract. The help
    /// stays text.
    Json,
}

/// What a command returns: its output, printable in every [`Format`].
///
/// The JSON form is the `Serialize` impl.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use std::io::{self, Write};
/// use xlsx_graft_cli::{Format, Report, emit};
///
/// #[derive(Serialize)]
/// struct Count {
///     count: u32,
/// }
///
/// impl Report for Count {
///     fn write_text(&self, out: &mut dyn Write) -> io::Result<()> {
///         writeln!(out, "{} sheets", self.count)
///     }
/// }
///
/// let mut out = Vec::new();
/// emit(Format::Json, &Count { count: 3 }, &mut out)?;
/// assert_eq!(out, b"{\"count\":3}\n");
/// # Ok::<(), xlsx_graft_cli::CliError>(())
/// ```
pub trait Report: Serialize {
    /// Writes the text form, ending with a newline.
    ///
    /// # Errors
    ///
    /// The writer's error.
    fn write_text(&self, out: &mut dyn Write) -> io::Result<()>;
}

/// Prints `value` to `out` in `format`. JSON is one line.
///
/// # Errors
///
/// [`CliError::Serialize`] when `value` will not serialize,
/// [`CliError::Write`] when `out` refuses the bytes.
pub fn emit<T: Report>(format: Format, value: &T, out: &mut dyn Write) -> Result<(), CliError> {
    match format {
        Format::Text => value.write_text(out).map_err(CliError::Write),
        Format::Json => {
            let json = serde_json::to_string(value)?;
            writeln!(out, "{json}").map_err(CliError::Write)
        }
    }
}

#[cfg(test)]
mod tests;
