//! Helpers the unit tests of the modules share.

use serde_json::Value;
use xlsx_graft_diagnostics::{Diagnostic, Record};

/// The folder of the workbooks Excel saved, at the repository's root.
pub(crate) const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

/// The JSON `text` holds.
pub(crate) fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// What each record says, one string each: `<code> <key>|<reason>|<hint>`
/// for a fault of a setting, the code alone for any other diagnostic.
pub(crate) fn faults(records: &[Record]) -> Vec<String> {
    let said = |code: &str, key, reason, hint| format!("{code} {key}|{reason}|{hint}");
    (records.iter())
        .map(|record| match &record.diagnostic {
            Diagnostic::PolicyInvalid { key, reason, hint }
            | Diagnostic::RulesInvalid { key, reason, hint } => {
                said(record.diagnostic.code().as_str(), key, reason, hint)
            }
            other => other.code().as_str().to_owned(),
        })
        .collect()
}
