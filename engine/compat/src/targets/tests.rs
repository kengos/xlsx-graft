//! Unit tests for reading `targets:` and `bestEffort:`: every form of an
//! entry, every reason one names no versions, and the overlap of the lists.

use super::Targets;
use super::versions::Versions;
use crate::{App, Excel, ExcelMac, GoogleSheets, LibreOffice, Numbers, OnlyOffice, Version};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, TargetList, TargetReason};

/// A problem as `(list, index, entry, reason, names)`.
type Problem = (TargetList, u64, String, TargetReason, String);

/// Reads the two lists, returning the targets and the problems.
fn parse(targets: &[&str], best_effort: &[&str]) -> (Option<Targets>, Vec<Problem>) {
    let mut diagnostics = Diagnostics::new();
    let read = Targets::parse(targets, best_effort, &mut diagnostics);
    let problems = diagnostics
        .finish()
        .into_iter()
        .map(|record| match record.diagnostic {
            Diagnostic::TargetInvalid {
                list,
                index,
                entry,
                reason,
                names,
            } => (list, index, entry.to_string(), reason, names.to_string()),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    (read, problems)
}

/// Reads the two lists, which hold no problem.
fn read(targets: &[&str], best_effort: &[&str]) -> Targets {
    let (read, problems) = parse(targets, best_effort);
    assert_eq!(problems, [], "{targets:?} {best_effort:?}");
    read.unwrap()
}

/// The versions as written.
fn names<V: Version>(versions: impl Iterator<Item = V>) -> Vec<&'static str> {
    versions.map(Version::as_str).collect()
}

/// The versions `V` the one `targets:` entry `entry` holds, as written.
fn required<V: Version>(entry: &str) -> Vec<&'static str> {
    let targets = read(&[entry], &[]);
    names(targets.required::<V>())
}

#[test]
fn an_operator_holds_the_versions_on_its_side_of_the_version() {
    assert_eq!(
        required::<Excel>("excel >= 2016"),
        ["2016", "2019", "2021", "2024", "365"]
    );
    assert_eq!(
        required::<Excel>("excel > 2016"),
        ["2019", "2021", "2024", "365"]
    );
    assert_eq!(
        required::<Excel>("excel <= 2016"),
        ["2007", "2010", "2013", "2016"]
    );
    assert_eq!(required::<Excel>("excel < 2016"), ["2007", "2010", "2013"]);
    assert_eq!(required::<Excel>("excel 365"), ["365"]);
}

#[test]
fn white_space_around_the_operator_and_the_entry_is_optional() {
    assert_eq!(required::<Excel>("excel>=2024"), ["2024", "365"]);
    assert_eq!(required::<Excel>("  excel  <2010 "), ["2007"]);
}

#[test]
fn google_sheets_is_named_alone_and_holds_its_only_version() {
    assert_eq!(required::<GoogleSheets>("google-sheets"), ["current"]);
}

#[test]
fn each_application_keeps_its_own_versions() {
    let entries = [
        "excel 2019",
        "excel-mac >= 2024",
        "libreoffice > 26.2",
        "google-sheets",
        "numbers 14.4",
        "onlyoffice <= 8.1",
    ];
    let targets = read(&entries, &[]);
    assert_eq!(names(targets.required::<Excel>()), ["2019"]);
    assert_eq!(names(targets.required::<ExcelMac>()), ["2024", "365"]);
    assert_eq!(names(targets.required::<LibreOffice>()), ["26.8"]);
    assert_eq!(names(targets.required::<GoogleSheets>()), ["current"]);
    assert_eq!(names(targets.required::<Numbers>()), ["14.4"]);
    assert_eq!(names(targets.required::<OnlyOffice>()), ["8.0", "8.1"]);
    assert_eq!(targets.best_effort::<Excel>().count(), 0);
}

#[test]
fn entries_for_one_application_add_up() {
    let targets = read(&["excel 2010", "excel >= 2024", "excel 365"], &[]);
    let versions: Vec<_> = targets.required::<Excel>().collect();
    assert_eq!(versions, [Excel::V2010, Excel::V2024, Excel::V365]);
}

#[test]
fn best_effort_is_read_apart_from_targets() {
    let targets = read(
        &["excel >= 2016"],
        &[
            "excel < 2016",
            "excel-mac 365",
            "libreoffice >= 24.8",
            "google-sheets",
            "numbers 14.4",
            "onlyoffice 9.3",
        ],
    );
    let best_effort: Vec<_> = targets.best_effort::<Excel>().collect();
    assert_eq!(best_effort, [Excel::V2007, Excel::V2010, Excel::V2013]);
    assert_eq!(targets.required::<Excel>().count(), 5);
    assert_eq!(targets.required::<LibreOffice>().count(), 0);
    assert_eq!(targets.best_effort::<LibreOffice>().count(), 5);
    assert_eq!(targets.best_effort::<GoogleSheets>().count(), 1);
}

#[test]
fn an_empty_template_checks_nothing() {
    assert!(Targets::default().is_empty());
    assert!(read(&[], &[]).is_empty());
    assert!(!read(&[], &["numbers 14.4"]).is_empty());
}

const EXCEL: &str = "2007, 2010, 2013, 2016, 2019, 2021, 2024, 365";

#[test]
fn every_entry_that_names_no_versions_is_reported_in_order() {
    let entries = [
        "",
        "excel",
        ">= 2016",
        "excel >=",
        "excel 2016 2019",
        "excel = 2016",
        "excel >= >2016",
        "Excel 2016",
        "lotus 1-2-3",
        "excel 2020",
        "excel-mac 2013",
        "google-sheets current",
        "google-sheets >= current",
        "excel < 2007",
        "libreoffice > 26.8",
        "excel 365",
    ];
    let (targets, problems) = parse(&entries, &[]);
    let apps = "excel, excel-mac, libreoffice, google-sheets, numbers, onlyoffice";
    let libreoffice = "24.2, 24.8, 25.2, 25.8, 26.2, 26.8";
    let expected: Vec<(u64, TargetReason, &str)> = vec![
        (0, TargetReason::Malformed, ""),
        (1, TargetReason::VersionRequired, EXCEL),
        (2, TargetReason::Malformed, ""),
        (3, TargetReason::Malformed, ""),
        (4, TargetReason::Malformed, ""),
        (5, TargetReason::Malformed, ""),
        (6, TargetReason::Malformed, ""),
        (7, TargetReason::UnknownApp, apps),
        (8, TargetReason::UnknownApp, apps),
        (9, TargetReason::UnknownVersion, EXCEL),
        (
            10,
            TargetReason::UnknownVersion,
            "2016, 2019, 2021, 2024, 365",
        ),
        (11, TargetReason::VersionNotAllowed, "google-sheets"),
        (12, TargetReason::VersionNotAllowed, "google-sheets"),
        (13, TargetReason::Empty, EXCEL),
        (14, TargetReason::Empty, libreoffice),
    ];
    let found: Vec<(u64, TargetReason, &str)> = problems
        .iter()
        .map(|(list, index, entry, reason, names)| {
            assert_eq!(*list, TargetList::Targets);
            assert_eq!(entry, entries[usize::try_from(*index).unwrap()]);
            (*index, *reason, names.as_str())
        })
        .collect();
    assert_eq!(found, expected);
    assert_eq!(targets, None);
}

#[test]
fn a_best_effort_entry_reports_the_versions_targets_holds_as_well_in_list_order() {
    let (targets, problems) = parse(
        &["excel >= 2016", "google-sheets"],
        &[
            "excel < 2016",
            "excel 365",
            "excel >= 2013",
            "google-sheets",
            "excel 2020",
        ],
    );
    let found: Vec<(u64, TargetReason, &str)> = problems
        .iter()
        .map(|(list, index, _, reason, names)| {
            assert_eq!(*list, TargetList::BestEffort);
            (*index, *reason, names.as_str())
        })
        .collect();
    assert_eq!(
        found,
        [
            (1, TargetReason::Overlap, "365"),
            (2, TargetReason::Overlap, "2016, 2019, 2021, 2024, 365"),
            (3, TargetReason::Overlap, "google-sheets"),
            (4, TargetReason::UnknownVersion, EXCEL),
        ]
    );
    assert_eq!(targets, None);
}

#[test]
fn the_problems_of_targets_come_before_those_of_best_effort() {
    let (targets, problems) = parse(&["excel", "numbers 14.4"], &["numbers >= 14.0", "excel"]);
    let found: Vec<(TargetList, u64, TargetReason)> = problems
        .iter()
        .map(|(list, index, _, reason, _)| (*list, *index, *reason))
        .collect();
    assert_eq!(
        found,
        [
            (TargetList::Targets, 0, TargetReason::VersionRequired),
            (TargetList::BestEffort, 0, TargetReason::Overlap),
            (TargetList::BestEffort, 1, TargetReason::VersionRequired),
        ]
    );
    assert_eq!(targets, None);
}

#[test]
fn every_application_has_at_most_32_versions_so_each_fits_a_bit() {
    for app in App::ALL {
        assert!(app.version_names().len() <= 32, "{app:?}");
    }
}

#[test]
fn a_set_holds_no_position_past_the_32nd() {
    let set = Versions::range(30, 40);
    assert_eq!(set.names(&[""; 40]).split(", ").count(), 2);
    assert!(Versions::range(32, 40).is_empty());
}

#[test]
fn an_overlap_is_found_against_every_targets_entry_added_up() {
    let (targets, problems) = parse(&["excel 2010", "excel 365"], &["excel >= 2010"]);
    let names: Vec<&str> = problems.iter().map(|problem| problem.4.as_str()).collect();
    assert_eq!(names, ["2010, 365"]);
    assert_eq!(targets, None);
}
