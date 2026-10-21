//! The command line itself: no arguments, and arguments clap refuses.

use crate::{run, stderr, stdout};

#[test]
fn no_arguments_prints_the_help_and_succeeds() {
    let output = run(&[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("Usage: xlsx-graft"));
    assert!(stdout(&output).contains("--format <FORMAT>"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn format_json_alone_still_prints_the_help_as_text() {
    let output = run(&["--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), stdout(&run(&[])));
    assert!(serde_json::from_str::<serde_json::Value>(stdout(&output)).is_err());
}

#[test]
fn an_unknown_format_is_refused_with_exit_2_and_names_the_choices() {
    let output = run(&["--version", "--format", "xml"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("invalid value 'xml'"));
    assert!(stderr(&output).contains("[possible values: text, json]"));
}

#[test]
fn an_unknown_flag_is_refused_with_exit_2() {
    let output = run(&["--no-such-flag"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("unexpected argument '--no-such-flag'"));
}
