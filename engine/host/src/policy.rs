//! [`Policy`]: what the host sets, read from JSON.
//!
//! ```json
//! {
//!   "maxFormulaBytes": 32768,
//!   "maxDiagnostics": 500,
//!   "failOn": "warn",
//!   "denyScheduled": true,
//!   "forbid": {
//!     "functions": [{ "name": "HYPERLINK", "reason": "links leave the workbook" }],
//!     "parts": [{ "kind": "ole", "reason": "no embedded objects" }]
//!   }
//! }
//! ```
//!
//! Every key may be left out. A key that is not one of those is a fault
//! (`unknown_key`), and so is a limit of 0 or past its hard max
//! ([`limits`]). A `forbid` entry names a function (any letter case, once)
//! or a kind of part (once) and says why; the reason is what a workbook that
//! holds it is told. A function is named without the prefix a workbook
//! writes it with (`WEBSERVICE`, not `_xlfn.WEBSERVICE`), as the prefix is
//! removed before a function is ruled on. When one object of the policy holds
//! the same key twice (two `maxDiagnostics`), `serde_json` keeps the later
//! value; the policy is the host's, so that is not reported.

pub mod limits;

use crate::input::{self, Input};
use crate::report::FailOn;
use crate::walk::{KeyPath, Reporter, Side};
use limits::{HostLimits, LIMITS};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use xlsx_graft_diagnostics::{Diagnostics, PartKind, Record, SettingReason};
use xlsx_graft_verify::Rules;

/// The host's policy: its limits, what it forbids and how strict it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    limits: HostLimits,
    forbidden_functions: Vec<(String, String)>,
    forbidden_parts: Vec<(PartKind, String)>,
    deny_scheduled: bool,
    fail_on: Option<FailOn>,
}

impl Policy {
    /// Reads the policy `input` holds, or every fault found in it.
    ///
    /// # Errors
    ///
    /// The `policy_invalid` records for each fault.
    pub fn read(input: Input<'_>) -> Result<Self, Vec<Record>> {
        let mut diagnostics = Diagnostics::new();
        Self::read_into(input, &mut diagnostics).ok_or_else(|| diagnostics.finish())
    }

    /// The limits in force: the engine's, and the policy's where it sets one.
    #[must_use]
    pub fn limits(&self) -> &HostLimits {
        &self.limits
    }

    /// How strict the policy makes every result; `None` when it does not say.
    #[must_use]
    pub fn fail_on(&self) -> Option<FailOn> {
        self.fail_on
    }

    /// The rules the policy sets: what it forbids and whether it denies the
    /// scheduled functions now. [`prepare`](crate::prepare) adds the caller's
    /// `allow` lists to what this returns.
    pub(crate) fn rules(&self) -> Rules {
        let mut rules = Rules::default();
        if self.deny_scheduled {
            rules = rules.deny_scheduled();
        }
        for (name, reason) in &self.forbidden_functions {
            rules = rules.forbid_function(name, reason);
        }
        for (kind, reason) in &self.forbidden_parts {
            rules = rules.forbid_part(*kind, reason);
        }
        rules
    }

    /// Reads `input`, pushing each fault into `diagnostics`; `None` when
    /// there was one.
    pub(crate) fn read_into(input: Input<'_>, diagnostics: &mut Diagnostics) -> Option<Self> {
        let mut reporter = Reporter::new(Side::Policy, diagnostics);
        let value = input::read(input, &mut reporter)?;
        let mut known: Vec<&str> = LIMITS.iter().map(|key| key.name).collect();
        known.extend(["failOn", "denyScheduled", "forbid"]);
        let top = KeyPath::default();
        let map = reporter.object(&value, &top, &known)?;
        let mut policy = Self::default();
        policy.read_limits(map, &top, &mut reporter);
        policy.fail_on = (map.get("failOn"))
            .and_then(|value| FailOn::read(value, &top.key("failOn"), &mut reporter));
        if let Some(value) = map.get("denyScheduled") {
            let read = reporter.boolean(value, &top.key("denyScheduled"));
            policy.deny_scheduled = read.unwrap_or(false);
        }
        if let Some(value) = map.get("forbid") {
            policy.read_forbid(value, &top.key("forbid"), &mut reporter);
        }
        (!reporter.failed()).then_some(policy)
    }

    fn read_limits(&mut self, map: &Map<String, Value>, top: &KeyPath, reporter: &mut Reporter) {
        for key in LIMITS {
            let Some(value) = map.get(key.name) else {
                continue;
            };
            if let Some(read) = reporter.limit(value, &top.key(key.name), key.hard_max) {
                key.set(&mut self.limits, read);
            }
        }
    }

    fn read_forbid(&mut self, value: &Value, at: &KeyPath, reporter: &mut Reporter) {
        let Some(forbid) = reporter.object(value, at, &["functions", "parts"]) else {
            return;
        };
        if let Some(value) = forbid.get("functions") {
            self.read_functions(value, &at.key("functions"), reporter);
        }
        if let Some(value) = forbid.get("parts") {
            self.read_parts(value, &at.key("parts"), reporter);
        }
    }

    fn read_functions(&mut self, value: &Value, at: &KeyPath, reporter: &mut Reporter) {
        let mut seen = BTreeSet::new();
        for (index, entry) in reporter.array(value, at).unwrap_or(&[]).iter().enumerate() {
            let at = at.index(index);
            let Some((name, reason)) = entry_of(entry, &at, "name", reporter) else {
                continue;
            };
            let Some(name) = reporter.function_name(name, &at.key("name")) else {
                continue;
            };
            if seen.insert(name.to_ascii_uppercase()) {
                self.forbidden_functions
                    .push((name.to_owned(), reason.to_owned()));
            } else {
                reporter.push(&at.key("name"), SettingReason::Duplicate, "");
            }
        }
    }

    fn read_parts(&mut self, value: &Value, at: &KeyPath, reporter: &mut Reporter) {
        for (index, entry) in reporter.array(value, at).unwrap_or(&[]).iter().enumerate() {
            let at = at.index(index);
            let Some((kind, reason)) = entry_of(entry, &at, "kind", reporter) else {
                continue;
            };
            let Some(kind) = reporter.part_kind(kind, &at.key("kind")) else {
                continue;
            };
            if self.forbidden_parts.iter().any(|(known, _)| *known == kind) {
                reporter.push(&at.key("kind"), SettingReason::Duplicate, "");
            } else {
                self.forbidden_parts.push((kind, reason.to_owned()));
            }
        }
    }
}

/// The `subject` (`name` or `kind`) and the `reason` of one `forbid` entry;
/// each fault of it is reported.
fn entry_of<'v>(
    entry: &'v Value,
    at: &KeyPath,
    subject: &'static str,
    reporter: &mut Reporter,
) -> Option<(&'v str, &'v str)> {
    let map = reporter.object(entry, at, &[subject, "reason"])?;
    let mut read = |key: &'static str| match map.get(key) {
        Some(value) => reporter.text(value, &at.key(key)),
        None => {
            reporter.push(at, SettingReason::Required, key);
            None
        }
    };
    let (subject, reason) = (read(subject), read("reason"));
    Some((subject?, reason?))
}

#[cfg(test)]
mod tests;
