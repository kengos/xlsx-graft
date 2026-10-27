//! Unit tests for the JSON shapes of `DecidedBy` and `Fix`.

use super::{DecidedBy, Fix};
use crate::Limit;

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn a_standard_rule_serializes_with_its_clause() {
    let decided = DecidedBy::standard("ECMA-376-2:2021 §7.2.3.2.1");
    assert_eq!(
        json(&decided),
        r#"{"kind":"standard","ref":"ECMA-376-2:2021 §7.2.3.2.1"}"#
    );
}

#[test]
fn a_limit_serializes_with_its_value_and_default_source() {
    let decided = DecidedBy::limit("maxEntries", Limit::default_of(10_000));
    assert_eq!(
        json(&decided),
        r#"{"kind":"limit","name":"maxEntries","value":10000,"source":"default"}"#
    );
}

#[test]
fn a_workbook_fix_serializes_with_its_text() {
    let fix = Fix::workbook("open the workbook in Excel and save it again");
    assert_eq!(
        json(&fix),
        r#"{"in":"workbook","text":"open the workbook in Excel and save it again"}"#
    );
}

#[test]
fn a_policy_fix_serializes_with_its_line() {
    assert_eq!(
        json(&Fix::policy("maxEntries: 12000")),
        r#"{"in":"policy","line":"maxEntries: 12000"}"#
    );
}
