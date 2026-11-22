//! `describe-function`: the table's answer for each state a name is in, in
//! each format, the exit status (0 for all three states), and the help. The
//! JSON is the host's, written in one line.

use crate::{run, stderr, stdout};
use xlsx_graft_host::describe_function;

#[test]
fn json_is_the_hosts_json_in_one_line_for_each_state_and_exits_0() {
    for name in ["SEQUENCE", "SUM", "NOPE"] {
        let output = run(&["describe-function", name, "--format", "json"]);
        assert_eq!(output.status.code(), Some(0), "{name}");
        assert_eq!(stderr(&output), "", "{name}");
        let expected = format!("{}\n", describe_function(name).to_json().unwrap());
        assert_eq!(stdout(&output), expected, "{name}");
    }
    let output = run(&["describe-function", "NOPE", "--format", "json"]);
    assert_eq!(
        stdout(&output),
        "{\"name\":\"NOPE\",\"known\":\"unknown\"}\n"
    );
}

#[test]
fn text_names_the_state_of_each_name_and_exits_0() {
    let versioned = run(&["describe-function", "sequence"]);
    assert_eq!(versioned.status.code(), Some(0));
    let text = stdout(&versioned);
    assert_eq!(text.lines().next(), Some("SEQUENCE — versioned"));
    assert!(text.contains("  file name: _xlfn.SEQUENCE\n"), "{text}");
    assert!(text.contains("  requires dynamic_array\n"), "{text}");
    assert_eq!(stderr(&versioned), "");

    let baseline = run(&["describe-function", "SUM"]);
    assert_eq!(baseline.status.code(), Some(0));
    assert_eq!(stdout(&baseline).lines().next(), Some("SUM — baseline"));
    assert_eq!(stderr(&baseline), "");

    let unknown = run(&["describe-function", "NOPE"]);
    assert_eq!(unknown.status.code(), Some(0));
    assert_eq!(
        stdout(&unknown),
        "NOPE — unknown: the table does not list it; verify denies it\n"
    );
    assert_eq!(stderr(&unknown), "");
}

#[test]
fn a_name_is_looked_up_without_its_prefixes() {
    let output = run(&["describe-function", "_xlfn._xlws.sort", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).starts_with(r#"{"name":"SORT","known":"versioned""#));
}

#[test]
fn a_call_without_a_name_is_a_usage_error() {
    let output = run(&["describe-function"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("<NAME>"), "{}", stderr(&output));
}

#[test]
fn the_help_of_describe_function_gives_the_json_contract_and_the_exit_status() {
    let output = run(&["describe-function", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = stdout(&output);
    assert!(help.contains("Usage: xlsx-graft describe-function [OPTIONS] <NAME>"));
    assert!(help.contains(r#""known": "versioned""#));
    assert!(help.contains("Exit status: 0"));
}
