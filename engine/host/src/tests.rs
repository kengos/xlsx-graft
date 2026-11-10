//! Tests for verifying workbooks through the host: the workbooks Excel saved
//! in `fixtures/`, under policies and rules.

use crate::support::{FIXTURES, faults, json};
use crate::{
    FailOn, Input, Prepared, check_policy, prepare_parts, prepare_request, verify, verify_bytes,
    verify_path,
};
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(FIXTURES).join(name)
}

fn prepared(policy: &str, rules: &str) -> Prepared {
    prepare_parts(
        Input::Bytes(policy.as_bytes()),
        Input::Bytes(rules.as_bytes()),
    )
}

/// The JSON of verifying the fixture `name` under `policy` and `rules`.
fn verified(name: &str, policy: &str, rules: &str) -> Value {
    let report = verify_path(&fixture(name), &prepared(policy, rules));
    json(&report.to_json().unwrap())
}

fn codes(report: &Value) -> Vec<&str> {
    let listed = report["diagnostics"].as_array().unwrap();
    listed.iter().map(|d| d["code"].as_str().unwrap()).collect()
}

#[test]
fn a_workbook_with_a_macro_is_refused_by_default_with_the_json_every_host_returns() {
    let report = verify_path(&fixture("macro.xlsm"), &prepared("", ""));
    let text = report.to_json().unwrap();
    assert!(text.starts_with(
        r#"{"ok":false,"failOn":"error","counts":{"error":2,"warn":0,"notice":0},"error":null,"diagnostics":[{"code":"part_denied","severity":"error""#
    ), "{text}");
    assert_eq!(codes(&json(&text)), ["part_denied", "part_denied"]);
}

#[test]
fn a_workbook_that_holds_nothing_denied_is_ok() {
    let text = verify_path(&fixture("table.xlsx"), &prepared("", ""))
        .to_json()
        .unwrap();
    assert_eq!(
        text,
        r#"{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},"error":null,"diagnostics":[]}"#
    );
}

#[test]
fn the_same_bytes_give_the_same_report_as_the_path() {
    let bytes = std::fs::read(fixture("external-link.xlsx")).unwrap();
    let by_bytes = verify_bytes(&bytes, &prepared("", "")).to_json().unwrap();
    let by_path = verify_path(&fixture("external-link.xlsx"), &prepared("", ""))
        .to_json()
        .unwrap();
    assert_eq!(by_bytes, by_path);
    assert_eq!(codes(&json(&by_bytes)), ["part_denied", "formula_denied"]);
}

#[test]
fn the_callers_allow_admits_what_is_denied_by_default() {
    let report = verified("macro.xlsm", "", r#"{"parts": {"allow": ["macro"]}}"#);
    assert_eq!(
        (report["ok"].as_bool(), codes(&report)),
        (Some(true), vec![])
    );
}

#[test]
fn the_policys_forbid_holds_over_the_callers_allow() {
    let policy = r#"{"forbid": {"parts": [{"kind": "macro", "reason": "no macros here"}]}}"#;
    let report = verified("macro.xlsm", policy, r#"{"parts": {"allow": ["macro"]}}"#);
    assert_eq!(report["ok"], false);
    assert_eq!(codes(&report), ["part_forbidden", "part_forbidden"]);
    assert_eq!(report["diagnostics"][0]["decidedBy"]["kind"], "policy");
}

#[test]
fn a_limit_the_policy_sets_is_named_as_the_policys_in_what_it_decides() {
    let report = verified("table.xlsx", r#"{"maxEntries": 1}"#, "");
    assert_eq!(report["ok"], false);
    let first = &report["diagnostics"][0];
    assert_eq!(first["code"], "zip_entry_count_exceeded");
    assert_eq!(first["decidedBy"]["name"], "maxEntries");
    assert_eq!(first["decidedBy"]["source"], "policy");
    assert!(first["fix"]["line"].as_str().is_some());
}

#[test]
fn the_policy_bounds_how_many_diagnostics_are_listed() {
    let report = verified("macro.xlsm", r#"{"maxDiagnostics": 1}"#, "");
    assert_eq!(codes(&report), ["part_denied", "diagnostics_truncated"]);
    assert_eq!(
        report["counts"],
        json(r#"{"error": 1, "warn": 0, "notice": 1}"#)
    );
    assert_eq!(report["ok"], false);
    assert_eq!(report["diagnostics"][1]["decidedBy"]["source"], "policy");
}

#[test]
fn failon_decides_whether_a_best_effort_warning_makes_the_report_not_ok() {
    let rules = r#"{"bestEffort": ["excel 2016"]}"#;
    let lenient = verified("dynamic-array.xlsx", "", rules);
    assert_eq!(lenient["ok"], true);
    assert!(lenient["counts"]["warn"].as_u64().unwrap() > 0, "{lenient}");
    let strict = verified(
        "dynamic-array.xlsx",
        "",
        r#"{"bestEffort": ["excel 2016"], "failOn": "warn"}"#,
    );
    assert_eq!(
        (strict["ok"].as_bool(), strict["failOn"].as_str()),
        (Some(false), Some("warn"))
    );
    // The policy's `warn` holds over the caller's `error`.
    let policy = verified(
        "dynamic-array.xlsx",
        r#"{"failOn": "warn"}"#,
        r#"{"bestEffort": ["excel 2016"], "failOn": "error"}"#,
    );
    assert_eq!(
        (policy["ok"].as_bool(), policy["failOn"].as_str()),
        (Some(false), Some("warn"))
    );
}

#[test]
fn a_policy_and_rules_that_are_not_valid_are_reported_together_and_nothing_is_opened() {
    let invalid = prepared(
        r#"{"maxEntries": 0}"#,
        r#"{"targets": ["foo"], "parts": {"allow": ["x"]}}"#,
    );
    // A path that is not there and bytes that are no workbook: neither is read.
    let by_path = verify_path(Path::new("/no/such/workbook.xlsx"), &invalid);
    let by_bytes = verify_bytes(b"not a zip", &invalid);
    for report in [&by_path, &by_bytes] {
        assert!(!report.ok);
        assert_eq!(report.error, None);
        assert_eq!(report.counts.error, 3);
        let said = faults(&report.diagnostics);
        assert_eq!(
            said[0],
            "policy_invalid maxEntries|out_of_range|1 to 1048576"
        );
        assert!(
            said[1].starts_with("rules_invalid parts.allow[0]|unknown_value|"),
            "{said:?}"
        );
        assert_eq!(said[2], "target_invalid");
    }
}

#[test]
fn a_file_that_cannot_be_opened_is_an_io_error_in_the_report() {
    let report = verify_path(Path::new("/no/such/workbook.xlsx"), &prepared("", ""));
    assert!(!report.ok);
    let failure = report.error.as_ref().unwrap();
    assert_eq!(failure.code, "io");
    assert!(
        failure
            .message
            .starts_with("/no/such/workbook.xlsx could not be opened: "),
        "{}",
        failure.message
    );
    assert!(report.diagnostics.is_empty());
    let value = json(&report.to_json().unwrap());
    assert_eq!(value["error"]["code"], "io");
}

/// A reader that fails as soon as it is read or sought.
struct Broken;

impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("disk gone"))
    }
}

impl Seek for Broken {
    fn seek(&mut self, _: SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::other("disk gone"))
    }
}

#[test]
fn a_package_that_cannot_be_read_is_an_io_error_in_the_report() {
    let report = verify(Broken, &prepared("", ""));
    assert!(!report.ok);
    let failure = report.error.as_ref().unwrap();
    assert_eq!(failure.code, "io");
    assert!(
        failure
            .message
            .starts_with("the package could not be read: "),
        "{}",
        failure.message
    );
}

#[test]
fn bytes_that_are_no_workbook_are_diagnostics_not_an_error() {
    let report = verify_bytes(b"not a zip", &prepared("", ""));
    assert!(!report.ok);
    assert_eq!(report.error, None);
    assert_eq!(report.counts.error, 1);
}

#[test]
fn a_request_is_prepared_and_a_policy_is_checked_alone() {
    let request = prepare_request(br#"{"policy": {"failOn": "warn"}, "rules": {}}"#).unwrap();
    assert!(verify_bytes(b"not a zip", &request).fail_on == FailOn::Warn);
    let report = check_policy(Input::Bytes(br#"{"maxEntries": 0}"#));
    assert!(!report.ok);
    assert_eq!(
        faults(&report.diagnostics),
        ["policy_invalid maxEntries|out_of_range|1 to 1048576"]
    );
}

#[test]
fn the_callers_allow_of_a_function_admits_it_in_any_letter_case() {
    let denied = verified("formulas.xlsx", "", "");
    assert!(codes(&denied).contains(&"function_dangerous"), "{denied}");
    let allowed = verified(
        "formulas.xlsx",
        "",
        r#"{"functions": {"allow": ["webservice"]}}"#,
    );
    assert!(
        !codes(&allowed).contains(&"function_dangerous"),
        "{allowed}"
    );
}
