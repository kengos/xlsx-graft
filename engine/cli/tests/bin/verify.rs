//! `verify` on the workbooks Excel saved in `fixtures/`: the result in each
//! format, the exit status of each way it ends, and the help. The JSON
//! cases pin the contract: whole lines are compared.

use crate::{run, stderr, stdout};
use serde_json::Value;
use std::path::Path;

/// The path of the fixture `name`.
fn fixture(name: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    dir.join(name).to_str().unwrap().to_owned()
}

/// The result `output` printed, which is one line of JSON.
fn result(output: &std::process::Output) -> Value {
    assert_eq!(stdout(output).lines().count(), 1);
    serde_json::from_str(stdout(output)).unwrap()
}

fn codes(result: &Value) -> Vec<&str> {
    let diagnostics = result["diagnostics"].as_array().unwrap();
    (diagnostics.iter())
        .map(|record| record["code"].as_str().unwrap())
        .collect()
}

/// A policy file of its own in a directory of its own.
fn policy(text: &[u8]) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("policy.json");
    std::fs::write(&path, text).unwrap();
    let path = path.to_str().unwrap().to_owned();
    (dir, path)
}

#[test]
fn a_workbook_with_no_problem_is_ok_and_its_json_is_the_whole_contract() {
    let output = run(&["verify", &fixture("table.xlsx"), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        concat!(
            r#"{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},"#,
            r#""error":null,"diagnostics":[]}"#,
            "\n",
        )
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_workbook_the_check_refuses_is_not_ok_with_exit_1() {
    let output = run(&["--format=json", "verify", &fixture("macro.xlsm")]);
    assert_eq!(output.status.code(), Some(1));
    let result = result(&output);
    assert_eq!(result["ok"], false);
    assert_eq!(result["error"], Value::Null);
    assert_eq!(result["counts"]["error"], 2);
    assert_eq!(codes(&result), ["part_denied", "part_denied"]);
    assert_eq!(stderr(&output), "");
}

#[test]
fn text_groups_the_diagnostics_under_headings_with_a_fix_and_the_counts() {
    let output = run(&["verify", &fixture("macro.xlsm")]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.starts_with("error\n  part_denied /xl/"), "{text}");
    assert!(text.contains("\n    fix (rules): add macro to parts.allow\n"));
    assert!(text.ends_with("\n2 errors, 0 warnings, 0 notices — not ok\n"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn allow_part_lets_the_part_through() {
    let output = run(&["verify", &fixture("macro.xlsm"), "--allow-part", "macro"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).ends_with("0 errors, 0 warnings, 0 notices — ok\n"));
}

#[test]
fn fail_on_warn_makes_a_best_effort_warning_fail() {
    let book = fixture("dynamic-array.xlsx");
    let lenient = run(&[
        "verify",
        &book,
        "--best-effort",
        "excel 2016",
        "--format",
        "json",
    ]);
    assert_eq!(lenient.status.code(), Some(0));
    let lenient = result(&lenient);
    assert!(lenient["counts"]["warn"].as_u64().unwrap() > 0);
    let strict = run(&[
        "verify",
        &book,
        "--best-effort",
        "excel 2016",
        "--fail-on",
        "warn",
        "--format",
        "json",
    ]);
    assert_eq!(strict.status.code(), Some(1));
    let strict = result(&strict);
    assert_eq!(
        (strict["ok"].as_bool(), strict["failOn"].as_str()),
        (Some(false), Some("warn"))
    );
    let text = run(&[
        "verify",
        &book,
        "--best-effort",
        "excel 2016",
        "--fail-on",
        "warn",
    ]);
    assert!(stdout(&text).contains(" — not ok (failOn: warn)\n"));
}

#[test]
fn the_stricter_fail_on_of_the_policy_wins_over_the_callers() {
    let (_dir, file) = policy(br#"{"failOn": "warn"}"#);
    let output = run(&[
        "verify",
        &fixture("dynamic-array.xlsx"),
        "--policy",
        &file,
        "--best-effort",
        "excel 2016",
        "--fail-on",
        "error",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(result(&output)["failOn"], "warn");
}

#[test]
fn a_workbook_that_cannot_be_opened_is_the_results_error_with_exit_1() {
    let missing = fixture("none.xlsx");
    let output = run(&["verify", &missing, "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let result = result(&output);
    assert_eq!(result["ok"], false);
    assert_eq!(result["error"]["code"], "io");
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains("could not be opened")
    );
    assert_eq!(result["diagnostics"], Value::Array(vec![]));
    assert_eq!(stderr(&output), "");
    let text = run(&["verify", &missing]);
    assert_eq!(text.status.code(), Some(1));
    assert!(stdout(&text).starts_with("could not verify: "));
    assert_eq!(stderr(&text), "");
}

#[test]
fn a_policy_file_that_cannot_be_read_is_the_results_error_with_exit_1() {
    let missing = fixture("none.json");
    let output = run(&[
        "verify",
        &fixture("table.xlsx"),
        "--policy",
        &missing,
        "--fail-on",
        "warn",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let result = result(&output);
    assert_eq!(result["error"]["code"], "io");
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("the policy ")
    );
    assert_eq!(result["failOn"], "warn");
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_fault_of_the_policy_is_a_diagnostic_with_exit_2_and_the_workbook_is_not_opened() {
    let (_dir, file) = policy(br#"{"maxEntries": 0}"#);
    let output = run(&[
        "verify",
        &fixture("none.xlsx"),
        "--policy",
        &file,
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let result = result(&output);
    assert_eq!(
        (result["ok"].as_bool(), &result["error"]),
        (Some(false), &Value::Null)
    );
    assert_eq!(codes(&result), ["policy_invalid"]);
    assert_eq!(stderr(&output), "");
    let text = run(&["verify", &fixture("none.xlsx"), "--policy", &file]);
    assert_eq!(text.status.code(), Some(2));
    assert!(stdout(&text).starts_with("error\n  policy_invalid"));
}

#[test]
fn a_policy_file_larger_than_the_host_reads_is_too_large() {
    let (_dir, file) = policy(&vec![b' '; (1 << 20) + 1]);
    let output = run(&[
        "verify",
        &fixture("table.xlsx"),
        "--policy",
        &file,
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let result = result(&output);
    assert_eq!(codes(&result), ["policy_invalid"]);
    assert_eq!(result["diagnostics"][0]["args"]["reason"], "too_large");
}

#[test]
fn every_fault_of_the_rules_is_found_at_once_with_exit_2() {
    let output = run(&[
        "verify",
        &fixture("table.xlsx"),
        "--allow-part",
        "bogus",
        "--target",
        "nope",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let result = result(&output);
    let mut found = codes(&result);
    found.sort_unstable();
    assert_eq!(found, ["rules_invalid", "target_invalid"]);
}

#[test]
fn a_fail_on_clap_does_not_know_is_a_usage_error_without_a_result() {
    let output = run(&["verify", &fixture("table.xlsx"), "--fail-on", "info"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("possible values: error, warn"));
}

#[test]
fn the_help_says_it_prints_its_result_whatever_happened_and_how_it_exits() {
    let output = run(&["verify", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = stdout(&output);
    assert!(help.contains("Unlike sheets and read, verify prints its result whatever happened"));
    assert!(help.contains("Exit status: 0 when ok."));
    assert!(help.contains("--allow-part <KIND>"));
}
