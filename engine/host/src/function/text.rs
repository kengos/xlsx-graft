//! A [`FunctionDescription`] as the text a person reads: what the CLI prints
//! for `describe-function --format text`. Not a contract: it may change.

use super::FunctionDescription;
use super::json::{AddedJson, EntryJson, Requirement, SafetyJson, SupportJson, file_name};
use std::fmt;
use xlsx_graft_compat::{Known, Lookup, Prefix, Safety};

/// `description` as text: the name and what the table knows of it, then its
/// file name, safety, the support of each application and the sources.
///
/// ```text
/// SEQUENCE — versioned
///   file name: _xlfn.SEQUENCE
///   safety: reviewed_safe
///   excel          2021    fallback name_error
///   numbers        not known
///   requires dynamic_array
///     excel          2021    fallback legacy_array
///   sources
///     https://...
/// ```
///
/// A version of `no` means no version has it, and `not known` that no source
/// says. A name the table does not list is one line.
#[derive(Debug, Clone, Copy)]
pub struct FunctionText<'a>(pub &'a FunctionDescription);

impl fmt::Display for FunctionText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lookup = &self.0.0;
        let (state, header) = match lookup.known {
            Known::Versioned { .. } => ("versioned", ""),
            Known::Baseline { .. } => ("baseline", ""),
            _ => ("unknown", ": the table does not list it; verify denies it"),
        };
        writeln!(f, "{} — {state}{header}", lookup.name)?;
        match lookup.known {
            Known::Versioned { row, safety } => {
                head(f, lookup, row.prefix, safety)?;
                write_support(f, "  ", &SupportJson::of(&row.support))?;
                for feature in row.requires {
                    let requirement = Requirement::of(*feature);
                    writeln!(f, "  requires {}", requirement.feature)?;
                    write_support(f, "    ", &requirement.support)?;
                }
                write_sources(f, row.sources)
            }
            Known::Baseline { baseline, safety } => {
                head(f, lookup, baseline.prefix, safety)?;
                write_sources(f, baseline.sources)
            }
            _ => Ok(()),
        }
    }
}

/// The file name and safety lines.
fn head(
    f: &mut fmt::Formatter<'_>,
    lookup: &Lookup,
    prefix: Prefix,
    safety: Safety,
) -> fmt::Result {
    match file_name(prefix, &lookup.name) {
        Some(name) => writeln!(f, "  file name: {name}")?,
        None => writeln!(f, "  file name: not known")?,
    }
    let SafetyJson { class, danger } = SafetyJson::of(safety);
    match danger {
        Some(danger) => writeln!(f, "  safety: {class} ({danger})"),
        None => writeln!(f, "  safety: {class}"),
    }
}

/// A line per application.
fn write_support(f: &mut fmt::Formatter<'_>, indent: &str, support: &SupportJson) -> fmt::Result {
    for (app, entry) in &support.0 {
        writeln!(f, "{indent}{}", entry_line(app.as_str(), entry))?;
    }
    Ok(())
}

/// One application's line: the version, then what else the cell says.
fn entry_line(app: &str, entry: &EntryJson) -> String {
    let added = match &entry.added {
        AddedJson::Version(text) => text.as_str(),
        AddedJson::Never => "no",
        AddedJson::Unknown => "not known",
    };
    let mut line = format!("{app:<13}  {added:<7}");
    if let Some(removed) = entry.removed {
        line.push_str(&format!(" removed {removed}"));
    }
    if entry.partial {
        line.push_str(" partial");
    }
    if let Some(fallback) = entry.fallback {
        line.push_str(&format!(" fallback {fallback}"));
    }
    if let Some(notes) = entry.notes {
        line.push_str(&format!(" note: {notes}"));
    }
    line.trim_end().to_owned()
}

/// The sources, a line each, or `sources: none`.
fn write_sources(f: &mut fmt::Formatter<'_>, sources: &[&str]) -> fmt::Result {
    if sources.is_empty() {
        return writeln!(f, "  sources: none");
    }
    writeln!(f, "  sources")?;
    for source in sources {
        writeln!(f, "    {source}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
