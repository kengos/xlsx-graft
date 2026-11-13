//! Unit tests for the command line: which arguments are accepted, and which
//! error each file that cannot be used gives.

use super::{StartError, start};
use std::ffi::OsString;
use xlsx_graft_host::{Input, Policy};

/// The `start` of `args`.
fn started(args: &[&str]) -> Result<crate::Context, StartError> {
    start(args.iter().map(OsString::from))
}

/// The message of the usage error `args` give.
fn usage(args: &[&str]) -> String {
    match started(args) {
        Err(StartError::Usage(reason)) => reason,
        other => panic!("expected a usage error, got {other:?}"),
    }
}

/// A file named `policy.json` that holds `text`, in a directory of its own.
fn policy_file(text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("policy.json");
    std::fs::write(&path, text).unwrap();
    let path = path.to_str().unwrap().to_owned();
    (dir, path)
}

#[test]
fn no_arguments_start_with_the_default_policy() {
    assert_eq!(started(&[]).unwrap().policy(), &Policy::default());
}

#[test]
fn a_policy_file_is_read_into_the_context() {
    let (_dir, path) = policy_file(r#"{"maxEntries": 5}"#);
    let expected = Policy::read(Input::Bytes(br#"{"maxEntries": 5}"#)).unwrap();
    assert_eq!(started(&["--policy", &path]).unwrap().policy(), &expected);
}

#[test]
fn an_argument_other_than_policy_and_its_file_is_a_usage_error() {
    assert_eq!(usage(&["--policy"]), "--policy needs a FILE");
    assert_eq!(
        usage(&["--policy", "a.json", "--policy", "b.json"]),
        "unexpected argument: --policy"
    );
    assert_eq!(
        usage(&["--policy=a.json"]),
        "unexpected argument: --policy=a.json"
    );
    assert_eq!(usage(&["--root", "."]), "unexpected argument: --root");
    assert_eq!(usage(&["a.json"]), "unexpected argument: a.json");
}

#[test]
fn a_usage_error_says_how_to_start_the_server() {
    let message = started(&["-x"]).unwrap_err().to_string();
    assert_eq!(
        message,
        "unexpected argument: -x\nusage: xlsx-graft-mcp [--policy FILE]"
    );
}

#[test]
fn a_policy_file_that_cannot_be_read_names_the_file_and_the_reason() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("none.json");
    let error = started(&["--policy", path.to_str().unwrap()]).unwrap_err();
    let StartError::PolicyUnreadable { path: named, .. } = &error else {
        panic!("expected PolicyUnreadable, got {error:?}");
    };
    assert_eq!(named, &path);
    assert!(error.to_string().starts_with(&format!(
        "the policy {} could not be read: ",
        path.display()
    )));
}

#[test]
fn a_policy_with_faults_holds_every_policy_invalid_record() {
    let (_dir, path) = policy_file(r#"{"maxEntries": 0, "nope": 1}"#);
    let error = started(&["--policy", &path]).unwrap_err();
    let StartError::PolicyInvalid { records, .. } = &error else {
        panic!("expected PolicyInvalid, got {error:?}");
    };
    assert_eq!(records.len(), 2);
    let text = error.to_string();
    assert!(text.starts_with(&format!("the policy {path} is not valid:\nerror\n")));
    assert!(text.contains("policy_invalid"));
}
