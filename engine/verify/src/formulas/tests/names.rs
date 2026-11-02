//! The defined names a formula reaches: judged by their own formulas,
//! followed from one to the next however deep, in the scope of the
//! formula's sheet.

use super::{SHEET1, at, cell, dangerous, judge, judge_at, one};
use crate::formulas::Place;
use crate::{Names, Rules};
use xlsx_graft_diagnostics::{Diagnostic, FormulaMalformedReason, Location};

/// A defined name holding a LAMBDA that fetches its argument.
const FETCH: &str = "_xlfn.LAMBDA(_xlpm.u,WEBSERVICE(_xlpm.u))";

/// The location of a finding first found in the cell `place` of the first
/// sheet, in the formula of the defined name `name`.
fn via(place: &str, name: &str) -> Location {
    at(place).with_defined_name(name)
}

/// Names `Step1` to `Step<count>`, each naming the next, the last naming
/// `last`.
fn chain(count: u32, last: &str) -> Names {
    (1..=count).fold(Names::default(), |names, step| {
        let next = if step == count {
            last.to_owned()
        } else {
            format!("Step{}+1", step + 1)
        };
        names.defined_name(&format!("Step{step}"), None, &next)
    })
}

#[test]
fn a_lambda_is_judged_by_its_formula() {
    let names = Names::default()
        .defined_name("MyLambda", None, "_xlfn.LAMBDA(_xlpm.x,_xlpm.x*2)")
        .defined_name("Fetch", None, FETCH);
    let rules = Rules::default();
    assert_eq!(
        judge(&rules, &names, &[("A1", "MyLambda(A1)+mylambda")]),
        []
    );
    assert_eq!(
        judge(
            &rules,
            &names,
            &[("A1", "Fetch(\"x\")"), ("A2", "_xlfn.MAP(A1:A3,Fetch)")]
        ),
        [(dangerous("WEBSERVICE", 1), via("A1", "Fetch"))]
    );
}

#[test]
fn a_problem_found_directly_and_through_a_name_are_two_diagnostics() {
    let names = Names::default().defined_name("Fetch", None, FETCH);
    assert_eq!(
        judge(
            &Rules::default(),
            &names,
            &[("A1", "WEBSERVICE(1)"), ("A2", "Fetch(1)")]
        ),
        [
            (dangerous("WEBSERVICE", 1), at("A1")),
            (dangerous("WEBSERVICE", 1), via("A2", "Fetch")),
        ]
    );
}

#[test]
fn a_name_is_followed_to_the_name_whose_formula_holds_the_problem() {
    let names = chain(3, "WEBSERVICE(1)+'S");
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "Step1")]),
        [(
            Diagnostic::FormulaMalformed {
                reason: FormulaMalformedReason::UnclosedSheetName,
                count: 1,
            },
            via("A1", "Step3"),
        )]
    );
    let names = chain(2, "WEBSERVICE(1)");
    assert_eq!(
        judge(
            &Rules::default(),
            &names,
            &[("A1", "Step1"), ("A2", "Step2")]
        ),
        [(dangerous("WEBSERVICE", 1), via("A1", "Step2"))]
    );
}

#[test]
fn a_name_naming_itself_or_a_loop_of_names_ends() {
    let names = Names::default()
        .defined_name(
            "Fact",
            None,
            "_xlfn.LAMBDA(_xlpm.n,IF(_xlpm.n<2,1,_xlpm.n*Fact(_xlpm.n-1)))",
        )
        .defined_name("Ping", None, "Pong+1")
        .defined_name("Pong", None, "Ping+WEBSERVICE(1)");
    let rules = Rules::default();
    assert_eq!(judge(&rules, &names, &[("A1", "Fact(5)")]), []);
    assert_eq!(
        judge(&rules, &names, &[("A1", "Ping"), ("A2", "Pong")]),
        [(dangerous("WEBSERVICE", 1), via("A1", "Pong"))]
    );
}

#[test]
fn names_are_followed_however_deep() {
    let names = chain(200, "WEBSERVICE(1)");
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "Step1")]),
        [(dangerous("WEBSERVICE", 1), via("A1", "Step200"))]
    );
}

#[test]
fn a_sheet_sees_its_own_names_first_then_the_workbook_s_then_any() {
    let names = Names::default()
        .defined_name("F", Some(1), FETCH)
        .defined_name("F", None, "1")
        .defined_name("G", Some(0), "1")
        .defined_name("K", Some(1), FETCH);
    let alone =
        |place: Place<'_>, formula: &str| judge_at(&Rules::default(), &names, &[(place, formula)]);
    let other = "/xl/worksheets/sheet2.xml";
    assert_eq!(alone(cell(SHEET1, "A1", 0), "F(1)+G"), []);
    assert_eq!(
        alone(cell(other, "B1", 1), "F(1)"),
        [(
            dangerous("WEBSERVICE", 1),
            Location::default()
                .with_part(other)
                .with_place("B1")
                .with_defined_name("F"),
        )]
    );
    for formula in ["Sheet2!F(1)", "Sheet2!F"] {
        assert_eq!(
            alone(cell(SHEET1, "A2", 0), formula),
            [(dangerous("WEBSERVICE", 1), via("A2", "F"))]
        );
    }
    assert_eq!(
        alone(cell(SHEET1, "A2", 0), "K"),
        [
            (super::unknown("K"), at("A2")),
            (dangerous("WEBSERVICE", 1), via("A2", "K")),
        ]
    );
    let workbook = Place {
        part: "/xl/workbook.xml",
        at: "definedName H",
        sheet: None,
    };
    assert_eq!(
        alone(workbook, "F+G+K"),
        [
            (
                super::unknown("G"),
                Location::default()
                    .with_part("/xl/workbook.xml")
                    .with_place("definedName H"),
            ),
            (
                super::unknown("K"),
                Location::default()
                    .with_part("/xl/workbook.xml")
                    .with_place("definedName H"),
            ),
            (
                dangerous("WEBSERVICE", 1),
                Location::default()
                    .with_part("/xl/workbook.xml")
                    .with_place("definedName H")
                    .with_defined_name("K"),
            ),
        ]
    );
}

#[test]
fn a_name_in_a_defined_name_s_formula_may_mean_any_of_its_scopes() {
    let names = Names::default()
        .defined_name("Outer", None, "Inner+1")
        .defined_name("Inner", None, "1")
        .defined_name("Inner", Some(0), FETCH);
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "Outer")]),
        [(dangerous("WEBSERVICE", 1), via("A1", "Inner"))]
    );
}

#[test]
fn every_defined_name_of_one_name_and_scope_is_judged() {
    let names = Names::default()
        .defined_name("F", None, "1")
        .defined_name("f", None, FETCH);
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "F(1)")]),
        [(dangerous("WEBSERVICE", 1), via("A1", "f"))]
    );
    assert_eq!(one("F(1)"), [(super::unknown("F"), at("A1"))]);
}

#[test]
fn a_bucket_reached_from_many_sheets_is_read_and_reported_once() {
    let names = Names::default()
        .defined_name("X", None, FETCH)
        .defined_name("X", None, FETCH)
        .defined_name("X", Some(9), "1");
    let placed: Vec<(Place<'_>, &str)> = (0..4)
        .map(|sheet| (cell(SHEET1, "A1", sheet), "X(1)"))
        .collect();
    assert_eq!(
        judge_at(&Rules::default(), &names, &placed),
        [
            (dangerous("WEBSERVICE", 1), via("A1", "X")),
            (dangerous("WEBSERVICE", 1), via("A1", "X")),
        ]
    );
}
