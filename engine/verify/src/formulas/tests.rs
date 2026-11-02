//! Tests for the formula judge, driven through [`FormulaJudge`] with a
//! small table. `words.rs` covers what a word in a formula is taken for
//! (a function, a table, a reference, a prefixed or user-defined function),
//! and `names.rs` the defined names a formula reaches.

mod names;
mod words;

use super::{FormulaJudge, Names, Place};
use crate::{FunctionClass, FunctionTable, Rules};
use xlsx_graft_diagnostics::{
    Diagnostic, Diagnostics, Echo, FormulaMalformedReason, FunctionDenial, Location, PartKind,
};

/// The part every test formula is in, unless a test says otherwise.
const SHEET1: &str = "/xl/worksheets/sheet1.xml";

/// A table that marks a few functions reviewed safe, finds a few
/// dangerous, has not reviewed `INFO` and knows nothing else.
struct Table;

impl FunctionTable for Table {
    fn classify(&self, name: &str) -> Option<FunctionClass<'_>> {
        match name {
            "SUM" | "IF" | "LAMBDA" | "LET" | "BYROW" | "MAP" | "FILTER" | "SORT" | "SEQUENCE"
            | "ANCHORARRAY" | "LOG10" => Some(FunctionClass::ReviewedSafe),
            "WEBSERVICE" | "RTD" | "CALL" => Some(FunctionClass::Dangerous {
                reason: "reaches outside the workbook",
            }),
            "INFO" => Some(FunctionClass::Unreviewed),
            _ => None,
        }
    }
}

/// What judging each formula at its place, by `rules` with `names`,
/// reports.
fn judge_at(
    rules: &Rules,
    names: &Names,
    formulas: &[(Place<'_>, &str)],
) -> Vec<(Diagnostic, Location)> {
    let mut judge = FormulaJudge::new(rules, &Table, names);
    for (place, formula) in formulas {
        judge.judge(place, formula);
    }
    let mut diagnostics = Diagnostics::new();
    judge.finish(&mut diagnostics);
    diagnostics
        .finish()
        .into_iter()
        .map(|record| (record.diagnostic, record.location))
        .collect()
}

/// What judging each formula, in the cell it is paired with on the first
/// sheet, reports.
fn judge(rules: &Rules, names: &Names, formulas: &[(&str, &str)]) -> Vec<(Diagnostic, Location)> {
    let placed: Vec<(Place<'_>, &str)> = formulas
        .iter()
        .map(|&(at, formula)| (cell(SHEET1, at, 0), formula))
        .collect();
    judge_at(rules, names, &placed)
}

/// What judging `formula` alone, in A1 of the first sheet, with no rules
/// and no names, reports.
fn one(formula: &str) -> Vec<(Diagnostic, Location)> {
    judge(&Rules::default(), &Names::default(), &[("A1", formula)])
}

/// The cell `at` in `part`, the sheet at index `sheet`.
fn cell<'a>(part: &'a str, at: &'a str, sheet: u32) -> Place<'a> {
    Place {
        part,
        at,
        sheet: Some(sheet),
    }
}

/// The location of a finding first found in the cell `at` of the first
/// sheet.
fn at(place: &str) -> Location {
    Location::default().with_part(SHEET1).with_place(place)
}

/// The table's diagnostic for the dangerous function `name`, found
/// `count` times.
fn dangerous(name: &str, count: u64) -> Diagnostic {
    Diagnostic::FunctionDangerous {
        name: Echo::new(name),
        reason: Echo::new("reaches outside the workbook"),
        count,
    }
}

/// The diagnostic for the function `name` the table does not know.
fn unknown(name: &str) -> Diagnostic {
    Diagnostic::FunctionDenied {
        name: Echo::new(name),
        denial: FunctionDenial::Unknown,
        count: 1,
    }
}

#[test]
fn reviewed_safe_functions_pass() {
    assert_eq!(one("SUM(A1:A3)+IF(TRUE,1,0)"), []);
    assert_eq!(one("1+2"), []);
}

#[test]
fn a_function_the_table_does_not_mark_safe_is_denied_for_why() {
    assert_eq!(
        one("WEBSERVICE(\"https://x\")"),
        [(dangerous("WEBSERVICE", 1), at("A1"))]
    );
    assert_eq!(
        one("INFO(\"osversion\")"),
        [(
            Diagnostic::FunctionDenied {
                name: Echo::new("INFO"),
                denial: FunctionDenial::Unreviewed,
                count: 1,
            },
            at("A1"),
        )]
    );
    for name in ["STOCKHISTORY", "CUBEVALUE"] {
        assert_eq!(one(&format!("{name}(\"x\")")), [(unknown(name), at("A1"))]);
    }
}

#[test]
fn the_template_allows_and_the_policy_forbids_over_the_table() {
    let names = Names::default();
    let formula = [("A1", "WEBSERVICE(\"x\")+SUM(1)+STOCKHISTORY(1)")];
    let allowed = Rules::default()
        .allow_function("webservice")
        .allow_function("StockHistory");
    assert_eq!(judge(&allowed, &names, &formula), []);
    let forbidden = allowed
        .forbid_function("WEBSERVICE", "no network")
        .forbid_function("SUM", "no sums");
    let forbid = |name: &str, reason: &str| Diagnostic::FunctionForbidden {
        name: Echo::new(name),
        reason: Echo::new(reason),
        count: 1,
    };
    assert_eq!(
        judge(&forbidden, &names, &formula),
        [
            (forbid("WEBSERVICE", "no network"), at("A1")),
            (forbid("SUM", "no sums"), at("A1")),
        ]
    );
}

#[test]
fn a_part_gets_one_diagnostic_per_finding_at_its_first_place_with_a_count() {
    let other = "/xl/worksheets/sheet2.xml";
    let found = judge_at(
        &Rules::default(),
        &Names::default(),
        &[
            (cell(SHEET1, "B2", 0), "SUM(1)"),
            (cell(SHEET1, "B3", 0), "WEBSERVICE(1)"),
            (cell(other, "C1", 1), "WEBSERVICE(1)"),
            (cell(SHEET1, "B4", 0), "WEBSERVICE(1)+WEBSERVICE(2)+INFO(1)"),
        ],
    );
    assert_eq!(
        found,
        [
            (dangerous("WEBSERVICE", 3), at("B3")),
            (
                dangerous("WEBSERVICE", 1),
                Location::default().with_part(other).with_place("C1"),
            ),
            (
                Diagnostic::FunctionDenied {
                    name: Echo::new("INFO"),
                    denial: FunctionDenial::Unreviewed,
                    count: 1,
                },
                at("B4"),
            ),
        ]
    );
}

#[test]
fn a_malformed_formula_is_refused_whole_for_its_first_reason() {
    let malformed = |reason| Diagnostic::FormulaMalformed { reason, count: 1 };
    assert_eq!(
        one("WEBSERVICE(\"x)"),
        [(malformed(FormulaMalformedReason::UnclosedString), at("A1"))]
    );
    assert_eq!(
        one("_X0057_EBSERVICE(1)+'S"),
        [(malformed(FormulaMalformedReason::UpperCaseEscape), at("A1"))]
    );
    let every = [
        ("_xD800_", FormulaMalformedReason::UndecodableEscape),
        ("_X0041_", FormulaMalformedReason::UpperCaseEscape),
        ("\"a", FormulaMalformedReason::UnclosedString),
        ("'S", FormulaMalformedReason::UnclosedSheetName),
        ("T[a", FormulaMalformedReason::UnclosedBracket),
        (
            &*format!("T{}", "[".repeat(70)),
            FormulaMalformedReason::BracketsTooDeep,
        ),
        ("1~2", FormulaMalformedReason::UnknownCharacter),
        ("'S'+1", FormulaMalformedReason::StraySheetName),
    ];
    for (formula, reason) in every {
        assert_eq!(one(formula), [(malformed(reason), at("A1"))], "{formula}");
    }
}

#[test]
fn a_dde_link_and_another_workbook_are_judged_as_their_kinds() {
    let formula = [("A1", "cmd|' /C calc'!A0+'C:\\x\\[b.xlsx]S'!A1+[1]S!B1")];
    let names = Names::default();
    let denied = |kind, count| Diagnostic::FormulaDenied { kind, count };
    assert_eq!(
        judge(&Rules::default(), &names, &formula),
        [
            (denied(PartKind::Dde, 1), at("A1")),
            (denied(PartKind::ExternalLink, 2), at("A1")),
        ]
    );
    let rules = Rules::default()
        .allow_part(PartKind::Dde)
        .allow_part(PartKind::ExternalLink);
    assert_eq!(judge(&rules, &names, &formula), []);
    let rules = rules.forbid_part(PartKind::Dde, "no DDE");
    assert_eq!(
        judge(&rules, &names, &formula),
        [(
            Diagnostic::FormulaForbidden {
                kind: PartKind::Dde,
                reason: Echo::new("no DDE"),
                count: 1,
            },
            at("A1"),
        )]
    );
}
