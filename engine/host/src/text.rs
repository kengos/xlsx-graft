//! The result as text, for a person: what the CLI prints for `verify`
//! without `--format json`, and what the MCP server puts beside the JSON.
//! Not a contract: it may change.
//!
//! [`DiagnosticsText`] is the diagnostics alone, so a command that returns
//! something else beside them prints them the same way.

use crate::report::{FailOn, Report};
use std::fmt;
use xlsx_graft_diagnostics::{Fix, Location, Record, Severity};

/// `diagnostics` as text: under a heading per severity (`error`, `warn`,
/// `notice`, in that order, a heading only when it has one), a line each,
/// and the fix, when there is one, on the line under it.
///
/// ```text
/// error
///   part_denied /xl/vbaProject.bin — The package holds a macro ...
///     fix (rules): add macro to parts.allow
/// ```
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticsText<'a>(pub &'a [Record]);

impl fmt::Display for DiagnosticsText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for severity in [Severity::Error, Severity::Warn, Severity::Notice] {
            let mut listed = (self.0.iter())
                .filter(|record| record.diagnostic.severity() == severity)
                .peekable();
            if listed.peek().is_none() {
                continue;
            }
            writeln!(f, "{}", severity.as_str())?;
            for record in listed {
                line(f, record)?;
            }
        }
        Ok(())
    }
}

/// One diagnostic's line, then its fix's line when it has a fix.
fn line(f: &mut fmt::Formatter<'_>, record: &Record) -> fmt::Result {
    let diagnostic = &record.diagnostic;
    write!(f, "  {}", diagnostic.code().as_str())?;
    place(f, &record.location)?;
    writeln!(f, " — {}", diagnostic.message())?;
    match diagnostic.fix() {
        Some(Fix::Workbook { text }) => writeln!(f, "    fix: {text}"),
        Some(Fix::Policy { line }) => writeln!(f, "    fix (policy): {line}"),
        Some(Fix::Template { list, add }) => {
            writeln!(f, "    fix (rules): add {add} to {list}")
        }
        None => Ok(()),
    }
}

/// Where in the package `location` points, each part after a space: the
/// part (or else the zip entry, or else the relationships part), the place
/// in it, and the defined name the problem was reached through.
fn place(f: &mut fmt::Formatter<'_>, location: &Location) -> fmt::Result {
    let package = (location.part.as_ref())
        .or(location.entry.as_ref())
        .or(location.rels_part.as_ref());
    if let Some(package) = package {
        write!(f, " {package}")?;
    }
    if let Some(place) = &location.place {
        write!(f, " {place}")?;
    }
    if let Some(name) = &location.defined_name {
        write!(f, " via {name}")?;
    }
    Ok(())
}

/// The whole result as text: its diagnostics as [`DiagnosticsText`] writes
/// them, why the engine could not look at the workbook when it could not,
/// and a last line of the counts and whether the result is `ok`.
///
/// ```text
/// could not verify: book.xlsx could not be opened: No such file or directory (os error 2)
/// 0 errors, 0 warnings, 0 notices — not ok
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ReportText<'a>(pub &'a Report);

impl fmt::Display for ReportText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let report = self.0;
        write!(f, "{}", DiagnosticsText(&report.diagnostics))?;
        if let Some(error) = &report.error {
            writeln!(f, "could not verify: {}", error.message)?;
        }
        let verdict = if report.ok { "ok" } else { "not ok" };
        write!(f, "{} — {verdict}", report.counts)?;
        if report.fail_on == FailOn::Warn {
            write!(f, " (failOn: warn)")?;
        }
        writeln!(f)
    }
}

#[cfg(test)]
mod tests;
