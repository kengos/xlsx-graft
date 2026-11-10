//! Unit tests for reading the rules the caller passes.

use super::CallerRules;
use crate::input::Input;
use crate::report::FailOn;
use crate::support::faults;
use xlsx_graft_diagnostics::{Diagnostics, PartKind};

fn read(text: &str) -> Result<CallerRules, Vec<String>> {
    let mut diagnostics = Diagnostics::new();
    let (rules, valid) = CallerRules::read_into(Input::Bytes(text.as_bytes()), &mut diagnostics);
    if valid {
        Ok(rules)
    } else {
        Err(faults(&diagnostics.finish()))
    }
}

#[test]
fn no_rules_are_empty_lists_and_no_failon() {
    let mut diagnostics = Diagnostics::new();
    let (rules, valid) = CallerRules::read_into(Input::Absent, &mut diagnostics);
    assert!(valid);
    assert_eq!(rules, CallerRules::default());
    assert_eq!(read("{}"), Ok(CallerRules::default()));
}

#[test]
fn the_rules_hold_the_templates_keys() {
    let rules = read(
        r#"{
            "targets": ["excel >= 2016", "numbers"],
            "bestEffort": ["google-sheets"],
            "functions": {"allow": ["webservice"]},
            "parts": {"allow": ["macro", "external_link"]},
            "failOn": "warn"
        }"#,
    )
    .unwrap();
    assert_eq!(rules.targets, ["excel >= 2016", "numbers"]);
    assert_eq!(rules.best_effort, ["google-sheets"]);
    assert_eq!(rules.allow_functions, ["webservice"]);
    assert_eq!(rules.allow_parts, [PartKind::Macro, PartKind::ExternalLink]);
    assert_eq!(rules.fail_on, Some(FailOn::Warn));
}

#[test]
fn every_fault_is_reported_in_one_pass_as_a_fault_of_the_rules() {
    assert_eq!(
        read(
            r#"{
                "target": [],
                "targets": "excel",
                "bestEffort": [1],
                "functions": {"allow": [""], "deny": []},
                "parts": {"allow": ["script", "macro"]},
                "failOn": "notice"
            }"#
        ),
        Err(vec![
            "rules_invalid target|unknown_key|".to_owned(),
            "rules_invalid targets|wrong_type|an array of strings".to_owned(),
            "rules_invalid bestEffort[0]|wrong_type|a string".to_owned(),
            "rules_invalid functions.deny|unknown_key|".to_owned(),
            "rules_invalid functions.allow[0]|empty|".to_owned(),
            "rules_invalid parts.allow[0]|unknown_value|macro, external_link, dde, ole, activex, data_connection, web_publish".to_owned(),
            "rules_invalid failOn|unknown_value|error, warn".to_owned(),
        ])
    );
}

#[test]
fn a_group_that_is_no_object_or_an_allow_that_is_no_list_is_reported() {
    assert_eq!(
        read(r#"{"functions": [], "parts": {"allow": 1}}"#),
        Err(vec![
            "rules_invalid functions|not_object|".to_owned(),
            "rules_invalid parts.allow|wrong_type|an array of strings".to_owned(),
        ])
    );
    assert_eq!(
        read("[]"),
        Err(vec!["rules_invalid |not_object|".to_owned()])
    );
    assert_eq!(
        read("{"),
        Err(vec!["rules_invalid |malformed|line 1 column 1".to_owned()])
    );
}

#[test]
fn a_function_allowed_with_its_workbook_prefix_is_reported() {
    assert_eq!(
        read(r#"{"functions": {"allow": ["WEBSERVICE", "_xlfn.FILTER"]}}"#),
        Err(vec![
            "rules_invalid functions.allow[1]|prefixed|_xlfn.".to_owned()
        ])
    );
}

#[test]
fn a_list_with_an_entry_that_is_no_string_is_not_read_for_target_faults() {
    assert_eq!(
        read(r#"{"targets": ["excel >= 2016", 3, "bogus"]}"#),
        Err(vec![
            "rules_invalid targets[1]|wrong_type|a string".to_owned()
        ])
    );
}
