//! Unit tests for the text of a result: the headings, a diagnostic's line
//! and fix, and the last line.

use super::{DiagnosticsText, ReportText};
use crate::{FailOn, Report};
use xlsx_graft_diagnostics::{
    Diagnostic, Diagnostics, Echo, Limit, Location, PartKind, Record, TargetApp, TargetFallback,
};

fn push(diagnostic: Diagnostic, location: Location) -> Vec<Record> {
    let mut found = Diagnostics::with_limit(Limit::default_of(10));
    found.push(diagnostic, location);
    found.finish()
}

fn ratio() -> Diagnostic {
    Diagnostic::ZipRatioExceeded {
        size: 1_001,
        compressed_size: 1,
        limit: Limit::default_of(1_000),
    }
}

#[test]
fn a_result_without_a_problem_is_one_line() {
    let report = Report::finished(Diagnostics::new(), FailOn::Error, None);
    assert_eq!(
        ReportText(&report).to_string(),
        "0 errors, 0 warnings, 0 notices — ok\n"
    );
}

#[test]
fn the_headings_come_in_severity_order_and_only_when_they_have_a_line() {
    let mut found = Diagnostics::new();
    found.push(
        Diagnostic::FunctionReferenceUnchecked {
            name: Echo::new("OFFSET"),
            count: 1,
        },
        Location::default(),
    );
    found.push(
        Diagnostic::BestEffortFunctionUnsupported {
            name: Echo::new("XLOOKUP"),
            app: TargetApp::Excel,
            unsupported: Echo::new("2016"),
            unverified: Echo::new(""),
            requires: Echo::new(""),
            fallback: TargetFallback::NameError,
            count: 1,
        },
        Location::default(),
    );
    found.push(ratio(), Location::default());
    let text = DiagnosticsText(&found.finish()).to_string();
    let headings: Vec<&str> = text.lines().filter(|l| !l.starts_with(' ')).collect();
    assert_eq!(headings, ["error", "warn", "notice"]);
    let only_notice = push(
        Diagnostic::FunctionReferenceUnchecked {
            name: Echo::new("OFFSET"),
            count: 1,
        },
        Location::default(),
    );
    assert!(
        DiagnosticsText(&only_notice)
            .to_string()
            .starts_with("notice\n")
    );
}

#[test]
fn a_line_names_its_code_place_and_message_and_the_fix_follows() {
    let denied = Diagnostic::PartDenied {
        kind: PartKind::Macro,
    };
    let location = Location::default().with_part("/xl/vbaProject.bin");
    let text = DiagnosticsText(&push(denied.clone(), location)).to_string();
    let expected = format!(
        "error\n  part_denied /xl/vbaProject.bin — {}\n    fix (rules): add macro to parts.allow\n",
        denied.message()
    );
    assert_eq!(text, expected);
}

#[test]
fn the_fix_of_a_limit_is_the_policy_line_and_of_the_workbook_its_text() {
    let text = DiagnosticsText(&push(ratio(), Location::default())).to_string();
    assert!(text.ends_with("    fix (policy): maxCompressionRatio: 1001\n"));
    let empty = Diagnostic::ZipRatioExceeded {
        size: 1_001,
        compressed_size: 0,
        limit: Limit::default_of(1_000),
    };
    let text = DiagnosticsText(&push(empty, Location::default())).to_string();
    assert!(text.ends_with("    fix: save the workbook again from Excel\n"));
}

#[test]
fn a_diagnostic_without_a_fix_has_no_fix_line() {
    let notice = Diagnostic::FunctionReferenceUnchecked {
        name: Echo::new("OFFSET"),
        count: 1,
    };
    let text = DiagnosticsText(&push(notice, Location::default())).to_string();
    assert_eq!(text.lines().count(), 2);
    assert!(!text.contains("fix"));
}

#[test]
fn the_place_is_the_part_or_the_entry_or_the_rels_part_then_the_place_and_the_name() {
    let line = |location: Location| {
        let text = DiagnosticsText(&push(ratio(), location)).to_string();
        let line = text.lines().nth(1).unwrap().to_owned();
        line.split(" — ").next().unwrap().to_owned()
    };
    assert_eq!(line(Location::default()), "  zip_ratio_exceeded");
    assert_eq!(
        line(Location::default().with_entry("a.xml")),
        "  zip_ratio_exceeded a.xml"
    );
    assert_eq!(
        line(Location::default().with_rels_part("/_rels/.rels")),
        "  zip_ratio_exceeded /_rels/.rels"
    );
    let reached = Location::default()
        .with_part("/xl/workbook.xml")
        .with_entry("ignored")
        .with_place("B2")
        .with_defined_name("MyFn");
    assert_eq!(
        line(reached),
        "  zip_ratio_exceeded /xl/workbook.xml B2 via MyFn"
    );
}

#[test]
fn an_io_failure_is_said_before_the_last_line_and_a_strict_fail_on_is_named() {
    let report = Report::io_failure("book.xlsx could not be opened".to_owned(), FailOn::Warn);
    assert_eq!(
        ReportText(&report).to_string(),
        "could not verify: book.xlsx could not be opened\n0 errors, 0 warnings, 0 notices — not ok (failOn: warn)\n"
    );
}
