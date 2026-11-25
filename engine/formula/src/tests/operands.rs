//! The operand each sheet prefix qualifies (`SheetPrefix::operand`): the
//! text from the operand's first token to its last, for each kind of operand,
//! and `None` for a prefix with no operand to give.

use crate::analyze;

/// The operand of each sheet prefix in `formula`, as written, in the order of
/// the prefixes.
fn operands(formula: &str) -> Vec<Option<String>> {
    let read = analyze(formula);
    (read.facts.sheet_prefixes.iter())
        .map(|prefix| prefix.operand.map(|span| read.slice(span).to_owned()))
        .collect()
}

/// Checks each case: a formula and the operand of each of its prefixes.
fn check(cases: &[(&str, &[Option<&str>])]) {
    for (formula, expected) in cases {
        let expected: Vec<Option<String>> = expected
            .iter()
            .map(|operand| operand.map(str::to_owned))
            .collect();
        assert_eq!(operands(formula), expected, "{formula}");
    }
}

#[test]
fn a_cell_or_a_range_is_the_operand() {
    check(&[
        ("Sheet1!A1", &[Some("A1")]),
        ("Sheet1!$A$1+1", &[Some("$A$1")]),
        ("Sheet1!A1:B2", &[Some("A1:B2")]),
        // The second cell may come after whitespace, but the `:` may not.
        ("Sheet1!A1: B2", &[Some("A1: B2")]),
        ("Sheet1!A1 :B2", &[Some("A1")]),
        ("Sheet1!A1 : B2", &[Some("A1")]),
        ("Sheet1!A:C", &[Some("A:C")]),
        ("Sheet1!$A:$C", &[Some("$A:$C")]),
        ("Sheet1!$1:$3", &[Some("$1:$3")]),
        ("Sheet1!1:3", &[Some("1:3")]),
        // A range goes no further than its second cell, or a name.
        ("Sheet1!A1:B2:C3", &[Some("A1:B2")]),
        ("Sheet1!A1:MyName", &[Some("A1")]),
        ("Sheet1!A:C:D1", &[Some("A:C")]),
        // Whitespace between the prefix and the operand is not part of it.
        ("Sheet1!  A1", &[Some("A1")]),
        ("Sheet1! #N/A", &[Some("#N/A")]),
        ("Sheet1!A1)+1", &[Some("A1")]),
        ("SUM(Sheet1!A1:B2,1)", &[Some("A1:B2")]),
        // A union: the outer parenthesis is no operand, each prefix inside
        // has its own.
        ("(Sheet1!$A$1,Sheet1!$B$2)", &[Some("$A$1"), Some("$B$2")]),
    ]);
}

#[test]
fn a_name_a_table_reference_or_an_error_value_is_the_operand() {
    check(&[
        ("Sheet1!MyName", &[Some("MyName")]),
        ("Sheet1!MyName+1", &[Some("MyName")]),
        ("Sheet1!#N/A", &[Some("#N/A")]),
        ("Sheet1!#REF!+1", &[Some("#REF!")]),
        ("Sheet1!T[a]", &[Some("T[a]")]),
        ("Sheet1!T[[#Headers],[b]]", &[Some("T[[#Headers],[b]]")]),
        ("Sheet1![a]", &[Some("[a]")]),
    ]);
}

#[test]
fn a_call_is_the_operand_up_to_its_closing_parenthesis() {
    check(&[
        ("Sheet1!MyFn(1)", &[Some("MyFn(1)")]),
        ("Sheet1!MyFn (1)+2", &[Some("MyFn (1)")]),
        ("Sheet1!MyFn(A1,(B2))+1", &[Some("MyFn(A1,(B2))")]),
        ("Sheet1!MyFn(1)+(2)", &[Some("MyFn(1)")]),
        ("SUM(Sheet1!MyFn(1),2)", &[Some("MyFn(1)")]),
        // A `)` with no `(` before it closes nothing.
        ("Sheet1!MyFn(1))+1", &[Some("MyFn(1)")]),
        ("Sheet1!_xlfn.SINGLE(A1)", &[Some("_xlfn.SINGLE(A1)")]),
        // Excel saves `Sheet1!A2#` as `_xlfn.ANCHORARRAY(Sheet1!A2)`: the prefix
        // is inside the call.
        ("_xlfn.ANCHORARRAY(Sheet1!A2)", &[Some("A2")]),
        // A prefix inside the call has an operand of its own.
        (
            "Sheet1!MyFn(Sheet1!A1,2)+Sheet1!B2",
            &[Some("MyFn(Sheet1!A1,2)"), Some("A1"), Some("B2")],
        ),
    ]);
}

#[test]
fn a_spill_operator_right_after_the_operand_is_part_of_it() {
    check(&[
        ("Sheet1!A2#", &[Some("A2#")]),
        ("Sheet1!A2#+1", &[Some("A2#")]),
        // Only the first `#` is part of the operand; `Sheet1!A2##` is not a
        // correct formula to begin with.
        ("Sheet1!A2##", &[Some("A2#")]),
        ("Sheet1!A1:B2#", &[Some("A1:B2#")]),
        ("SUM(Sheet1!A1:B2#)", &[Some("A1:B2#")]),
        ("Sheet1!MyName#", &[Some("MyName#")]),
        ("Sheet1!T[a]#", &[Some("T[a]#")]),
        ("Sheet1!MyFn(1)#", &[Some("MyFn(1)#")]),
        (
            "Sheet1!MyFn(Sheet1!A1)#",
            &[Some("MyFn(Sheet1!A1)#"), Some("A1")],
        ),
        // Whitespace before the `#` ends the operand.
        ("Sheet1!A2 #", &[Some("A2")]),
        // A `#` that follows no operand of a prefix belongs to nothing.
        ("Sheet1!A2+B1#", &[Some("A2")]),
        ("Sheet1!#", &[None]),
    ]);
}

#[test]
fn a_prefix_with_no_operand_gives_none() {
    check(&[
        ("Sheet1!", &[None]),
        ("Sheet1!+1", &[None]),
        ("Sheet1!(A1)", &[None]),
        ("Sheet1! (A1+(B2))*2", &[None]),
        ("Sheet1!1", &[None]),
        ("Sheet1!TRUE", &[None]),
        ("Sheet1!\"x\"", &[None]),
        ("Sheet1!_xlfn.X", &[None]),
        ("Sheet1!_xlpm.x(1)", &[None]),
        ("Sheet1!MyFn(1", &[None]),
        ("Sheet1!(A1", &[None]),
        // `A1:Sheet1!` is read as a prefix of two sheets, which takes the
        // first prefix's operand.
        ("Sheet1!A1:Sheet1!B2", &[None, Some("B2")]),
        ("Sheet1!Sheet2!A1", &[None, Some("A1")]),
        // A prefix inside a parenthesis has its own operand.
        (
            "Sheet1!(Sheet1!MyFn(1)+Sheet1!A1)*2",
            &[None, Some("MyFn(1)"), Some("A1")],
        ),
    ]);
}

#[test]
fn a_prefix_of_a_quoted_sheet_a_book_or_two_sheets_has_the_operand_after_it() {
    check(&[
        ("'S 1'!A1+1", &[Some("A1")]),
        ("S1:S3!A1:B2", &[Some("A1:B2")]),
        ("[1]S!A1", &[Some("A1")]),
        ("[1]S1:S3!MyName", &[Some("MyName")]),
        (r"'C:\d\[b.xlsx]S'!A1:B2#", &[Some("A1:B2#")]),
        // A book alone is no sheet prefix: the range after it is no prefix's
        // operand, and does not extend the operand of the prefix before it.
        (r"Sheet1!A1+'C:\d\b.xlsx'!B1:C2", &[Some("A1")]),
    ]);
}
