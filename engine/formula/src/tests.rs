//! Tests for `analyze`, driven through it. Each file under `tests/` covers
//! one aspect; the helpers here list a formula's facts as plain text, so a
//! case reads as the formula and what it states.

mod bypass;
mod excel;
mod facts;
mod lexing;
mod properties;

use crate::{Formula, analyze};

/// The calls `formula` states, each as its prefix, its name, and `*` when
/// it is passed as a value.
pub(crate) fn calls(formula: &str) -> Vec<String> {
    analyze(formula)
        .facts
        .calls
        .iter()
        .map(|call| {
            let passed = if call.passed { "*" } else { "" };
            format!("{}{}{passed}", call.prefix, call.name)
        })
        .collect()
}

/// The names of the unseen calls in `formula`.
pub(crate) fn unseen(formula: &str) -> Vec<String> {
    analyze(formula)
        .facts
        .unseen_calls
        .iter()
        .map(|call| call.name.clone())
        .collect()
}

/// The text of each token of `formula`, in order.
pub(crate) fn texts(formula: &Formula) -> Vec<&str> {
    formula
        .tokens
        .iter()
        .map(|token| formula.slice(token.span))
        .collect()
}
