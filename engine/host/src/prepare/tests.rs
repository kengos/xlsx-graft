//! Unit tests for reading the policy and the rules together.

use super::{Prepared, RequestError, check_policy, prepare, prepare_parts, prepare_request};
use crate::input::{Input, MAX_REQUEST_BYTES};
use crate::policy::Policy;
use crate::report::FailOn;
use crate::support::{faults, json};

fn invalid(prepared: Prepared) -> (Vec<String>, FailOn) {
    match prepared {
        Prepared::Invalid {
            diagnostics,
            fail_on,
        } => (faults(&diagnostics), fail_on),
        Prepared::Ready(_) => panic!("the policy and the rules were read"),
    }
}

fn ready(prepared: &Prepared) -> (u64, FailOn) {
    match prepared {
        Prepared::Ready(options) => (
            options.limits.verify.max_formula_bytes.value,
            options.fail_on,
        ),
        Prepared::Invalid { .. } => panic!("the policy and the rules were not read"),
    }
}

#[test]
fn a_policy_read_once_is_prepared_with_the_rules_of_each_call() {
    let policy = Policy::read(Input::Bytes(
        br#"{"maxFormulaBytes": 100, "failOn": "warn"}"#,
    ))
    .unwrap();
    let rules = json(r#"{"parts": {"allow": ["macro"]}, "failOn": "error"}"#);
    assert_eq!(
        ready(&prepare(&policy, Input::Value(&rules))),
        (100, FailOn::Warn)
    );
    assert_eq!(ready(&prepare(&policy, Input::Absent)), (100, FailOn::Warn));
    let bad = json(r#"{"targets": ["foo"]}"#);
    let (said, fail_on) = invalid(prepare(&policy, Input::Value(&bad)));
    assert_eq!(said, ["target_invalid"]);
    assert_eq!(fail_on, FailOn::Warn);
}

#[test]
fn every_fault_of_the_policy_the_rules_and_the_targets_comes_at_once_policy_first() {
    let policy = br#"{"maxEntries": 0}"#;
    let rules = json(r#"{"parts": {"allow": ["x"]}, "targets": ["foo"], "failOn": "warn"}"#);
    let (said, fail_on) = invalid(prepare_parts(Input::Bytes(policy), Input::Value(&rules)));
    assert_eq!(said.len(), 3, "{said:?}");
    assert_eq!(
        said[0],
        "policy_invalid maxEntries|out_of_range|1 to 1048576"
    );
    assert!(said[1].starts_with("rules_invalid parts.allow[0]|unknown_value|"));
    assert_eq!(said[2], "target_invalid");
    assert_eq!(fail_on, FailOn::Warn);
}

#[test]
fn the_targets_are_checked_when_the_policy_is_not_valid_too() {
    let rules = json(r#"{"targets": ["foo"]}"#);
    let (said, _) = invalid(prepare_parts(Input::Bytes(b"[]"), Input::Value(&rules)));
    assert_eq!(said, ["policy_invalid |not_object|", "target_invalid"]);
}

#[test]
fn valid_settings_make_options() {
    assert_eq!(
        ready(&prepare_parts(Input::Absent, Input::Absent)),
        (65_536, FailOn::Error)
    );
}

#[test]
fn a_request_holds_the_policy_and_the_rules_each_absent_or_null() {
    let request = |text: &str| prepare_request(text.as_bytes()).unwrap();
    for text in [
        "",
        " \n",
        "null\n",
        "{}",
        r#"{"policy": null, "rules": null}"#,
    ] {
        assert_eq!(ready(&request(text)), (65_536, FailOn::Error), "{text}");
    }
    let read = request(r#"{"policy": {"maxFormulaBytes": 9}, "rules": {"failOn": "warn"}}"#);
    assert_eq!(ready(&read), (9, FailOn::Warn));
    let (said, _) = invalid(request(r#"{"policy": {"x": 1}, "rules": {"y": 1}}"#));
    assert_eq!(
        said,
        [
            "policy_invalid x|unknown_key|",
            "rules_invalid y|unknown_key|"
        ]
    );
}

#[test]
fn a_request_that_is_no_object_of_those_two_keys_is_an_error_of_the_caller() {
    let error = |bytes: &[u8]| prepare_request(bytes).unwrap_err();
    assert_eq!(error(b"[]"), RequestError::NotObject);
    assert_eq!(
        error(br#"{"policy": {}, "other": 1}"#),
        RequestError::UnknownKey("other".to_owned())
    );
    let long = format!("{{\"\\u202e{}\": 1}}", "k".repeat(300));
    let RequestError::UnknownKey(key) = error(long.as_bytes()) else {
        panic!("a key other than policy and rules")
    };
    assert_eq!(key.chars().count(), 201);
    assert!(key.ends_with('…') && !key.contains('\u{202e}'));
    assert!(matches!(error(b"{"), RequestError::NotJson(_)));
    assert!(matches!(error(&[0xff]), RequestError::NotJson(_)));
    let large = vec![b' '; MAX_REQUEST_BYTES + 1];
    assert_eq!(error(&large), RequestError::TooLarge);
    for error in [
        RequestError::TooLarge,
        RequestError::NotObject,
        RequestError::UnknownKey("k".to_owned()),
        RequestError::NotJson("m".to_owned()),
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn a_policy_is_checked_alone() {
    let ok = check_policy(Input::Bytes(br#"{"failOn": "warn"}"#));
    assert!(ok.ok);
    assert_eq!((ok.fail_on, ok.diagnostics.len()), (FailOn::Warn, 0));
    let bad = check_policy(Input::Bytes(br#"{"maxEntries": 0, "x": 1}"#));
    assert!(!bad.ok);
    assert_eq!(bad.fail_on, FailOn::Error);
    assert_eq!(bad.counts.error, 2);
    assert_eq!(bad.error, None);
}
