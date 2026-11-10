//! Unit tests for reading the host's policy.

use super::Policy;
use crate::input::{Input, MAX_SETTING_BYTES};
use crate::policy::limits::LIMITS;
use crate::report::FailOn;
use crate::support::{faults, json};
use xlsx_graft_diagnostics::{LimitSource, PartKind};
use xlsx_graft_verify::Ruling;

fn read(text: &str) -> Result<Policy, Vec<String>> {
    Policy::read(Input::Bytes(text.as_bytes())).map_err(|records| faults(&records))
}

#[test]
fn no_policy_holds_every_default() {
    for input in [Input::Absent, Input::Bytes(b""), Input::Bytes(b"{}")] {
        let policy = Policy::read(input).unwrap();
        assert_eq!(policy, Policy::default());
        assert_eq!(policy.fail_on(), None);
        assert_eq!(policy.rules(), xlsx_graft_verify::Rules::default());
    }
}

#[test]
fn a_policy_sets_limits_what_it_forbids_and_how_strict_it_is() {
    let policy = read(
        r#"{
            "maxFormulaBytes": 32768,
            "maxEntries": 20000,
            "maxDiagnostics": 500,
            "failOn": "warn",
            "denyScheduled": true,
            "forbid": {
              "functions": [{"name": "hyperlink", "reason": "links leave the workbook"}],
              "parts": [{"kind": "ole", "reason": "no embedded objects"}]
            }
        }"#,
    )
    .unwrap();
    let limits = policy.limits();
    assert_eq!(
        (
            limits.verify.max_formula_bytes.value,
            limits.verify.max_formula_bytes.source
        ),
        (32_768, LimitSource::Policy)
    );
    assert_eq!(limits.verify.package.zip.max_entries.value, 20_000);
    assert_eq!(
        (limits.max_diagnostics.value, limits.max_diagnostics.source),
        (500, LimitSource::Policy)
    );
    assert_eq!(limits.verify.max_defined_names.source, LimitSource::Default);
    assert_eq!(policy.fail_on(), Some(FailOn::Warn));
    let rules = policy.rules();
    assert!(rules.denies_scheduled());
    assert_eq!(
        rules.function("HYPERLINK"),
        Ruling::Forbidden {
            reason: "links leave the workbook"
        }
    );
    assert_eq!(
        rules.part(PartKind::Ole),
        Ruling::Forbidden {
            reason: "no embedded objects"
        }
    );
    assert_eq!(rules.part(PartKind::Macro), Ruling::Unruled);
}

#[test]
fn a_policy_of_a_value_is_read_as_the_text_is() {
    let value = json(r#"{"denyScheduled": false, "failOn": "error"}"#);
    let policy = Policy::read(Input::Value(&value)).unwrap();
    assert!(!policy.rules().denies_scheduled());
    assert_eq!(policy.fail_on(), Some(FailOn::Error));
}

#[test]
fn every_limit_is_set_up_to_its_hard_max_and_no_further() {
    for key in LIMITS {
        let at = |value: u64| format!(r#"{{"{}": {value}}}"#, key.name);
        let policy = read(&at(key.hard_max)).unwrap();
        let rules_say = format!(
            "policy_invalid {}|out_of_range|1 to {}",
            key.name, key.hard_max
        );
        assert_ne!(
            policy.limits(),
            &super::HostLimits::default(),
            "{}",
            key.name
        );
        assert_eq!(
            read(&at(key.hard_max + 1)),
            Err(vec![rules_say.clone()]),
            "{}",
            key.name
        );
        assert_eq!(read(&at(0)), Err(vec![rules_say]), "{}", key.name);
    }
}

#[test]
fn every_fault_is_reported_in_one_pass() {
    let said = read(
        r#"{
            "max_entries": 1,
            "maxXmlDepth": "deep",
            "maxEntryBytes": 0,
            "failOn": "notice",
            "denyScheduled": "yes",
            "forbid": {
              "functions": [
                {"name": "A", "reason": "x"},
                {"name": "a", "reason": "y"},
                {"name": "", "reason": "z"},
                {"reason": "no name"},
                {"name": "B"},
                {"name": "C", "reason": "z", "why": "z"},
                1
              ],
              "parts": [
                {"kind": "macro", "reason": "x"},
                {"kind": "macro", "reason": "y"},
                {"kind": "script", "reason": "z"}
              ],
              "others": []
            }
        }"#,
    );
    assert_eq!(
        said,
        Err(vec![
            "policy_invalid max_entries|unknown_key|maxEntries".to_owned(),
            "policy_invalid maxEntryBytes|out_of_range|1 to 9007199254740991".to_owned(),
            "policy_invalid maxXmlDepth|wrong_type|a positive integer".to_owned(),
            "policy_invalid failOn|unknown_value|error, warn".to_owned(),
            "policy_invalid denyScheduled|wrong_type|true or false".to_owned(),
            "policy_invalid forbid.others|unknown_key|".to_owned(),
            "policy_invalid forbid.functions[1].name|duplicate|".to_owned(),
            "policy_invalid forbid.functions[2].name|empty|".to_owned(),
            "policy_invalid forbid.functions[3]|required|name".to_owned(),
            "policy_invalid forbid.functions[4]|required|reason".to_owned(),
            "policy_invalid forbid.functions[5].why|unknown_key|".to_owned(),
            "policy_invalid forbid.functions[6]|not_object|".to_owned(),
            "policy_invalid forbid.parts[1].kind|duplicate|".to_owned(),
            "policy_invalid forbid.parts[2].kind|unknown_value|macro, external_link, dde, ole, activex, data_connection, web_publish".to_owned(),
        ])
    );
}

#[test]
fn a_forbid_of_the_wrong_shape_is_reported_where_it_is() {
    assert_eq!(
        read(r#"{"forbid": []}"#),
        Err(vec!["policy_invalid forbid|not_object|".to_owned()])
    );
    assert_eq!(
        read(r#"{"forbid": {"functions": {}, "parts": 1}}"#),
        Err(vec![
            "policy_invalid forbid.functions|wrong_type|an array of objects".to_owned(),
            "policy_invalid forbid.parts|wrong_type|an array of objects".to_owned(),
        ])
    );
    assert_eq!(
        read(r#"{"forbid": {"parts": [{"kind": 1, "reason": 2}]}}"#),
        Err(vec![
            "policy_invalid forbid.parts[0].kind|wrong_type|a string".to_owned(),
            "policy_invalid forbid.parts[0].reason|wrong_type|a string".to_owned(),
        ])
    );
}

#[test]
fn a_policy_that_is_no_object_or_not_json_or_too_large_is_one_fault() {
    assert_eq!(
        read("[]"),
        Err(vec!["policy_invalid |not_object|".to_owned()])
    );
    assert_eq!(
        read("{"),
        Err(vec!["policy_invalid |malformed|line 1 column 1".to_owned()])
    );
    let large = " ".repeat(MAX_SETTING_BYTES + 1);
    assert_eq!(
        read(&large),
        Err(vec![format!(
            "policy_invalid |too_large|{MAX_SETTING_BYTES}"
        )])
    );
}

#[test]
fn a_function_forbidden_with_its_workbook_prefix_is_reported() {
    assert_eq!(
        read(r#"{"forbid": {"functions": [{"name": "_xlfn.WEBSERVICE", "reason": "x"}]}}"#),
        Err(vec![
            "policy_invalid forbid.functions[0].name|prefixed|_xlfn.".to_owned()
        ])
    );
}
