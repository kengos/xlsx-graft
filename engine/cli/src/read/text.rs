//! `read --format text`: for a person at a terminal, and not a contract.
//!
//! Each row is a line on stdout: its number, then the value of each column
//! from A, separated by tabs. A string's or an error value's backslash,
//! tab, line feed and carriage return are written `\\`, `\t`, `\n` and
//! `\r`, so a value never splits its line or its column. A number is
//! written as Rust writes it, but in an exponent's form below 1e-5 and
//! from 1e16; a boolean `TRUE` or `FALSE`, as Excel shows one; a date or
//! time in ISO 8601 (`2024-01-31T08:30:00.000`), and elapsed time in hours
//! that do not wrap at a day (`26:03:00.000`). The problems found, and why
//! no sheet was read, go to stderr, a line each, after the rows.

use super::{End, Sink};
use crate::CliError;
use std::fmt::{self, Write as _};
use std::io::Write;
use xlsx_graft_diagnostics::Record;
use xlsx_graft_read::{Row, Value};

/// Prints the rows to `out` and the problems to `err`.
pub(super) struct Text<'a> {
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    /// The line being written, reused.
    line: String,
}

impl<'a> Text<'a> {
    pub(super) fn new(out: &'a mut dyn Write, err: &'a mut dyn Write) -> Self {
        Self {
            out,
            err,
            line: String::new(),
        }
    }
}

impl Sink for Text<'_> {
    fn row(&mut self, row: &Row) -> Result<(), CliError> {
        self.line.clear();
        // Writing to a String does not fail.
        let _ = write!(self.line, "{}", row.number());
        for value in row.values() {
            self.line.push('\t');
            let _ = write_value(&mut self.line, value);
        }
        self.line.push('\n');
        self.out
            .write_all(self.line.as_bytes())
            .map_err(CliError::Write)
    }

    /// Writes the problems to `err` once the rows before them are out. A
    /// stderr that refuses them is passed over: the status still says the
    /// run failed, and a closed stderr must not pass for a closed stdout.
    fn end(&mut self, end: &End) -> Result<(), CliError> {
        self.out.flush().map_err(CliError::Write)?;
        for record in &end.diagnostics {
            let _ = writeln!(self.err, "xlsx-graft: {}", Problem(record));
        }
        if let Some(error) = &end.error {
            let _ = writeln!(self.err, "xlsx-graft: {error}");
        }
        Ok(())
    }
}

/// Writes `value` to `line` as the module docs say.
pub(super) fn write_value(line: &mut String, value: Value<'_>) -> fmt::Result {
    match value {
        Value::Empty => Ok(()),
        Value::String(text) | Value::Error(text) => {
            escape(line, text);
            Ok(())
        }
        Value::Number(number) if number == 0.0 || (1e-5..1e16).contains(&number.abs()) => {
            write!(line, "{number}")
        }
        Value::Number(number) => write!(line, "{number:e}"),
        Value::Bool(bool) => line.write_str(if bool { "TRUE" } else { "FALSE" }),
        Value::Date(date) => write!(line, "{date}"),
        Value::Time(time) => write!(line, "{time}"),
        Value::DateTime(at) => write!(line, "{at}"),
        Value::Elapsed(elapsed) => {
            let ms = elapsed.milliseconds();
            let (hours, minutes) = (ms / 3_600_000, ms / 60_000 % 60);
            let (seconds, ms) = (ms / 1_000 % 60, ms % 1_000);
            write!(line, "{hours}:{minutes:02}:{seconds:02}.{ms:03}")
        }
    }
}

/// Writes `text` to `line`, its backslashes, tabs and line ends escaped.
fn escape(line: &mut String, text: &str) {
    for char in text.chars() {
        match char {
            '\\' => line.push_str("\\\\"),
            '\t' => line.push_str("\\t"),
            '\n' => line.push_str("\\n"),
            '\r' => line.push_str("\\r"),
            char => line.push(char),
        }
    }
}

/// A diagnostic, on one line: `error xml_schema_invalid in /xl/...: ...`.
pub(super) struct Problem<'r>(pub(super) &'r Record);

impl fmt::Display for Problem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let diagnostic = &self.0.diagnostic;
        let severity = diagnostic.severity().as_str();
        write!(f, "{severity} {}", diagnostic.code().as_str())?;
        let location = &self.0.location;
        let place = (location.part.as_ref())
            .or(location.entry.as_ref())
            .or(location.rels_part.as_ref());
        if let Some(place) = place {
            write!(f, " in {place}")?;
        }
        write!(f, ": {}", diagnostic.message())
    }
}
