//! [`CallerRules`]: what the caller of a verification passes, read from
//! JSON. The keys are the template's, so a template's lists can be passed
//! on as they are.
//!
//! ```json
//! {
//!   "targets": ["excel >= 2016"],
//!   "bestEffort": ["google-sheets"],
//!   "functions": { "allow": ["WEBSERVICE"] },
//!   "parts": { "allow": ["macro"] },
//!   "failOn": "warn"
//! }
//! ```
//!
//! Every key may be left out. `targets` and `bestEffort` are read by
//! [`Targets::parse`](xlsx_graft_verify::Targets::parse), which reports
//! each entry that names no version as `target_invalid`. An `allow` entry is
//! a string; a function's in any letter case, a part's as the kind's name
//! (`external_link`). A function is named without the prefix a workbook
//! writes it with (`WEBSERVICE`, not `_xlfn.WEBSERVICE`). The policy's forbids
//! hold over the caller's `functions.allow` and `parts.allow` entries.
//!
//! When an entry of `targets` or `bestEffort` is not a string, that is the
//! fault reported for it, and the list is not read for `target_invalid`: its
//! entries would be counted from the wrong position.

use crate::input::{self, Input};
use crate::report::FailOn;
use crate::walk::{KeyPath, Reporter, Side};
use serde_json::{Map, Value};
use xlsx_graft_diagnostics::{Diagnostics, PartKind};

/// The rules the caller passes: where the workbook must work, what it
/// allows that is denied by default, and how strict to be.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CallerRules {
    pub(crate) targets: Vec<String>,
    pub(crate) best_effort: Vec<String>,
    pub(crate) allow_functions: Vec<String>,
    pub(crate) allow_parts: Vec<PartKind>,
    pub(crate) fail_on: Option<FailOn>,
}

impl CallerRules {
    /// Reads `input`, pushing each fault into `diagnostics`: the rules as
    /// far as they could be read, and whether they were read without one.
    pub(crate) fn read_into(input: Input<'_>, diagnostics: &mut Diagnostics) -> (Self, bool) {
        let mut reporter = Reporter::new(Side::Rules, diagnostics);
        let Some(value) = input::read(input, &mut reporter) else {
            return (Self::default(), false);
        };
        let top = KeyPath::default();
        let known = ["targets", "bestEffort", "functions", "parts", "failOn"];
        let Some(map) = reporter.object(&value, &top, &known) else {
            return (Self::default(), false);
        };
        let targets = list(map, "targets", &top, &mut reporter);
        let best_effort = list(map, "bestEffort", &top, &mut reporter);
        let functions = allow(map, "functions", &top, &mut reporter);
        let allow_functions = (functions.iter().enumerate())
            .filter_map(|(index, name)| {
                let at = top.key("functions").key("allow").index(index);
                reporter.function_name(name, &at).map(str::to_owned)
            })
            .collect();
        let kinds = allow(map, "parts", &top, &mut reporter);
        let allow_parts = (kinds.iter().enumerate())
            .filter_map(|(index, kind)| {
                let at = top.key("parts").key("allow").index(index);
                reporter.part_kind(kind, &at)
            })
            .collect();
        let fail_on = map
            .get("failOn")
            .and_then(|value| FailOn::read(value, &top.key("failOn"), &mut reporter));
        let rules = Self {
            targets,
            best_effort,
            allow_functions,
            allow_parts,
            fail_on,
        };
        (rules, !reporter.failed())
    }
}

/// The strings of the array `key` of `map`, none when it is not there.
fn list(map: &Map<String, Value>, key: &str, at: &KeyPath, reporter: &mut Reporter) -> Vec<String> {
    let Some(value) = map.get(key) else {
        return Vec::new();
    };
    let read = reporter.texts(value, &at.key(key));
    read.unwrap_or_default()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// The strings of `allow` of the object `key` of `map` (`functions`,
/// `parts`).
fn allow(
    map: &Map<String, Value>,
    key: &str,
    at: &KeyPath,
    reporter: &mut Reporter,
) -> Vec<String> {
    let Some(value) = map.get(key) else {
        return Vec::new();
    };
    let at = at.key(key);
    let Some(object) = reporter.object(value, &at, &["allow"]) else {
        return Vec::new();
    };
    list(object, "allow", &at, reporter)
}

#[cfg(test)]
mod tests;
