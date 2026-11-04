//! One `targets:` or `bestEffort:` entry, read into an application and the
//! versions it holds.
//!
//! The grammar is small on purpose: `name`, `name version` or
//! `name OP version`, OP one of `>=`, `>`, `<=` and `<`. Only an application
//! without versions (Google Sheets) is written by its name alone. Names and
//! versions are matched as written, without folding case. White space may
//! stand around the operator, and nothing may follow the version.

use super::versions::Versions;
use crate::App;
use xlsx_graft_diagnostics::TargetReason;

/// Why an entry names no versions, and what the message lists for it (see
/// `target_invalid`'s `names`).
pub(super) type Problem = (TargetReason, String);

/// A comparison an entry makes against its version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    /// `>=`: the version and every later one.
    AtLeast,
    /// `>`: every version after it.
    After,
    /// `<=`: the version and every earlier one.
    AtMost,
    /// `<`: every version before it.
    Before,
}

impl Operator {
    /// The operators, the two-character ones first so that `>=` is not
    /// read as `>` followed by `=`.
    const ALL: [(&'static str, Self); 4] = [
        (">=", Self::AtLeast),
        ("<=", Self::AtMost),
        (">", Self::After),
        ("<", Self::Before),
    ];
}

/// An entry split into its name, operator and version.
struct Parts<'a> {
    name: &'a str,
    operator: Option<Operator>,
    version: Option<&'a str>,
}

/// Reads `entry` into the application it names and the versions it holds.
pub(super) fn read(entry: &str) -> Result<(App, Versions), Problem> {
    let Some(parts) = split(entry) else {
        return Err((TargetReason::Malformed, String::new()));
    };
    let Some(app) = App::ALL
        .iter()
        .copied()
        .find(|app| app.as_str() == parts.name)
    else {
        let apps: Vec<&str> = App::ALL.iter().map(|app| app.as_str()).collect();
        return Err((TargetReason::UnknownApp, apps.join(", ")));
    };
    let names = app.version_names();
    if !app.has_versions() {
        return match parts.version {
            None => Ok((app, Versions::range(0, names.len()))),
            Some(_) => Err((TargetReason::VersionNotAllowed, app.as_str().to_owned())),
        };
    }
    let Some(version) = parts.version else {
        return Err((TargetReason::VersionRequired, names.join(", ")));
    };
    let Some(at) = names.iter().position(|name| *name == version) else {
        return Err((TargetReason::UnknownVersion, names.join(", ")));
    };
    let next = at.saturating_add(1);
    let (start, end) = match parts.operator {
        None => (at, next),
        Some(Operator::AtLeast) => (at, names.len()),
        Some(Operator::After) => (next, names.len()),
        Some(Operator::AtMost) => (0, next),
        Some(Operator::Before) => (0, at),
    };
    let versions = Versions::range(start, end);
    if versions.is_empty() {
        return Err((TargetReason::Empty, names.join(", ")));
    }
    Ok((app, versions))
}

/// Splits `entry` into its parts, or `None` when it is none of the forms.
fn split(entry: &str) -> Option<Parts<'_>> {
    let entry = entry.trim();
    let is_operator = |c: char| matches!(c, '<' | '>' | '=');
    let name_end = entry
        .find(|c: char| c.is_whitespace() || is_operator(c))
        .unwrap_or(entry.len());
    let (name, rest) = entry.split_at_checked(name_end)?;
    let rest = rest.trim_start();
    let (operator, rest) = Operator::ALL
        .iter()
        .find_map(|(text, operator)| Some((Some(*operator), rest.strip_prefix(text)?)))
        .unwrap_or((None, rest));
    let version = rest.trim_start();
    let version = if version.is_empty() {
        None
    } else {
        Some(version)
    };
    let malformed = name.is_empty()
        || (operator.is_some() && version.is_none())
        || version
            .is_some_and(|version| version.contains(|c: char| c.is_whitespace() || is_operator(c)));
    (!malformed).then_some(Parts {
        name,
        operator,
        version,
    })
}
