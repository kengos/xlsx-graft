//! What a word in a formula is taken for: a function, a table, a
//! reference, or a prefixed or user-defined function.

use super::{at, dangerous, judge, one, unknown};
use crate::{Names, Rules};
use xlsx_graft_diagnostics::{Diagnostic, Echo};

#[test]
fn a_quote_in_a_structured_reference_does_not_hide_a_call() {
    assert_eq!(
        one("SUM(T[a']]+WEBSERVICE(\"x\"))"),
        [(dangerous("WEBSERVICE", 1), at("A1"))]
    );
    assert_eq!(one("SUM(T[x'[y']])+SUM(T[a\"b])"), []);
}

#[test]
fn a_column_or_a_sheet_named_like_a_function_is_no_call() {
    assert_eq!(one("SUM(Table1[WEBSERVICE])+Table1[[#This Row],[RTD]]"), []);
    assert_eq!(one("RTD!A1+'WEBSERVICE'!B2+SUM(Sheet1:Sheet2!A1)"), []);
}

#[test]
fn a_prefixed_function_is_looked_up_by_its_name() {
    assert_eq!(
        one("_xlfn._xlws.FILTER(A1:A3,A1:A3>0)+_xlfn.BYROW(A1:B3,_xleta.SUM)"),
        []
    );
    assert_eq!(one("_xlfn.SEQUENCE(3)+SUM(_xlfn.ANCHORARRAY(J1))"), []);
    assert_eq!(
        one("_xlfn.BYROW(A1:B3,_xleta.WEBSERVICE)"),
        [(dangerous("WEBSERVICE", 1), at("A1"))]
    );
}

#[test]
fn a_prefixed_function_is_not_a_defined_name() {
    let names = Names::default().defined_name("STOCKHISTORY", None, "1");
    assert_eq!(
        judge(
            &Rules::default(),
            &names,
            &[("A1", "_xlfn.STOCKHISTORY(1)")]
        ),
        [(unknown("STOCKHISTORY"), at("A1"))]
    );
}

#[test]
fn a_user_defined_function_is_denied_unless_the_template_allows_it_by_its_prefix() {
    let names = Names::default().defined_name("SUM", None, "1");
    let formula = [("A1", "_xludf.SUM(1)+_xlfn._xludf.MyFn(2)")];
    assert_eq!(
        judge(&Rules::default(), &names, &formula),
        [
            (unknown("_XLUDF.SUM"), at("A1")),
            (unknown("_XLUDF.MYFN"), at("A1")),
        ]
    );
    let rules = Rules::default()
        .allow_function("_xludf.sum")
        .allow_function("_XLUDF.MYFN");
    assert_eq!(judge(&rules, &names, &formula), []);
}

#[test]
fn a_name_that_is_no_defined_name_or_table_is_looked_up_as_a_function() {
    assert_eq!(one("LOG10(100)+Rate"), [(unknown("RATE"), at("A1"))]);
    let names = Names::default().defined_name("Rate", None, "Sheet1!$B$1");
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "LOG10(100)+Rate")]),
        []
    );
}

#[test]
fn a_table_passes_as_a_name_but_not_as_a_call() {
    let names = Names::default().table("Sales");
    let rules = Rules::default();
    assert_eq!(judge(&rules, &names, &[("A1", "SUM(sales)")]), []);
    assert_eq!(
        judge(&rules, &names, &[("A1", "Sales(1)")]),
        [(unknown("SALES"), at("A1"))]
    );
    assert_eq!(one("SUM(Sales)"), [(unknown("SALES"), at("A1"))]);
}

#[test]
fn a_name_before_another_workbook_is_looked_up_as_a_function() {
    let names = Names::default().defined_name("Rate", None, "1");
    let rules = Rules::default().allow_part(crate::PartKind::ExternalLink);
    assert_eq!(
        judge(&rules, &names, &[("A1", "[1]!Rate+[1]Book!Fn(1)")]),
        [(unknown("FN"), at("A1")), (unknown("RATE"), at("A1"))]
    );
}

#[test]
fn a_call_only_the_safety_net_sees_is_judged_too() {
    assert_eq!(one("IF(TRUE,\"SUM(\"\"x\"\")\",0)"), []);
    assert_eq!(
        one("\"WEBSERVICE(1)\"&\"_xludf.F(\""),
        [
            (dangerous("WEBSERVICE", 1), at("A1")),
            (unknown("_XLUDF.F"), at("A1")),
        ]
    );
}

#[test]
fn a_defined_name_of_a_function_s_name_does_not_make_it_safe() {
    let names = Names::default()
        .defined_name("WEBSERVICE", None, "1")
        .defined_name("RTD", None, "1");
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "WEBSERVICE(1)+RTD")]),
        [
            (dangerous("WEBSERVICE", 1), at("A1")),
            (dangerous("RTD", 1), at("A1")),
        ]
    );
}

#[test]
fn a_table_of_a_function_s_name_does_not_make_it_safe() {
    let names = Names::default().table("RTD").table("Sales");
    assert_eq!(
        judge(
            &Rules::default(),
            &names,
            &[("A1", "_xlfn.BYROW(A1:A2,RTD)+SUM(Sales)")]
        ),
        [(dangerous("RTD", 1), at("A1"))]
    );
}

#[test]
fn a_call_only_the_safety_net_sees_after_a_bang_is_no_defined_name() {
    let names = Names::default().defined_name("Fetch", None, "1");
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "\"[1]!Fetch(\"")]),
        [(unknown("FETCH"), at("A1"))]
    );
}

#[test]
fn the_policy_forbids_a_user_defined_function_by_its_name_too() {
    let rules = Rules::default()
        .allow_function("_XLUDF.WEBSERVICE")
        .forbid_function("WEBSERVICE", "no network");
    assert_eq!(
        judge(&rules, &Names::default(), &[("A1", "_xludf.WEBSERVICE(1)")]),
        [(
            Diagnostic::FunctionForbidden {
                name: Echo::new("WEBSERVICE"),
                reason: Echo::new("no network"),
                count: 1,
            },
            at("A1"),
        )]
    );
}

#[test]
fn a_defined_name_out_of_the_word_s_scope_does_not_make_an_unknown_call_safe() {
    let names = Names::default().defined_name("_xll.Evil", Some(5), "1");
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "_xll.Evil(1)")]),
        [(unknown("_XLL.EVIL"), at("A1"))]
    );
    assert_eq!(
        judge(&Rules::default(), &names, &[("A1", "\"_xll.Evil(1)\"")]),
        [(unknown("_XLL.EVIL"), at("A1"))]
    );
}
