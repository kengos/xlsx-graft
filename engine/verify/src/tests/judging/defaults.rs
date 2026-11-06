//! `verify` with the engine's own table ([`DefaultFunctions`]): what each
//! class of the compatibility table's safety column reports, how the
//! template and the policy change it, and what the formulas Excel saved
//! report.

use super::{TABLE, cells, judged_by, table};
use crate::tests::excel_365;
use crate::{DefaultFunctions, FunctionClass, FunctionTable, Limits, Rules};

/// What verifying `book` under `rules` with [`DefaultFunctions`] finds, one
/// line each, the functions checked against [`excel_365`].
fn judged_by_default(book: super::Book, rules: &Rules) -> Vec<String> {
    judged_by(
        book,
        &Limits::default(),
        rules,
        &DefaultFunctions,
        &excel_365(),
    )
}

#[test]
fn a_workbook_that_calls_only_reviewed_safe_functions_reports_nothing() {
    let one: &[&str] = &[
        "SUM(B1:B3)+IF(B1&gt;0,1,0)",
        "SUMIFS(B1:B3,C1:C3,\"x\")",
        "VLOOKUP(1,B1:C3,2,FALSE)+INDEX(B1:B3,MATCH(1,C1:C3,0))",
        "TEXT(DATE(2026,11,6),\"yyyy-mm-dd\")&amp;ROUND(1.5,0)",
        "_xlfn.XLOOKUP(1,B1:B3,C1:C3)+_xlfn.LET(_xlpm.v,1,_xlpm.v*2)",
        "NOW()+RAND()",
    ];
    let book = cells("", [one, &[]]);
    assert_eq!(
        judged_by_default(book, &Rules::default()),
        Vec::<String>::new()
    );
}

#[test]
fn every_function_not_marked_safe_or_not_listed_is_denied_by_code_and_denial() {
    let one: &[&str] = &[
        "WEBSERVICE(\"https://example.com\")",
        "INFO(\"osversion\")",
        "FILTERXML(\"&lt;a/&gt;\",\"//a\")",
        "MYFUNCTION(1)",
    ];
    let book = cells("", [one, &[]]);
    let expected = [
        "/xl/worksheets/sheet1.xml A1: dangerous WEBSERVICE x1",
        "/xl/worksheets/sheet1.xml A2: dangerous INFO x1",
        "/xl/worksheets/sheet1.xml A3: unreviewed FILTERXML x1",
        "/xl/worksheets/sheet1.xml A4: unknown MYFUNCTION x1",
    ];
    assert_eq!(judged_by_default(book, &Rules::default()), expected);
}

#[test]
fn the_template_allows_what_the_table_denies_and_the_policys_forbid_wins_over_the_allow() {
    let one: &[&str] =
        &["WEBSERVICE(\"u\")+FILTERXML(\"x\",\"y\")+MYFUNCTION(1)+RTD(\"x\",,\"y\")"];
    let rules = Rules::default()
        .allow_function("WEBSERVICE")
        .allow_function("FILTERXML")
        .allow_function("MyFunction")
        .allow_function("RTD")
        .forbid_function("RTD", "no real-time data");
    let book = cells("", [one, &[]]);
    let expected = ["/xl/worksheets/sheet1.xml A1: forbidden RTD x1"];
    assert_eq!(judged_by_default(book, &rules), expected);
}

#[test]
fn indirect_and_offset_get_a_notice_the_template_allow_keeps_and_the_policy_forbid_replaces() {
    let one: &[&str] = &[
        "SUM(INDIRECT(\"B1:B3\"))+INDIRECT(\"C1\")",
        "SUM(OFFSET(B1,1,1,2,2))",
    ];
    let book = cells("", [one, &["INDIRECT(\"S1!A1\")"]]);
    let expected = [
        "/xl/worksheets/sheet1.xml A1: unchecked INDIRECT x2",
        "/xl/worksheets/sheet1.xml A2: unchecked OFFSET x1",
        "/xl/worksheets/sheet2.xml A1: unchecked INDIRECT x1",
    ];
    assert_eq!(judged_by_default(book, &Rules::default()), expected);
    // The template's allow does not stop the `function_reference_unchecked`
    // notice for INDIRECT; the policy's forbid of OFFSET reports
    // `function_forbidden` in place of the notice.
    let rules = Rules::default()
        .allow_function("INDIRECT")
        .forbid_function("OFFSET", "no moving references");
    let book = cells("", [one, &[]]);
    let expected = [
        "/xl/worksheets/sheet1.xml A1: unchecked INDIRECT x2",
        "/xl/worksheets/sheet1.xml A2: forbidden OFFSET x1",
    ];
    assert_eq!(judged_by_default(book, &rules), expected);
}

#[test]
fn a_function_passed_by_its_name_with_xleta_is_judged_as_that_function() {
    let one: &[&str] = &[
        "_xlfn.GROUPBY(A1:A3,B1:B3,_xleta.SUM)",
        "_xlfn.PIVOTBY(A1:A3,B1:B3,C1:C3,_xleta.AVERAGE)",
        "_xlfn.GROUPBY(A1:A3,B1:B3,_xleta.WEBSERVICE)",
        "_xlfn.PIVOTBY(A1:A3,B1:B3,C1:C3,_xleta.FILTERXML)",
        "_xlfn.BYROW(A1:B3,_xleta.MYFUNCTION)",
    ];
    let book = cells("", [one, &[]]);
    let expected = [
        "/xl/worksheets/sheet1.xml A3: dangerous WEBSERVICE x1",
        "/xl/worksheets/sheet1.xml A4: unreviewed FILTERXML x1",
        "/xl/worksheets/sheet1.xml A5: unknown MYFUNCTION x1",
    ];
    assert_eq!(judged_by_default(book, &Rules::default()), expected);
}

#[test]
fn a_lambda_in_a_defined_name_is_judged_by_the_functions_inside_it_not_by_its_name() {
    let names = r#"<definedName name="Double">_xlfn.LAMBDA(_xlpm.x,_xlpm.x*2)</definedName><definedName name="Fetch">_xlfn.LAMBDA(_xlpm.u,_xlfn.WEBSERVICE(_xlpm.u))</definedName>"#;
    let one: &[&str] = &["Double(A1)+Double(2)", "Fetch(\"u\")"];
    let book = cells(names, [one, &[]]);
    let expected = [
        "/xl/workbook.xml definedName Fetch: dangerous WEBSERVICE x1",
        "/xl/worksheets/sheet1.xml A2 via Fetch: dangerous WEBSERVICE x1",
    ];
    assert_eq!(judged_by_default(book, &Rules::default()), expected);
}

/// The formulas Excel for Mac saved in the workbook the formula reader's
/// tests read (`engine/formula/src/tests/excel.rs`), escaped for `<f>`,
/// one cell each in the order of that file; the defined names hold the
/// formulas Excel saved for `MyLambda` and `Rate`, `Rate`'s sheet renamed
/// to the sheet the test workbook has.
const SAVED: &[&str] = &[
    "_xlfn._xlws.FILTER(A1:A3,B1:B3&gt;1)",
    "_xlfn._xlws.SORT(A1:A3)",
    "_xlfn.SEQUENCE(3)",
    "_xlfn.LAMBDA(_xlpm.x,_xlpm.x+1)(A1)",
    "_xlfn.LET(_xlpm.v,A1,_xlpm.v*2)",
    "_xlfn.BYROW(A1:B3,_xleta.SUM)",
    "_xlfn.MAP(A1:A3,_xlfn.LAMBDA(_xlpm.x,_xlpm.x*2))",
    "MyLambda(A1)",
    "LOG10(100)+Rate",
    "SUM(T[a\"b])",
    "SUM(T[x'[y']])",
    "#REF!&amp;T[[#This Row],[a\"b]]",
    "SUM(#REF!)",
    "'it''s'!A1+'a b'!A1",
    "SUM(Sheet1:Sheet2!A1)",
    "_xlfn.WEBSERVICE(\"https://example.com\")",
    "SUM(_xlfn.ANCHORARRAY(J1))",
    "A1:A3",
    "IF(TRUE,\"SUM(\"\"x\"\")\",0)",
    "SUM({1,2;3,4})",
    "A1&gt;1",
    "ISNUMBER(K1)",
];

#[test]
fn the_formulas_excel_saved_are_denied_only_for_webservice_and_anchorarray() {
    let names = r#"<definedName name="MyLambda">_xlfn.LAMBDA(_xlpm.x,_xlpm.x*2)</definedName><definedName name="Rate">S1!$A$1</definedName>"#;
    let book = cells(names, [SAVED, &[]]).of_s1(
        "table",
        "/xl/tables/table1.xml",
        TABLE,
        &table("T", "a&quot;b", ""),
    );
    // WEBSERVICE reaches the network. ANCHORARRAY, which Excel saves for
    // a reference to a spilled range (`J1#`), is not reviewed: no
    // definition of it was found to cite.
    let expected = [
        "/xl/worksheets/sheet1.xml A16: dangerous WEBSERVICE x1",
        "/xl/worksheets/sheet1.xml A17: unreviewed ANCHORARRAY x1",
    ];
    assert_eq!(judged_by_default(book, &Rules::default()), expected);
}

/// A table that schedules `ENCODEURL` to be denied from xlsx-graft 0.9.0,
/// for the made-up reason "it builds a URL", and passes the rest to
/// [`DefaultFunctions`].
struct ScheduledEncodeurl;

impl FunctionTable for ScheduledEncodeurl {
    fn classify(&self, name: &str) -> Option<FunctionClass<'_>> {
        if name == "ENCODEURL" {
            return Some(FunctionClass::Scheduled {
                from: "0.9.0",
                reason: "it builds a URL",
            });
        }
        DefaultFunctions.classify(name)
    }
}

#[test]
fn verify_accepts_a_scheduled_function_with_a_notice_unless_the_policy_denies_it_now() {
    let one: &[&str] = &["ENCODEURL(\"a\")&amp;SUM(1)"];
    let judge = |rules: &Rules| {
        let book = cells("", [one, &[]]);
        judged_by(
            book,
            &Limits::default(),
            rules,
            &ScheduledEncodeurl,
            &excel_365(),
        )
    };
    let expected = ["/xl/worksheets/sheet1.xml A1: scheduled ENCODEURL 0.9.0 x1"];
    assert_eq!(judge(&Rules::default()), expected);
    let expected = ["/xl/worksheets/sheet1.xml A1: dangerous ENCODEURL x1"];
    assert_eq!(judge(&Rules::default().deny_scheduled()), expected);
}
