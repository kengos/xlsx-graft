//! Unit tests for `verify`: the rules the arguments make and the exit a
//! result makes.

use super::{VerifyArgs, fail_on, outcome, rules};
use crate::{Cli, Command, Outcome};
use serde_json::json;
use xlsx_graft_host::{FailOn, Report};

fn args(arguments: &[&str]) -> VerifyArgs {
    let cli = Cli::try_parse_args(
        ["xlsx-graft", "verify"]
            .into_iter()
            .chain(arguments.iter().copied())
            .chain(["book.xlsx"]),
    )
    .unwrap();
    match cli.command {
        Some(Command::Verify(args)) => args,
        _ => unreachable!("a verify command line"),
    }
}

#[test]
fn rules_hold_only_the_keys_given() {
    assert_eq!(rules(&args(&[])), json!({}));
    let all = args(&[
        "--target",
        "excel >= 2016",
        "--best-effort",
        "excel_mac >= 2019",
        "--allow-function",
        "HYPERLINK",
        "--allow-part",
        "macro",
        "--fail-on",
        "warn",
    ]);
    assert_eq!(
        rules(&all),
        json!({
            "targets": ["excel >= 2016"],
            "bestEffort": ["excel_mac >= 2019"],
            "functions": {"allow": ["HYPERLINK"]},
            "parts": {"allow": ["macro"]},
            "failOn": "warn",
        })
    );
}

#[test]
fn the_fail_on_of_an_unread_policy_is_the_callers() {
    assert_eq!(fail_on(&args(&[])), FailOn::Error);
    assert_eq!(fail_on(&args(&["--fail-on", "error"])), FailOn::Error);
    assert_eq!(fail_on(&args(&["--fail-on", "warn"])), FailOn::Warn);
}

#[test]
fn the_exit_is_1_for_a_file_not_read_2_for_a_fault_of_the_policy_or_the_rules() {
    let io = Report::io_failure("m".to_owned(), FailOn::Error);
    assert_eq!(outcome(&io, true), Outcome::Failure);
    let not_ok = Report::io_failure("m".to_owned(), FailOn::Error);
    let refused = Report {
        error: None,
        ..not_ok
    };
    assert_eq!(outcome(&refused, true), Outcome::Invalid);
    assert_eq!(outcome(&refused, false), Outcome::Failure);
    let ok = Report {
        ok: true,
        ..refused
    };
    assert_eq!(outcome(&ok, false), Outcome::Success);
}
