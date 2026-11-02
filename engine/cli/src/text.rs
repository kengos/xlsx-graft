//! What every command prints the same way in text, which is for a person
//! and not a contract: a string escaped to stay on its line and in its
//! column, and the problems found, a line each on stderr.

use crate::workbook::Refused;
use std::fmt;
use std::io::Write;
use xlsx_graft_diagnostics::Record;

/// Writes `text` to `line`, its backslash, tab, line feed and carriage
/// return written `\\`, `\t`, `\n` and `\r`.
pub(crate) fn escape(line: &mut String, text: &str) {
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

/// Writes each of `diagnostics`, then `error`, as a line to `err`. A
/// stderr that refuses them is passed over: the status still says the
/// run failed, and a closed stderr must not pass for a closed stdout.
pub(crate) fn problems(err: &mut dyn Write, diagnostics: &[Record], error: Option<&Refused>) {
    for record in diagnostics {
        let _ = writeln!(err, "xlsx-graft: {}", Problem(record));
    }
    if let Some(error) = error {
        let _ = writeln!(err, "xlsx-graft: {}", error.0);
    }
}

/// A diagnostic, on one line: `error xml_schema_invalid in /xl/...: ...`.
pub(crate) struct Problem<'r>(pub(crate) &'r Record);

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

#[cfg(test)]
mod tests;
