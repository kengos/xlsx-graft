//! Where a delete ends the `#REF!` it writes for a prefix of the deleted
//! sheet: the operand the prefix qualifies as `xlsx_graft_formula::analyze`
//! reads it (`SheetPrefix::operand`), a prefix with no operand (kept as
//! written and reported as `KeptReason::UnknownOperand`), and the result for a
//! formula of 20,000 prefixes.

use super::parts::{CHART, CHART_NS, WORKSHEET, book, kept};
use super::{applied_with_report, defined, formulas, open, original, package};
use crate::structure::{KeptReason, StructureReport};

/// The defined name `n0` of `text` after deleting `Sheet1` of the sheets
/// `Sheet1` and `Keep`, and whether a `#REF!` was written.
fn deleted(text: &str) -> (String, bool) {
    let mut edit = open(&["Sheet1", "Keep"], &defined(&[("n0", "", text)]));
    edit.delete_sheet("Sheet1").unwrap();
    (formulas(&edit).remove(0), edit.state.ref_written)
}

#[test]
fn a_delete_writes_one_ref_for_a_prefix_and_its_operand_and_keeps_a_prefix_with_none() {
    let cases = [
        // The operand starts at the first token that is not whitespace.
        ("Sheet1! A1", "#REF!"),
        ("Sheet1!  MyName+1", "#REF!+1"),
        ("Sheet1! #N/A+1", "#REF!+1"),
        // A cell, a range of cells, of columns or of rows, a table reference.
        ("Sheet1!A1:B2+1", "#REF!+1"),
        ("Sheet1!A:C+Sheet1!1:3", "#REF!+#REF!"),
        ("Sheet1!T[[#Headers],[b]]", "#REF!"),
        // The second cell may come after whitespace, but the `:` may not
        // follow whitespace: `analyze` reads `:B2` as a cell with no prefix.
        ("Sheet1!A1: B2", "#REF!"),
        ("Sheet1!A1 :B2", "#REF! :B2"),
        ("Sheet1!A1 : B2", "#REF! : B2"),
        // The spill operator right after the operand is part of it, so that
        // no `#` is left after the `#REF!` (Excel repairs `#REF!#`).
        ("Sheet1!A2#+1", "#REF!+1"),
        ("SUM(Sheet1!A1:B2#)", "SUM(#REF!)"),
        // Excel saves `Sheet1!A2#` as `_xlfn.ANCHORARRAY(Sheet1!A2)`: the
        // operand of `Sheet1!` is `A2`, the call has no prefix. Excel 16.115.2
        // opens `_xlfn.ANCHORARRAY(#REF!)+1` with no repair.
        (
            "_xlfn.ANCHORARRAY(Sheet1!A2)+1",
            "_xlfn.ANCHORARRAY(#REF!)+1",
        ),
        // A call up to its closing parenthesis.
        ("Sheet1!MyFn(1)", "#REF!"),
        ("Sheet1!MyFn(A1,(B2))+1", "#REF!+1"),
        ("Sheet1!MyFn (1)+2", "#REF!+2"),
        ("SUM(Sheet1!MyFn(1),2)", "SUM(#REF!,2)"),
        ("Sheet1!MyFn(1)+(2)", "#REF!+(2)"),
        ("Sheet1!MyFn(Sheet1!A1)#+1", "#REF!+1"),
        // A `)` with no `(` before it closes nothing.
        ("Sheet1!MyFn(1))+1", "#REF!)+1"),
        // A prefix inside the call is part of what is replaced, and is not
        // written as a second `#REF!`.
        ("Sheet1!MyFn(Sheet1!A1,2)+Sheet1!B2", "#REF!+#REF!"),
        ("Sheet1!MyFn(Sheet1!A1:Sheet1!B2)", "#REF!"),
        // A prefix with no operand to give is kept as written, and the prefixes
        // inside what follows it are rewritten.
        ("Sheet1!A1:Sheet1!B2", "Sheet1!A1:Sheet1!B2"),
        ("Sheet1!+1", "Sheet1!+1"),
        ("Sheet1!", "Sheet1!"),
        ("Sheet1!1", "Sheet1!1"),
        ("Sheet1!TRUE", "Sheet1!TRUE"),
        ("Sheet1!_xlfn.X", "Sheet1!_xlfn.X"),
        // A call whose `(` is never closed has no operand.
        ("Sheet1!MyFn(1", "Sheet1!MyFn(1"),
        ("Sheet1!MyFn(1+Sheet1!A1", "Sheet1!MyFn(1+#REF!"),
        // A parenthesis is not an operand: the prefix is kept, and the
        // prefixes inside it are rewritten.
        ("Sheet1!(A1+1", "Sheet1!(A1+1"),
        ("Sheet1!(A1)", "Sheet1!(A1)"),
        ("Sheet1! (A1+(B2))*2", "Sheet1! (A1+(B2))*2"),
        ("Sheet1!(A1))+1", "Sheet1!(A1))+1"),
        (
            "Sheet1!(Sheet1!MyFn(1)+Sheet1!A1)*2",
            "Sheet1!(#REF!+#REF!)*2",
        ),
        ("Sheet1!A1:Sheet1!B2+Sheet1!C3", "Sheet1!A1:Sheet1!B2+#REF!"),
    ];
    for (text, expected) in cases {
        let (after, ref_written) = deleted(text);
        assert_eq!(after, expected, "{text}");
        assert_eq!(ref_written, expected.contains("#REF!"), "{text}");
    }
}

#[test]
fn a_formula_kept_for_an_unknown_operand_is_as_the_workbook_has_it() {
    let texts = ["Sheet1!A1:Sheet1!B2", "Sheet1!(A1", "Sheet1!(A1)"];
    let names: Vec<(String, &str, &str)> = (texts.iter().enumerate())
        .map(|(index, text)| (format!("n{index}"), "", *text))
        .collect();
    let borrowed: Vec<(&str, &str, &str)> = (names.iter())
        .map(|(name, scope, text)| (name.as_str(), *scope, *text))
        .collect();
    let mut edit = open(&["Sheet1", "Keep"], &defined(&borrowed));
    edit.delete_sheet("Sheet1").unwrap();
    assert!((0..3).all(|index| original(&edit, index)));
    assert!(!edit.state.ref_written);
}

#[test]
fn a_prefix_kept_for_an_unknown_operand_is_reported_for_a_name_and_for_the_other_parts() {
    let sheet = format!(
        r#"<worksheet xmlns="{}"><sheetData><row r="1"><c r="A1"><f>Alpha!A1:Alpha!B2+Alpha!C3</f></c><c r="B1"><f>Beta!A1</f></c></row></sheetData><conditionalFormatting sqref="F1:F3"><cfRule type="dataBar"><dataBar><cfvo type="formula" val="Alpha!A1:Alpha!B2+Alpha!C3"/><cfvo type="num" val="5"/></dataBar></cfRule></conditionalFormatting><conditionalFormatting sqref="G1:G3"><cfRule type="dataBar"><dataBar><cfvo type="formula" val="Alpha!D1:Alpha!D2"/><cfvo type="num" val="5"/></dataBar></cfRule></conditionalFormatting></worksheet>"#,
        super::NS
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let names = defined(&[
        ("Kept", "", "Alpha!(A1"),
        ("Fine", "", "Alpha!A1+Beta!A1"),
        ("Both", "", "Alpha!MyFn(1"),
        ("Paren", "", "Alpha!(A1)"),
        // The prefix inside the call goes with the call: not reported.
        ("Inner", "", "Alpha!MyFn(Alpha!A1:Alpha!B2)"),
    ]);
    let (output, report) = applied_with_report(book(&names, &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    let part = "/xl/worksheets/sheet2.xml";
    let workbook = "/xl/workbook.xml";
    assert_eq!(
        kept(&report),
        [
            (
                part,
                "f",
                Some("A1"),
                Some("Alpha"),
                KeptReason::UnknownOperand
            ),
            (
                part,
                "cfvo@val",
                Some("F1:F3"),
                Some("Alpha"),
                KeptReason::Outside
            ),
            (
                part,
                "cfvo@val",
                Some("G1:G3"),
                Some("Alpha"),
                KeptReason::Outside
            ),
            (
                workbook,
                "definedName",
                Some("definedName Kept"),
                Some("Alpha"),
                KeptReason::UnknownOperand
            ),
            (
                workbook,
                "definedName",
                Some("definedName Both"),
                Some("Alpha"),
                KeptReason::UnknownOperand
            ),
            (
                workbook,
                "definedName",
                Some("definedName Paren"),
                Some("Alpha"),
                KeptReason::UnknownOperand
            ),
        ]
    );
    // The prefix that has an operand is written, and so is the other part's.
    let written = output.part(part).unwrap();
    assert!(written.contains("<f>Alpha!A1:Alpha!B2+#REF!</f>"));
    assert!(written.contains(r#"val="Alpha!A1:Alpha!B2+Alpha!C3""#));
    assert!(written.contains(r#"val="Alpha!D1:Alpha!D2""#));
    let names = output.part(workbook).unwrap();
    assert!(names.contains("#REF!+Beta!A1"));
    assert!(names.contains(r#"name="Inner" comment="c">#REF!</definedName>"#));
}

#[test]
fn a_delete_that_keeps_every_prefix_does_not_set_the_workbook_to_calculate_on_load() {
    let sheet = format!(
        r#"<worksheet xmlns="{}"><c r="A1"><f>Alpha!A1:Alpha!B2</f></c></worksheet>"#,
        super::NS
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let names = defined(&[("Kept", "", "Alpha!(A1")]);
    let (output, report) = applied_with_report(book(&names, &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert_eq!(report.kept().len(), 2);
    assert!(output.holds(&sheet));
    assert!(
        !output
            .part("/xl/workbook.xml")
            .unwrap()
            .contains("fullCalcOnLoad")
    );
}

#[test]
fn a_name_set_again_after_a_delete_is_not_reported() {
    let names = defined(&[("Kept", "", "Alpha!(A1")]);
    let (_, report) = applied_with_report(book(&names, &[]), |edit| {
        edit.delete_sheet("Alpha").unwrap();
        edit.set_defined_name("Kept", None, "1+1").unwrap();
    });
    assert_eq!(kept(&report), []);
}

#[test]
fn a_sheet_added_with_the_name_of_the_deleted_one_is_not_named_by_a_rewritten_formula() {
    let names = defined(&[
        ("Call", "", "Alpha!MyFn(1)"),
        ("Pair", "", "Alpha!A1:Alpha!B2"),
        ("Cell", "", "Alpha!A1"),
    ]);
    let (output, report) = applied_with_report(book(&names, &[]), |edit| {
        edit.delete_sheet("Alpha").unwrap();
        edit.add_sheet("Alpha", None, None).unwrap();
    });
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(workbook.contains(">#REF!</definedName>"));
    assert!(!workbook.contains("MyFn"));
    // What still names `Alpha` is in the report.
    assert_eq!(
        kept(&report),
        [(
            "/xl/workbook.xml",
            "definedName",
            Some("definedName Pair"),
            Some("Alpha"),
            KeptReason::UnknownOperand,
        )]
    );
}

#[test]
fn a_delete_of_a_formula_with_20000_prefixes_replaces_each_with_its_operand() {
    let count = 20_000;
    let text = format!("{}1", "Sheet1!A1+".repeat(count));
    let (after, ref_written) = deleted(&text);
    assert_eq!(after, format!("{}1", "#REF!+".repeat(count)));
    assert!(ref_written);
    let text = format!(
        "{}1",
        "Sheet1!MyName+Sheet1!A1:B2*Sheet1!MyFn(1)+".repeat(count / 4)
    );
    let (after, _) = deleted(&text);
    assert_eq!(
        after,
        format!("{}1", "#REF!+#REF!*#REF!+".repeat(count / 4))
    );
}

/// The sheets `Alpha`, `Beta` and `Keep`, the defined names `names` and the
/// chart `chart`, after deleting `Alpha` and then `Beta`: the report.
fn deleted_in_turn(names: &str, chart: &str) -> StructureReport {
    let extra = [("xl/charts/chart1.xml", CHART, chart)];
    let sheets = [("Alpha", ""), ("Beta", ""), ("Keep", "")];
    let package = package("", &sheets, names, &extra);
    let (_, report) = applied_with_report(package, |edit| {
        edit.delete_sheet("Alpha").unwrap();
        edit.delete_sheet("Beta").unwrap();
    });
    report
}

#[test]
fn a_kept_prefix_is_reported_with_the_sheet_of_the_first_delete_that_kept_one() {
    let text = "Alpha!A1:Alpha!B2+Beta!A1:Beta!B2";
    let chart = format!(
        r#"<c:chartSpace xmlns:c="{CHART_NS}"><c:chart><c:ser><c:val><c:numRef><c:f>{text}</c:f></c:numRef></c:val></c:ser></c:chart></c:chartSpace>"#
    );
    let report = deleted_in_turn(&defined(&[("Two", "", text)]), &chart);
    assert_eq!(
        kept(&report),
        [
            (
                "/xl/charts/chart1.xml",
                "f",
                None,
                Some("Alpha"),
                KeptReason::UnknownOperand
            ),
            (
                "/xl/workbook.xml",
                "definedName",
                Some("definedName Two"),
                Some("Alpha"),
                KeptReason::UnknownOperand
            ),
        ]
    );
}
