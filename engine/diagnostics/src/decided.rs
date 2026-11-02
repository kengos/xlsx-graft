//! Where a diagnostic's rule comes from ([`DecidedBy`]) and how to resolve it
//! ([`Fix`]), the two fields a code's row in the table fills in.

use crate::{Limit, LimitSource};
use serde::Serialize;

/// What decided that this is a problem.
///
/// Serialized with a `kind` tag: `{"kind":"standard","ref":"..."}`,
/// `{"kind":"limit","name":"...","value":...,"source":"..."}`, or
/// `{"kind":"default"}`, `{"kind":"template"}` and `{"kind":"policy"}` for
/// what the engine denies, the template allows and the host forbids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DecidedBy {
    /// A rule of a published standard.
    Standard {
        /// The clause, e.g. `ECMA-376-2:2021 §7.2.3.2.1`.
        #[serde(rename = "ref")]
        reference: &'static str,
    },
    /// One of the engine's limits on untrusted input.
    Limit {
        /// The limit's name, in the camelCase a policy would spell it.
        name: &'static str,
        /// The limit in force.
        value: u64,
        /// Who set the value in force.
        source: LimitSource,
    },
    /// The engine's defaults: a part of a kind it denies is refused unless
    /// the template allows it, and so is a function its table does not
    /// mark reviewed safe, and a formula it does not read for reasons of
    /// its own rather than a standard's.
    Default {},
    /// The template's `allow` lists, which admit what the defaults deny. No
    /// code cites it yet: it is for a notice about what a template admits
    /// that the engine would otherwise deny.
    Template {},
    /// The host's policy, which forbids what no template may allow.
    Policy {},
}

impl DecidedBy {
    /// A rule of a published standard, cited by its clause.
    #[must_use]
    pub fn standard(reference: &'static str) -> Self {
        Self::Standard { reference }
    }

    /// The limit `name`, with the value in force and who set it.
    #[must_use]
    pub fn limit(name: &'static str, limit: Limit) -> Self {
        Self::Limit {
            name,
            value: limit.value,
            source: limit.source,
        }
    }
}

/// How the problem can be resolved.
///
/// Serialized with an `in` tag: `{"in":"workbook","text":"..."}`,
/// `{"in":"policy","line":"..."}` or
/// `{"in":"template","list":"...","add":"..."}`. The text is an English default and may
/// change; the shape is the contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "in", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Fix {
    /// Change the workbook.
    Workbook {
        /// What to do to the workbook, in English.
        text: String,
    },
    /// Raise a limit in the policy.
    Policy {
        /// The policy line that would admit this workbook.
        line: String,
    },
    /// Add a value to one of the template's lists.
    Template {
        /// The list, such as `parts.allow`.
        list: &'static str,
        /// The value to add to it.
        add: String,
    },
}

impl Fix {
    /// Change the workbook as `text` says.
    #[must_use]
    pub fn workbook(text: impl Into<String>) -> Self {
        Self::Workbook { text: text.into() }
    }

    /// Add `line` to the policy.
    #[must_use]
    pub fn policy(line: impl Into<String>) -> Self {
        Self::Policy { line: line.into() }
    }

    /// Add `value` to the template's list `list`.
    #[must_use]
    pub fn template(list: &'static str, value: impl Into<String>) -> Self {
        Self::Template {
            list,
            add: value.into(),
        }
    }
}

#[cfg(test)]
mod tests;
