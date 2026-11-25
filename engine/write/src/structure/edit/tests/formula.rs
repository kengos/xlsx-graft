//! How a rename and a delete rewrite the formulas of the defined names:
//! the prefixes of a plain, a quoted and a 3-D reference, the prefixes a
//! rename or a delete leaves unchanged, and when `#REF!` is written.

use super::{defined, formulas, open, original};
use crate::structure::edit::StructureEdit;

/// The edit of sheets `sheets` and the workbook-scoped defined names `n0`,
/// `n1`, ... of `texts`.
fn with(sheets: &[&str], texts: &[&str]) -> StructureEdit<'static> {
    let names: Vec<(String, &str, &str)> = (texts.iter().enumerate())
        .map(|(index, text)| (format!("n{index}"), "", *text))
        .collect();
    let borrowed: Vec<(&str, &str, &str)> = (names.iter())
        .map(|(name, scope, text)| (name.as_str(), *scope, *text))
        .collect();
    open(sheets, &defined(&borrowed))
}

#[test]
fn a_rename_writes_the_new_name_with_the_quotes_it_needs() {
    let mut edit = with(
        &["Data", "Sheet 2", "It's", "Last"],
        &[
            "Data!$A$1+DATA!B2",
            "'Sheet 2'!$A$1",
            "'It''s'!A1",
            "Data:Last!A1",
            "Last:Data!A1",
            "'Sheet _x0032_'!A1",
            "SUM(Data!A1:B2,Last!C3)",
            "Last!A1",
        ],
    );
    edit.rename_sheet("data", "A B").unwrap();
    edit.rename_sheet("sheet 2", "Plain").unwrap();
    edit.rename_sheet("It's", "O'Neil").unwrap();
    assert_eq!(
        formulas(&edit),
        [
            "'A B'!$A$1+'A B'!B2",
            "Plain!$A$1",
            "'O''Neil'!A1",
            "'A B:Last'!A1",
            "'Last:A B'!A1",
            "Plain!A1",
            "SUM('A B'!A1:B2,Last!C3)",
            "Last!A1",
        ]
    );
    // What names no renamed sheet is as it was.
    assert!(original(&edit, 7));
    assert!(!edit.state.ref_written);
}

#[test]
fn a_rename_leaves_a_book_a_malformed_formula_and_a_name_without_a_sheet_unchanged() {
    let mut edit = with(
        &["Data", "B"],
        &[
            "[1]Data!A1",
            "\"abc+Data!A1",
            "Data",
            "SUM(1,2)",
            "Data!A1:B:Data!B2",
        ],
    );
    edit.rename_sheet("Data", "Fresh").unwrap();
    assert_eq!(
        formulas(&edit)[..4],
        ["[1]Data!A1", "\"abc+Data!A1", "Data", "SUM(1,2)"]
    );
    assert!((0..4).all(|index| original(&edit, index)));
}

#[test]
fn a_rename_reads_a_formula_this_edit_wrote_with_its_escapes_kept() {
    let mut edit = open(&["Data"], "");
    edit.set_defined_name("N", None, "Data!A1&\"_x0041_\"")
        .unwrap();
    edit.rename_sheet("Data", "Quarter").unwrap();
    assert_eq!(formulas(&edit), ["Quarter!A1&\"_x0041_\""]);
}

#[test]
fn a_delete_writes_ref_for_a_reference_to_the_sheet_alone() {
    let mut edit = with(
        &["Alpha", "S 1", "Gamma"],
        &[
            "Alpha!$A$1+Gamma!B2",
            "'S 1'!$A$1:$B$2",
            "[1]Alpha!A1",
            "Alpha:Alpha!X1",
            "SUM(alpha!A1,Gamma!A1)",
        ],
    );
    edit.delete_sheet("alpha").unwrap();
    edit.delete_sheet("S 1").unwrap();
    assert_eq!(
        formulas(&edit),
        [
            "#REF!+Gamma!B2",
            "#REF!",
            "[1]Alpha!A1",
            "#REF!",
            "SUM(#REF!,Gamma!A1)",
        ]
    );
    assert!(original(&edit, 2));
    assert!(edit.state.ref_written);
}

#[test]
fn a_delete_writes_one_ref_in_place_of_the_whole_operand_the_prefix_qualifies() {
    let cases = [
        ("Sheet1!$A$1", "#REF!"),
        ("Sheet1!A1+1", "#REF!+1"),
        ("SUM(Sheet1!A1:B2)", "SUM(#REF!)"),
        ("Sheet1!A:C", "#REF!"),
        ("Sheet1!1:3", "#REF!"),
        ("'a b'!$A$1", "#REF!"),
        ("Sheet1!MyName", "#REF!"),
        ("Sheet1!MyName+Sheet1!A1:B2", "#REF!+#REF!"),
        ("Sheet1!#REF!+1", "#REF!+1"),
        // Only a cell right after the `:` is part of the operand.
        ("Sheet1!A1:MyName", "#REF!:MyName"),
        ("Sheet1!A1:B2:C3", "#REF!:C3"),
        ("Sheet1!A1 + Sheet1!B2", "#REF! + #REF!"),
        ("Sheet1!A1+B2", "#REF!+B2"),
        ("Sheet1!A:C:D1", "#REF!:D1"),
        ("Sheet1!T[a]", "#REF!"),
        ("Sheet1!T[a]+Sheet1!T[[#Headers],[b]]", "#REF!+#REF!"),
    ];
    for (text, expected) in cases {
        let mut edit = with(&["Sheet1", "a b", "Keep"], &[text]);
        edit.delete_sheet("Sheet1").unwrap();
        edit.delete_sheet("a b").unwrap();
        assert_eq!(formulas(&edit), [expected], "{text}");
        assert!(edit.state.ref_written, "{text}");
    }
}

#[test]
fn a_delete_writes_ref_for_the_whole_operand_of_a_3d_reference_whose_both_ends_are_deleted() {
    let mut edit = with(&["Sheet1", "Sheet3", "Keep"], &["Sheet1:Sheet3!A1"]);
    edit.delete_sheet("Sheet1").unwrap();
    assert_eq!(formulas(&edit), ["Sheet3!A1"]);
    edit.delete_sheet("Sheet3").unwrap();
    assert_eq!(formulas(&edit), ["#REF!"]);
    assert!(edit.state.ref_written);
    let mut edit = with(&["Sheet1", "Keep"], &["Sheet1:sheet1!A1:B2"]);
    edit.delete_sheet("Sheet1").unwrap();
    assert_eq!(formulas(&edit), ["#REF!"]);
    assert!(edit.state.ref_written);
}

#[test]
fn a_delete_moves_the_end_of_a_3d_reference_inward_in_the_order_before_the_delete() {
    let sheets = ["Alpha", "Beta", "Gamma", "Delta"];
    let texts = [
        "Alpha:Delta!X1",
        "Delta:Alpha!X1",
        "Beta:Gamma!X1",
        "Alpha:Beta!X1",
        "Gamma:Delta!X1",
    ];
    // Deleting the first end of the first, the second and the fourth name.
    let mut edit = with(&sheets, &texts);
    edit.delete_sheet("Alpha").unwrap();
    assert_eq!(
        formulas(&edit),
        [
            "Beta:Delta!X1",
            "Delta:Beta!X1",
            "Beta:Gamma!X1",
            "Beta!X1",
            "Gamma:Delta!X1"
        ]
    );
    assert!(!edit.state.ref_written);
    // Deleting the last end.
    let mut edit = with(&sheets, &texts);
    edit.delete_sheet("Delta").unwrap();
    assert_eq!(
        formulas(&edit),
        [
            "Alpha:Gamma!X1",
            "Gamma:Alpha!X1",
            "Beta:Gamma!X1",
            "Alpha:Beta!X1",
            "Gamma!X1"
        ]
    );
    // Inside the range, nothing changes.
    let mut edit = with(&sheets, &texts);
    edit.delete_sheet("Beta").unwrap();
    assert_eq!(
        formulas(&edit),
        [
            "Alpha:Delta!X1",
            "Delta:Alpha!X1",
            "Gamma!X1",
            "Alpha!X1",
            "Gamma:Delta!X1"
        ]
    );
    assert!(original(&edit, 0) && original(&edit, 1) && original(&edit, 4));
}

#[test]
fn a_3d_reference_a_quoted_one_and_an_unknown_end_are_rewritten_or_kept() {
    let mut edit = with(
        &["A B", "Gamma", "Delta"],
        &[
            "'A B:Gamma'!X1",
            "'Gamma:Delta'!X1",
            "Zed:Gamma!X1",
            "Gamma:Zed!X1",
            "Alpha:Beta:Gamma!X1",
        ],
    );
    edit.delete_sheet("gamma").unwrap();
    assert_eq!(formulas(&edit)[..2], ["'A B'!X1", "Delta!X1"]);
    // A reference with an end the workbook does not have, or a last end
    // with a colon, is not touched.
    assert!((2..5).all(|index| original(&edit, index)));
    assert!(!edit.state.ref_written);
}

#[test]
fn a_formula_changed_by_one_call_is_read_again_by_the_next() {
    let mut edit = with(&["Alpha", "Beta", "Gamma"], &["Alpha:Gamma!X1+Beta!A1"]);
    edit.delete_sheet("Gamma").unwrap();
    assert_eq!(formulas(&edit), ["Alpha:Beta!X1+Beta!A1"]);
    edit.rename_sheet("Beta", "Fresh").unwrap();
    assert_eq!(formulas(&edit), ["Alpha:Fresh!X1+Fresh!A1"]);
    edit.delete_sheet("Alpha").unwrap();
    assert_eq!(formulas(&edit), ["Fresh!X1+Fresh!A1"]);
}
