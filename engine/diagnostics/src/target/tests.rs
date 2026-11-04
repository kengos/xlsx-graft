//! Unit tests for a target code's message: the versions that lack it, those
//! not confirmed, the features it needs and what a version shows instead.

use crate::{DecidedBy, Diagnostic, Echo, Fix, Severity, TargetApp, TargetFallback};

/// `target_function_unsupported` of XLOOKUP in Excel, found 3 times.
fn unmet(
    unsupported: &str,
    unverified: &str,
    requires: &str,
    fallback: TargetFallback,
) -> Diagnostic {
    Diagnostic::TargetFunctionUnsupported {
        name: Echo::new("XLOOKUP"),
        app: TargetApp::Excel,
        unsupported: Echo::new(unsupported),
        unverified: Echo::new(unverified),
        requires: Echo::new(requires),
        fallback,
        count: 3,
    }
}

#[test]
fn the_message_names_the_versions_that_lack_it_and_what_they_show() {
    assert_eq!(
        unmet("2016, 2019", "", "", TargetFallback::NameError).message(),
        "a formula calls XLOOKUP; targets lists excel versions that lack it (2016, 2019); where it is missing, if it is, the formula evaluates to #NAME?; 3 time(s), the first here"
    );
}

#[test]
fn the_message_says_which_versions_are_not_confirmed() {
    assert_eq!(
        unmet("2007", "2010, 2013", "", TargetFallback::Unknown).message(),
        "a formula calls XLOOKUP; targets lists excel versions that lack it (2007) and that are not confirmed to have it (2010, 2013); 3 time(s), the first here"
    );
    assert_eq!(
        unmet("", "", "", TargetFallback::Unknown).message(),
        "a formula calls XLOOKUP; targets lists excel versions that may lack it; 3 time(s), the first here"
    );
    assert_eq!(
        unmet("", "2010", "", TargetFallback::Unknown).message(),
        "a formula calls XLOOKUP; targets lists excel versions that are not confirmed to have it (2010); 3 time(s), the first here"
    );
}

#[test]
fn the_message_names_the_features_it_needs_as_well() {
    assert_eq!(
        unmet("", "2019", "dynamic_array", TargetFallback::Unknown).message(),
        "a formula calls XLOOKUP; targets lists excel versions that are not confirmed to have it (2019); in some of them, it needs dynamic_array as well, which is missing or not confirmed there; 3 time(s), the first here"
    );
}

#[test]
fn every_fallback_reads_as_what_a_version_shows() {
    let shows = |fallback| {
        let message = unmet("2016", "", "", fallback).message();
        let (_, shows) = message
            .split_once("where it is missing, if it is, ")
            .unwrap();
        shows
            .trim_end_matches("; 3 time(s), the first here")
            .to_owned()
    };
    let expected = [
        (TargetFallback::Ignored, "it is skipped and not shown"),
        (
            TargetFallback::ValueOnly,
            "only the value saved with the cell is shown",
        ),
        (TargetFallback::NameError, "the formula evaluates to #NAME?"),
        (TargetFallback::ValueError, "the cell shows #VALUE!"),
        (
            TargetFallback::LegacyArray,
            "it is a legacy array formula of a fixed size",
        ),
        (TargetFallback::AsNote, "it shows as a legacy note"),
        (TargetFallback::PlaceholderShape, "a shape stands in for it"),
        (TargetFallback::Degraded, "only part of it works"),
        (
            TargetFallback::RepairPrompt,
            "the application offers to repair the workbook",
        ),
        (TargetFallback::Unopenable, "the workbook does not open"),
    ];
    for (fallback, words) in expected {
        assert_eq!(shows(fallback), words);
    }
}

#[test]
fn targets_refuses_and_best_effort_warns_each_with_a_fix() {
    let target = unmet("2016", "", "", TargetFallback::NameError);
    assert_eq!(target.severity(), Severity::Error);
    assert_eq!(target.decided_by(), Some(DecidedBy::Template {}));
    assert_eq!(
        target.fix(),
        Some(Fix::workbook(
            "call a function other than XLOOKUP so the workbook works in excel"
        ))
    );
    let best_effort = Diagnostic::BestEffortFunctionUnsupported {
        name: Echo::new("FILTER"),
        app: TargetApp::LibreOffice,
        unsupported: Echo::new("24.2"),
        unverified: Echo::new(""),
        requires: Echo::new(""),
        fallback: TargetFallback::Unknown,
        count: 1,
    };
    assert_eq!(best_effort.severity(), Severity::Warn);
    assert_eq!(best_effort.decided_by(), Some(DecidedBy::Template {}));
    assert_eq!(
        best_effort.message(),
        "a formula calls FILTER; bestEffort lists libreoffice versions that lack it (24.2); 1 time(s), the first here"
    );
    assert_eq!(
        best_effort.fix(),
        Some(Fix::workbook(
            "call a function other than FILTER so the workbook works in libreoffice"
        ))
    );
}

#[test]
fn the_message_names_each_application_as_the_template_does() {
    let apps = [
        (TargetApp::Excel, "excel"),
        (TargetApp::ExcelMac, "excel-mac"),
        (TargetApp::LibreOffice, "libreoffice"),
        (TargetApp::GoogleSheets, "google-sheets"),
        (TargetApp::Numbers, "numbers"),
        (TargetApp::OnlyOffice, "onlyoffice"),
    ];
    for (app, spelled) in apps {
        let diagnostic = Diagnostic::BestEffortFunctionUnsupported {
            name: Echo::new("XLOOKUP"),
            app,
            unsupported: Echo::new("1"),
            unverified: Echo::new(""),
            requires: Echo::new(""),
            fallback: TargetFallback::Unknown,
            count: 1,
        };
        assert!(
            diagnostic
                .message()
                .contains(&format!("lists {spelled} versions that lack it (1)")),
            "{app}"
        );
    }
}
