//! `--version` in each format. The JSON case pins the contract: the whole
//! object is compared, so a renamed, added or retyped key fails here.

use crate::{run, stderr, stdout};
use serde_json::{Value, json};

#[test]
fn version_prints_one_text_line() {
    let output = run(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "xlsx-graft 0.0.0\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn version_as_json_is_exactly_the_version_object() {
    let output = run(&["--version", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_str(stdout(&output)).unwrap();
    assert_eq!(value, json!({ "version": "0.0.0" }));
    assert_eq!(stdout(&output).lines().count(), 1);
    assert_eq!(stderr(&output), "");
}

#[test]
fn format_may_come_before_the_flag_it_formats() {
    let output = run(&["--format=json", "-V"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "{\"version\":\"0.0.0\"}\n");
}
