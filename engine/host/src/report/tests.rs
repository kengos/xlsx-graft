//! Unit tests for the report: when it is `ok`, what it counts and the JSON
//! it is written as.

use super::{Counts, FailOn, Failure, Report};
use crate::support::json;
use xlsx_graft_diagnostics::{
    Diagnostic, Diagnostics, Echo, Limit, Location, TargetApp, TargetFallback,
};

fn warning() -> Diagnostic {
    Diagnostic::BestEffortFunctionUnsupported {
        name: Echo::new("XLOOKUP"),
        app: TargetApp::Excel,
        unsupported: Echo::new("2016"),
        unverified: Echo::new(""),
        requires: Echo::new(""),
        fallback: TargetFallback::NameError,
        count: 1,
    }
}

fn notice() -> Diagnostic {
    Diagnostic::FunctionReferenceUnchecked {
        name: Echo::new("OFFSET"),
        count: 1,
    }
}

fn error() -> Diagnostic {
    Diagnostic::PolicyInvalid {
        key: Echo::new(""),
        reason: xlsx_graft_diagnostics::SettingReason::NotObject,
        hint: Echo::new(""),
    }
}

fn found(diagnostics: &[Diagnostic], limit: u64) -> Diagnostics {
    let mut found = Diagnostics::with_limit(Limit::default_of(limit));
    for diagnostic in diagnostics {
        found.push(diagnostic.clone(), Location::default());
    }
    found
}

#[test]
fn a_report_is_ok_unless_an_error_or_a_strict_enough_warning_was_found() {
    let ok = |diagnostics: &[Diagnostic], fail_on, limit| {
        Report::finished(found(diagnostics, limit), fail_on, None).ok
    };
    assert!(ok(&[], FailOn::Error, 10));
    assert!(ok(&[notice()], FailOn::Warn, 10));
    assert!(ok(&[warning()], FailOn::Error, 10));
    assert!(!ok(&[warning()], FailOn::Warn, 10));
    assert!(!ok(&[error()], FailOn::Error, 10));
    // What was dropped past the limit counts as much as what was kept.
    assert!(!ok(&[error()], FailOn::Error, 0));
    assert!(!ok(&[warning()], FailOn::Warn, 0));
    assert!(ok(&[warning()], FailOn::Error, 0));
}

#[test]
fn a_report_that_holds_an_error_is_never_ok() {
    let report = Report::finished(
        Diagnostics::new(),
        FailOn::Error,
        Some(Failure::io("x".to_owned())),
    );
    assert!(!report.ok);
    assert_eq!(
        report.error,
        Some(Failure {
            code: "io",
            message: "x".to_owned()
        })
    );
}

#[test]
fn the_counts_are_those_of_the_diagnostics_listed() {
    let all = [error(), warning(), notice(), notice(), warning(), notice()];
    let report = Report::finished(found(&all, 4), FailOn::Error, None);
    // The notice that says what was dropped is listed and counted; the two
    // dropped (a warning and a notice) are not.
    assert_eq!(
        report.counts,
        Counts {
            error: 1,
            warn: 1,
            notice: 3
        }
    );
    assert_eq!(report.diagnostics.len(), 5);
}

#[test]
fn a_refused_report_is_not_ok_and_lists_the_faults_it_was_given() {
    let report = Report::refused(found(&[error(), error()], 10).finish(), FailOn::Warn);
    assert!(!report.ok);
    assert_eq!((report.fail_on, report.counts.error), (FailOn::Warn, 2));
    assert_eq!(report.error, None);
}

#[test]
fn the_failon_of_two_is_the_stricter_and_error_when_neither_says() {
    use FailOn::{Error, Warn};
    assert_eq!(FailOn::stricter(None, None), Error);
    assert_eq!(FailOn::stricter(Some(Error), None), Error);
    assert_eq!(FailOn::stricter(None, Some(Warn)), Warn);
    assert_eq!(FailOn::stricter(Some(Warn), Some(Error)), Warn);
    assert_eq!(FailOn::stricter(Some(Error), Some(Warn)), Warn);
}

#[test]
fn the_json_has_these_keys_in_this_order_and_a_failure_has_two() {
    let clean = Report::finished(Diagnostics::new(), FailOn::Error, None);
    assert_eq!(
        clean.to_json().unwrap(),
        r#"{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},"error":null,"diagnostics":[]}"#
    );
    let failed = Report::finished(
        Diagnostics::new(),
        FailOn::Warn,
        Some(Failure::io("gone".to_owned())),
    );
    assert_eq!(
        failed.to_json().unwrap(),
        r#"{"ok":false,"failOn":"warn","counts":{"error":0,"warn":0,"notice":0},"error":{"code":"io","message":"gone"},"diagnostics":[]}"#
    );
    let listed = Report::finished(found(&[error()], 10), FailOn::Error, None);
    let value = json(&listed.to_json().unwrap());
    assert_eq!(value["diagnostics"][0]["code"], "policy_invalid");
}
