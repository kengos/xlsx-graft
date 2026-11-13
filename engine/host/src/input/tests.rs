//! Unit tests for reading a setting's JSON under a size bound.

use super::{Input, MAX_SETTING_BYTES, read, read_setting};
use crate::support::{faults, json};
use crate::walk::{Reporter, Side};
use serde_json::Value;
use xlsx_graft_diagnostics::Diagnostics;

/// The value `input` is read as, and what was reported.
fn read_as(input: Input<'_>) -> (Option<Value>, Vec<String>) {
    let mut diagnostics = Diagnostics::new();
    let mut reporter = Reporter::new(Side::Policy, &mut diagnostics);
    let value = read(input, &mut reporter).map(|value| value.into_owned());
    (value, faults(&diagnostics.finish()))
}

#[test]
fn no_setting_is_an_empty_object() {
    let empty = Some(json("{}"));
    assert_eq!(read_as(Input::Absent), (empty.clone(), vec![]));
    assert_eq!(read_as(Input::Bytes(b"")), (empty.clone(), vec![]));
    // A file with a newline only, and `null`, are no setting either.
    assert_eq!(read_as(Input::Bytes(b" \n\t")), (empty.clone(), vec![]));
    assert_eq!(read_as(Input::Bytes(b"null\n")), (empty.clone(), vec![]));
    assert_eq!(read_as(Input::Value(&Value::Null)), (empty, vec![]));
}

#[test]
fn a_value_is_read_as_it_is() {
    let value = json(r#"{"a": 1}"#);
    assert_eq!(read_as(Input::Value(&value)), (Some(value.clone()), vec![]));
    assert_eq!(read_as(Input::Bytes(br#"{"a": 1}"#)), (Some(value), vec![]));
}

#[test]
fn the_most_bytes_are_read_and_one_more_is_a_fault() {
    let mut text = vec![b' '; MAX_SETTING_BYTES - 2];
    text.extend_from_slice(b"{}");
    assert_eq!(read_as(Input::Bytes(&text)), (Some(json("{}")), vec![]));
    text.push(b' ');
    let (value, said) = read_as(Input::Bytes(&text));
    assert_eq!(value, None);
    assert_eq!(
        said,
        [format!("policy_invalid |too_large|{MAX_SETTING_BYTES}")]
    );
}

#[test]
fn text_that_is_not_json_says_where_it_stops() {
    let (value, said) = read_as(Input::Bytes(b"{\n \"a\": }"));
    assert_eq!(value, None);
    assert_eq!(said, ["policy_invalid |malformed|line 2 column 7"]);
}

#[test]
fn json_nested_past_what_is_read_is_not_json() {
    let deep = "[".repeat(1_000);
    let (value, said) = read_as(Input::Bytes(deep.as_bytes()));
    assert_eq!(value, None);
    assert_eq!(said.len(), 1);
    assert!(said[0].starts_with("policy_invalid |malformed|line 1 column "));
}

#[test]
fn a_setting_file_is_read_one_byte_past_what_the_host_takes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("policy.json");
    std::fs::write(&path, vec![b' '; MAX_SETTING_BYTES + 10]).unwrap();
    assert_eq!(read_setting(&path).unwrap().len(), MAX_SETTING_BYTES + 1);
    std::fs::write(&path, b"{}").unwrap();
    assert_eq!(read_setting(&path).unwrap(), b"{}");
    assert!(read_setting(&dir.path().join("none.json")).is_err());
}
