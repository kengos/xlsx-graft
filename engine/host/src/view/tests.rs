//! Unit tests for the view: when it is `ok` and the JSON it is written as.

use super::View;
use crate::report::Failure;
use crate::support::json;
use serde::Serialize;
use xlsx_graft_diagnostics::{
    Diagnostic, Diagnostics, Echo, Limit, Location, SettingReason, TargetApp, TargetFallback,
};

/// What a view shows, in these tests.
#[derive(Serialize)]
struct Shown {
    shape: u32,
    again: bool,
}

/// A view's content.
fn shown() -> Shown {
    Shown {
        shape: 3,
        again: true,
    }
}

/// A diagnostic that is an error.
fn error() -> Diagnostic {
    Diagnostic::PolicyInvalid {
        key: Echo::new(""),
        reason: SettingReason::NotObject,
        hint: Echo::new(""),
    }
}

/// A diagnostic that is a warning.
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

/// A collector that listed `diagnostic` once, listing `limit` diagnostics at
/// most.
fn found(diagnostic: &Diagnostic, limit: u64) -> Diagnostics {
    let mut found = Diagnostics::with_limit(Limit::default_of(limit));
    found.push(diagnostic.clone(), Location::default());
    found
}

#[test]
fn a_view_is_ok_unless_it_has_an_error_or_an_error_diagnostic() {
    let clean = View::finished(shown(), Diagnostics::new(), None);
    assert!(clean.ok);
    // A warning does not make a view not ok: a view has no `failOn`.
    let warned = View::finished(shown(), found(&warning(), 10), None);
    assert!(warned.ok);
    assert_eq!(warned.diagnostics.len(), 1);
    let errored = View::finished(shown(), found(&error(), 10), None);
    assert!(!errored.ok);
    let failure = Failure::new("io", "gone".to_owned());
    let failed = View::finished(shown(), Diagnostics::new(), Some(failure.clone()));
    assert!(!failed.ok);
    assert_eq!(failed.error, Some(failure));
}

#[test]
fn an_io_failure_is_not_ok_has_no_diagnostic_and_the_default_content() {
    #[derive(Default, Serialize)]
    struct Empty {
        shape: u32,
    }
    let view = View::<Empty>::io_failure("gone".to_owned());
    assert!(!view.ok);
    assert!(view.diagnostics.is_empty());
    assert_eq!(view.error, Some(Failure::new("io", "gone".to_owned())));
    assert_eq!(
        view.to_json().unwrap(),
        r#"{"ok":false,"error":{"code":"io","message":"gone"},"diagnostics":[],"shape":0}"#
    );
}

#[test]
fn an_error_diagnostic_not_listed_past_the_limit_still_makes_the_view_not_ok() {
    let unlisted = View::finished(shown(), found(&error(), 0), None);
    assert!(!unlisted.ok);
    // Only the notice that says how many were not listed is listed.
    assert_eq!(unlisted.diagnostics.len(), 1);
    assert_eq!(
        unlisted.diagnostics[0].diagnostic.code().as_str(),
        "diagnostics_truncated"
    );
}

#[test]
fn the_json_has_ok_error_and_diagnostics_and_then_the_keys_of_the_content() {
    let view = View::finished(shown(), Diagnostics::new(), None);
    assert_eq!(
        view.to_json().unwrap(),
        r#"{"ok":true,"error":null,"diagnostics":[],"shape":3,"again":true}"#
    );
    let failed = View::finished(
        shown(),
        Diagnostics::new(),
        Some(Failure::new("io", "gone".to_owned())),
    );
    let value = json(&failed.to_json().unwrap());
    assert_eq!(value["error"], json(r#"{"code":"io","message":"gone"}"#));
    assert_eq!(
        (value["ok"].as_bool(), value["shape"].as_u64()),
        (Some(false), Some(3))
    );
}
