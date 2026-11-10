//! [`Prepared`]: the policy and the rules read, or why they could not be,
//! before any workbook is opened.
//!
//! A host reads the policy once and the rules per call ([`prepare`]), or
//! both together ([`prepare_parts`], [`prepare_request`]). Every fault of
//! the policy, the rules and the `targets` and `bestEffort` lists is found
//! in one pass and reported together, and then the workbook is not opened.

use crate::input::{Input, MAX_REQUEST_BYTES};
use crate::policy::Policy;
use crate::policy::limits::HostLimits;
use crate::report::{FailOn, Report};
use crate::rules::CallerRules;
use serde_json::Value;
use thiserror::Error;
use xlsx_graft_diagnostics::{Diagnostics, Echo, Record};
use xlsx_graft_verify::{Rules, Targets};

/// What a verification runs under, as the policy and the rules make it.
#[derive(Debug, Clone)]
pub struct Options {
    pub(crate) limits: HostLimits,
    pub(crate) rules: Rules,
    pub(crate) targets: Targets,
    pub(crate) fail_on: FailOn,
}

/// The policy and the rules, read.
#[derive(Debug, Clone)]
pub enum Prepared {
    /// The policy and the rules are valid: a workbook can be verified under
    /// the policy and the rules.
    Ready(Box<Options>),
    /// The policy or the rules are not valid: the faults, as `policy_invalid`, `rules_invalid` and
    /// `target_invalid` records, and how strict the result is.
    Invalid {
        /// Every fault found.
        diagnostics: Vec<Record>,
        /// The stricter `failOn` the policy and the rules say.
        fail_on: FailOn,
    },
}

/// A request that is not an object of `policy` and `rules`, a fault of the
/// caller's code rather than of the policy or the rules.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RequestError {
    /// The request is larger than [`MAX_REQUEST_BYTES`].
    #[error("the request is too large")]
    TooLarge,
    /// The request is not JSON.
    #[error("the request is not JSON: {0}")]
    NotJson(String),
    /// The request is JSON, but not an object.
    #[error("the request is not a JSON object")]
    NotObject,
    /// The request has a key other than `policy` and `rules`, which is
    /// quoted as a diagnostic quotes a name (cleaned and clipped).
    #[error("the request has a key other than `policy` and `rules`: {0}")]
    UnknownKey(String),
}

/// The policy `policy` has read and the rules `rules` holds.
#[must_use]
pub fn prepare(policy: &Policy, rules: Input<'_>) -> Prepared {
    let mut diagnostics = Diagnostics::new();
    let caller = CallerRules::read_into(rules, &mut diagnostics);
    finish(Some(policy.clone()), caller, diagnostics)
}

/// The policy and the rules, both read from `policy` and `rules`.
#[must_use]
pub fn prepare_parts(policy: Input<'_>, rules: Input<'_>) -> Prepared {
    let mut diagnostics = Diagnostics::new();
    let policy = Policy::read_into(policy, &mut diagnostics);
    let caller = CallerRules::read_into(rules, &mut diagnostics);
    finish(policy, caller, diagnostics)
}

/// The policy and the rules, both read from the request `json`:
/// `{"policy": {...}, "rules": {...}}`, each of which may be absent or
/// `null`; an empty or whitespace-only request, or `null`, is read as both
/// absent.
///
/// # Errors
///
/// [`RequestError`] when `json` is no object of those two keys. That is a
/// fault of the code that built the request; a fault of the policy or the
/// rules in it is a [`Prepared::Invalid`].
pub fn prepare_request(json: &[u8]) -> Result<Prepared, RequestError> {
    if json.len() > MAX_REQUEST_BYTES {
        return Err(RequestError::TooLarge);
    }
    if json.iter().all(u8::is_ascii_whitespace) {
        return Ok(prepare_parts(Input::Absent, Input::Absent));
    }
    let value: Value =
        serde_json::from_slice(json).map_err(|error| RequestError::NotJson(error.to_string()))?;
    let map = match &value {
        Value::Object(map) => map,
        Value::Null => return Ok(prepare_parts(Input::Absent, Input::Absent)),
        _ => return Err(RequestError::NotObject),
    };
    if let Some(key) = map
        .keys()
        .find(|key| !["policy", "rules"].contains(&key.as_str()))
    {
        return Err(RequestError::UnknownKey(Echo::new(key).to_string()));
    }
    let part = |key: &str| match map.get(key) {
        Some(Value::Null) | None => Input::Absent,
        Some(value) => Input::Value(value),
    };
    Ok(prepare_parts(part("policy"), part("rules")))
}

/// Checks the policy alone, for a host that reads it before any workbook
/// (an SDK that is given a policy, the server that is started with one):
/// the report is `ok` when the policy is valid, and holds its
/// `policy_invalid` records when it is not.
#[must_use]
pub fn check_policy(policy: Input<'_>) -> Report {
    match Policy::read(policy) {
        Ok(policy) => Report::finished(
            Diagnostics::new(),
            policy.fail_on().unwrap_or_default(),
            None,
        ),
        Err(records) => Report::refused(records, FailOn::default()),
    }
}

/// The result of what was read: ready, or every fault found. The targets
/// are read for their faults whatever the rest held.
fn finish(
    policy: Option<Policy>,
    (caller, caller_valid): (CallerRules, bool),
    mut diagnostics: Diagnostics,
) -> Prepared {
    let fail_on = FailOn::stricter(policy.as_ref().and_then(Policy::fail_on), caller.fail_on);
    let targets = Targets::parse(&caller.targets, &caller.best_effort, &mut diagnostics);
    match (policy, targets) {
        (Some(policy), Some(targets)) if caller_valid => {
            let mut rules = policy.rules();
            for name in &caller.allow_functions {
                rules = rules.allow_function(name);
            }
            for kind in &caller.allow_parts {
                rules = rules.allow_part(*kind);
            }
            Prepared::Ready(Box::new(Options {
                limits: *policy.limits(),
                rules,
                targets,
                fail_on,
            }))
        }
        _ => Prepared::Invalid {
            diagnostics: diagnostics.finish(),
            fail_on,
        },
    }
}

#[cfg(test)]
mod tests;
