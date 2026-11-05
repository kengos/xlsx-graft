//! The functions a formula calls, checked against the targets: one
//! diagnostic per part, defined name reached through, function,
//! application and list, through the same names as the other findings.

use super::{SHEET1, at, cell, judge_against};
use crate::formulas::Place;
use crate::{Names, Rules, Targets};
use xlsx_graft_diagnostics::{
    Code, Diagnostic, Diagnostics, Echo, Location, TargetApp, TargetFallback,
};

/// The targets of the two lists.
fn targets(required: &[&str], best_effort: &[&str]) -> Targets {
    let mut diagnostics = Diagnostics::new();
    Targets::parse(required, best_effort, &mut diagnostics).unwrap()
}

/// The target diagnostics judging each formula at its place, with
/// `names`, under `excel >= 2019` and best effort `libreoffice 24.8`,
/// reports.
fn unmet_at(names: &Names, formulas: &[(Place<'_>, &str)]) -> Vec<(Diagnostic, Location)> {
    let targets = targets(&["excel >= 2019"], &["libreoffice 24.8"]);
    let found = judge_against(&Rules::default(), &targets, names, formulas);
    let codes = [
        Code::TargetFunctionUnsupported,
        Code::BestEffortFunctionUnsupported,
    ];
    (found.into_iter())
        .filter(|(diagnostic, _)| codes.contains(&diagnostic.code()))
        .collect()
}

/// [`unmet_at`] each formula in the cell it is paired with on the first
/// sheet, with no names.
fn unmet(formulas: &[(&str, &str)]) -> Vec<(Diagnostic, Location)> {
    let placed: Vec<(Place<'_>, &str)> = (formulas.iter())
        .map(|&(at, formula)| (cell(SHEET1, at, 0), formula))
        .collect();
    unmet_at(&Names::default(), &placed)
}

/// `target_function_unsupported` of XLOOKUP in Excel 2019, found `count`
/// times.
fn xlookup(count: u64) -> Diagnostic {
    Diagnostic::TargetFunctionUnsupported {
        name: Echo::new("XLOOKUP"),
        app: TargetApp::Excel,
        unsupported: Echo::new("2019"),
        unverified: Echo::new(""),
        requires: Echo::new(""),
        fallback: TargetFallback::NameError,
        count,
    }
}

#[test]
fn a_function_is_one_diagnostic_per_part_at_the_first_place() {
    let first = "_xlfn.XLOOKUP(1,A1:A3,B1:B3)";
    let other = "/xl/worksheets/sheet2.xml";
    let found = unmet_at(
        &Names::default(),
        &[
            (cell(SHEET1, "B2", 0), "SUM(1)"),
            (cell(SHEET1, "B3", 0), first),
            (
                cell(SHEET1, "B4", 0),
                "xlookup(2,A1:A3,B1:B3)*_xlfn.XLOOKUP(3,A1,B1)",
            ),
            (cell(other, "C1", 1), first),
        ],
    );
    assert_eq!(
        found,
        [
            (xlookup(3), at("B3")),
            (
                xlookup(1),
                Location::default().with_part(other).with_place("C1")
            ),
        ]
    );
}

#[test]
fn each_list_and_application_that_lacks_it_is_its_own_diagnostic() {
    let found = unmet(&[("A1", "_xlfn._xlws.FILTER(A1:A3,A1:A3>1)")]);
    let filter = [
        Diagnostic::TargetFunctionUnsupported {
            name: Echo::new("FILTER"),
            app: TargetApp::Excel,
            unsupported: Echo::new("2019"),
            unverified: Echo::new(""),
            requires: Echo::new(""),
            fallback: TargetFallback::NameError,
            count: 1,
        },
        Diagnostic::BestEffortFunctionUnsupported {
            name: Echo::new("FILTER"),
            app: TargetApp::LibreOffice,
            unsupported: Echo::new(""),
            unverified: Echo::new("24.8"),
            requires: Echo::new("dynamic_array"),
            fallback: TargetFallback::Unknown,
            count: 1,
        },
    ];
    assert_eq!(found, filter.map(|diagnostic| (diagnostic, at("A1"))));
}

#[test]
fn a_defined_name_is_checked_by_the_functions_of_its_formula() {
    let names = Names::default().defined_name(
        "Pick",
        None,
        "_xlfn.LAMBDA(_xlpm.x,_xlfn.XLOOKUP(_xlpm.x,A1:A3,B1:B3))",
    );
    let found = unmet_at(&names, &[(cell(SHEET1, "A1", 0), "Pick(1)+Pick(2)")]);
    let lambda = [
        Diagnostic::TargetFunctionUnsupported {
            name: Echo::new("LAMBDA"),
            app: TargetApp::Excel,
            unsupported: Echo::new("2019, 2021"),
            unverified: Echo::new(""),
            requires: Echo::new(""),
            fallback: TargetFallback::NameError,
            count: 1,
        },
        Diagnostic::BestEffortFunctionUnsupported {
            name: Echo::new("LAMBDA"),
            app: TargetApp::LibreOffice,
            unsupported: Echo::new("24.8"),
            unverified: Echo::new(""),
            requires: Echo::new(""),
            fallback: TargetFallback::Unknown,
            count: 1,
        },
    ];
    let via = at("A1").with_defined_name("Pick");
    let expected: Vec<_> = (lambda.into_iter().chain([xlookup(1)]))
        .map(|diagnostic| (diagnostic, via.clone()))
        .collect();
    assert_eq!(found, expected);
}

#[test]
fn a_bare_name_of_a_defined_name_or_table_calls_no_function() {
    let names = Names::default()
        .defined_name("Filter", None, "A1:A3")
        .table("Sort");
    let formulas = [(cell(SHEET1, "A1", 0), "SUM(Filter)+SUM(Sort)")];
    assert_eq!(unmet_at(&names, &formulas), []);
    let elsewhere = names.clone().defined_name("Sort", Some(1), "A1");
    assert_eq!(unmet_at(&elsewhere, &formulas), []);
    let call = [(cell(SHEET1, "A2", 0), "SORT(A1:A3)")];
    let sort = Diagnostic::TargetFunctionUnsupported {
        name: Echo::new("SORT"),
        app: TargetApp::Excel,
        unsupported: Echo::new("2019"),
        unverified: Echo::new(""),
        requires: Echo::new(""),
        fallback: TargetFallback::NameError,
        count: 1,
    };
    let best_effort = Diagnostic::BestEffortFunctionUnsupported {
        name: Echo::new("SORT"),
        app: TargetApp::LibreOffice,
        unsupported: Echo::new(""),
        unverified: Echo::new("24.8"),
        requires: Echo::new("dynamic_array"),
        fallback: TargetFallback::Unknown,
        count: 1,
    };
    let found = [(sort, at("A2")), (best_effort, at("A2"))];
    assert_eq!(unmet_at(&names, &call), found);
    let local = Names::default().defined_name("Sort", Some(0), "A1:A3");
    assert_eq!(unmet_at(&local, &call), found);
}

#[test]
fn a_function_passed_as_a_value_is_checked() {
    assert_eq!(unmet(&[("A1", "_xleta.XLOOKUP")]), [(xlookup(1), at("A1"))]);
}

#[test]
fn a_function_every_target_has_is_not_reported() {
    let targets = targets(&["excel >= 2021"], &["libreoffice >= 24.8"]);
    let found = judge_against(
        &Rules::default().allow_function("XLOOKUP"),
        &targets,
        &Names::default(),
        &[(cell(SHEET1, "A1", 0), "_xlfn.XLOOKUP(1,A1:A3,B1:B3)")],
    );
    assert_eq!(found, []);
}

#[test]
fn a_function_the_table_has_no_row_for_is_not_checked() {
    assert_eq!(unmet(&[("A1", "SUM(1)+IFERROR(1,2)+NOSUCH(1)")]), []);
}

#[test]
fn only_a_function_of_this_workbook_the_reader_saw_is_checked() {
    let formulas = [
        ("A1", "\"_xlfn.XLOOKUP(1)\""),
        ("A2", "_xludf.XLOOKUP(1)"),
        ("A3", "[1]!XLOOKUP(1)"),
    ];
    assert_eq!(unmet(&formulas), []);
}

#[test]
fn no_targets_check_the_default() {
    let found = judge_against(
        &Rules::default()
            .allow_function("XLOOKUP")
            .allow_function("WEBSERVICE"),
        &Targets::default(),
        &Names::default(),
        &[
            (cell(SHEET1, "A1", 0), "_xlfn.XLOOKUP(1,A1:A3,B1:B3)"),
            (cell(SHEET1, "A2", 0), "WEBSERVICE(\"https://x\")"),
        ],
    );
    let codes: Vec<Code> = found
        .iter()
        .map(|(diagnostic, _)| diagnostic.code())
        .collect();
    assert_eq!(codes, [Code::TargetFunctionUnsupported]);
}
