//! [`Report`]: the result of verifying a workbook, and the JSON every host
//! prints it as.
//!
//! ```json
//! {
//!   "ok": false,
//!   "failOn": "error",
//!   "counts": { "error": 2, "warn": 0, "notice": 1 },
//!   "error": null,
//!   "diagnostics": [ { "code": "part_denied", "severity": "error" } ]
//! }
//! ```
//!
//! The keys and their types are the contract; a key may be added, never
//! renamed or retyped, and the JSON has no version. `diagnostics` are in the
//! order they were found, each as [`Record`] writes it (its `origin` is
//! outside the contract); `counts` counts the ones listed, not the ones
//! dropped (`diagnostics_truncated` says how many). `error` is `null`, or
//! `{"code": "io", "message": ...}` when the engine could not open or read
//! the package: the only thing it holds, as everything the workbook shows
//! is a diagnostic. `ok` is false then, and `diagnostics` hold what was found
//! before the failure.

use crate::walk::{KeyPath, Reporter};
use serde::Serialize;
use serde_json::Value;
use std::fmt;
use xlsx_graft_diagnostics::{Diagnostics, Record, Severity};

/// How strict a result is: which diagnostic makes it not `ok`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailOn {
    /// An error does. The default.
    #[default]
    Error,
    /// A warning does too, the ones `bestEffort` raises among them. The
    /// stricter, so it is the larger.
    Warn,
}

impl FailOn {
    /// The values a setting may name, as it writes them.
    const ALL: [Self; 2] = [Self::Error, Self::Warn];

    fn wire(self) -> String {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
        }
        .to_owned()
    }

    /// The `failOn` `value` names, a fault reported when it names none.
    pub(crate) fn read(value: &Value, at: &KeyPath, reporter: &mut Reporter) -> Option<Self> {
        let text = reporter.text(value, at)?;
        reporter.named(text, at, &Self::ALL, Self::wire)
    }

    /// The stricter of two that may be left unsaid; `Error` when neither
    /// is said.
    pub(crate) fn stricter(first: Option<Self>, second: Option<Self>) -> Self {
        first.max(second).unwrap_or_default()
    }
}

/// How many diagnostics of each severity the result lists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// The errors.
    pub error: u64,
    /// The warnings.
    pub warn: u64,
    /// The notices.
    pub notice: u64,
}

impl Counts {
    /// How many of each severity `diagnostics` holds.
    #[must_use]
    pub fn tally(diagnostics: &[Record]) -> Self {
        let mut counts = Self::default();
        for record in diagnostics {
            match record.diagnostic.severity() {
                Severity::Error => counts.error = counts.error.saturating_add(1),
                Severity::Warn => counts.warn = counts.warn.saturating_add(1),
                Severity::Notice => counts.notice = counts.notice.saturating_add(1),
            }
        }
        counts
    }
}

impl fmt::Display for Counts {
    /// `2 errors, 1 warning, 0 notices`: each count with its noun, singular
    /// for 1.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let noun = |count: u64, one: &'static str, many: &'static str| {
            if count == 1 { one } else { many }
        };
        write!(
            f,
            "{} {}, {} {}, {} {}",
            self.error,
            noun(self.error, "error", "errors"),
            self.warn,
            noun(self.warn, "warning", "warnings"),
            self.notice,
            noun(self.notice, "notice", "notices"),
        )
    }
}

/// The engine failed to open or read the package, apart from anything the
/// workbook holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Failure {
    /// The stable code: `io`.
    pub code: &'static str,
    /// What failed, in English.
    pub message: String,
}

/// The result of verifying a workbook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// No error, no diagnostic as strong as `fail_on`, and no `error`.
    pub ok: bool,
    /// How strict this result is: the stricter of the policy and the caller.
    pub fail_on: FailOn,
    /// How many of each severity `diagnostics` lists.
    pub counts: Counts,
    /// Why the engine could not look at the package, when it could not.
    pub error: Option<Failure>,
    /// Every problem found, in the order found.
    pub diagnostics: Vec<Record>,
}

impl Report {
    /// The result of a run that gave `diagnostics`, stopped by `error`
    /// when it was.
    pub(crate) fn finished(
        diagnostics: Diagnostics,
        fail_on: FailOn,
        error: Option<Failure>,
    ) -> Self {
        let failed = diagnostics.has_errors()
            || (fail_on == FailOn::Warn && diagnostics.has_warnings())
            || error.is_some();
        Self::of(!failed, fail_on, error, diagnostics.finish())
    }

    /// The result of a run that did not look at the workbook, as the
    /// policy or the rules were not valid: never `ok`.
    pub(crate) fn refused(diagnostics: Vec<Record>, fail_on: FailOn) -> Self {
        Self::of(false, fail_on, None, diagnostics)
    }

    /// The result of a verification the host could not start, as a file it
    /// had to read (the workbook, the policy) could not be read: not `ok`,
    /// no diagnostic, and `error` an `io` failure saying `message`.
    #[must_use]
    pub fn io_failure(message: String, fail_on: FailOn) -> Self {
        Self::of(false, fail_on, Some(Failure::io(message)), Vec::new())
    }

    fn of(ok: bool, fail_on: FailOn, error: Option<Failure>, diagnostics: Vec<Record>) -> Self {
        let counts = Counts::tally(&diagnostics);
        Self {
            ok,
            fail_on,
            counts,
            error,
            diagnostics,
        }
    }

    /// The result as the JSON every host prints, in one line.
    ///
    /// # Errors
    ///
    /// Never in practice: every field serializes. `serde_json`'s own error
    /// is returned rather than hidden.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

impl Failure {
    /// An `io` failure: `message` says what could not be done.
    pub(crate) fn io(message: String) -> Self {
        Self {
            code: "io",
            message,
        }
    }
}

#[cfg(test)]
mod tests;
