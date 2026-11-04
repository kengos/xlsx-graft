//! What a sheet's cells, rows and columns refer to: a shared string, a
//! cell format, and the length of an inline string.

use super::{COUNTS, check, check_with, rows, worksheet};
use crate::sheet::Counts;
use xlsx_graft_diagnostics::{Diagnostic, Echo};

/// `cell_style_missing` at `reference`, of the cell format `index`.
fn cell_style(reference: &str, index: u64) -> Diagnostic {
    Diagnostic::CellStyleMissing {
        reference: Echo::new(reference),
        index,
        count: 7,
    }
}

/// `inline_string_too_long` at `reference`, `length` units long.
fn too_long(reference: &str, length: u64) -> Diagnostic {
    Diagnostic::InlineStringTooLong {
        reference: Echo::new(reference),
        length,
    }
}

#[test]
fn a_shared_string_past_the_table_is_refused() {
    // Excel for Mac 16.114.3 offers to repair it, dropping the cell.
    let xml = rows(
        r#"<row r="1"><c r="A1" t="s"><v> 35 </v></c><c r="B1" t="s"><v>3<![CDATA[6]]></v></c>
        <c r="C1" t="s"><v>&#x33;7</v></c><c r="D1" t="s"><v>99999999999999999999999</v></c></row>"#,
    );
    let missing = |index| Diagnostic::SharedStringMissing { index };
    assert_eq!(check(&xml), [missing(36), missing(37), missing(u64::MAX)]);
    // A workbook without the table holds no shared string.
    let none = Counts {
        strings: Some(0),
        ..COUNTS
    };
    let xml = rows(r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#);
    assert_eq!(check_with(&xml, none, 100), [missing(0)]);
}

#[test]
fn a_shared_string_is_not_checked_unless_its_index_and_the_table_are_known() {
    // Not a number: what that is, is not this check's to say.
    let xml = rows(
        r#"<row r="1"><c r="A1" t="s"><v>3 6</v></c><c r="B1" t="s"><v>x</v></c>
        <c r="C1" t="s"><v/></c><c r="D1" t="s"/><c r="E1" t="n"><v>99</v></c><c r="F1"><v>99</v></c><c r="G1" t="s"><v>3</v><v>6</v></c></row>"#,
    );
    assert_eq!(check(&xml), []);
    let unknown = Counts {
        strings: None,
        ..COUNTS
    };
    let xml = rows(r#"<row r="1"><c r="A1" t="s"><v>99</v></c></row>"#);
    assert_eq!(check_with(&xml, unknown, 100), []);
}

#[test]
fn a_cell_format_past_cell_xfs_is_refused_for_a_cell_and_a_column() {
    // Excel for Mac 16.114.3 offers to repair both, dropping the format.
    let xml = worksheet(
        r#"<cols><col min="3" max="5" style="7"/><col min="0" max="20000" style="99"/><col style="8"/></cols>
        <sheetData><row r="2"><c r="B2" s="7"/><c s=" 99 "/><c r="ZZZZ2" s="7"/></row>
        <row r="1"><c s="8"/></row></sheetData>"#,
    );
    let column = |columns: &str, index| Diagnostic::ColumnStyleMissing {
        columns: Echo::new(columns),
        index,
        count: 7,
    };
    let expected = [
        column("C:E", 7),
        column("0:20000", 99),
        column(":", 8),
        cell_style("B2", 7),
        cell_style("C2", 99),
        Diagnostic::CellReferenceInvalid {
            reference: Echo::new("ZZZZ2"),
        },
        cell_style("ZZZZ2", 7),
        Diagnostic::CellOrderInvalid {
            reference: Echo::new("1"),
            previous: Echo::new("2"),
        },
        cell_style("", 8),
    ];
    assert_eq!(check(&xml), expected);
}

#[test]
fn a_row_format_past_cell_xfs_is_a_notice() {
    // Excel for Mac 16.114.3 drops it without a word. Without
    // `customFormat`, a row's `s` is not in force, and not checked.
    let xml = rows(
        r#"<row r="4" s="99" customFormat="1"/><row s="7" customFormat="true"/>
        <row r="1" s="7" customFormat=" 1 "/><row r="9" s="x" customFormat="1"/>
        <row r="10" s="99"/><row r="11" s="99" customFormat="0"/>
        <row r="1048576"/><row s="7" customFormat="1"/>"#,
    );
    let row = |row: &str, index| Diagnostic::RowStyleMissing {
        row: Echo::new(row),
        index,
        count: 7,
    };
    let expected = [
        row("4", 99),
        row("5", 7),
        Diagnostic::CellOrderInvalid {
            reference: Echo::new("1"),
            previous: Echo::new("5"),
        },
        row("1", 7),
        Diagnostic::CellReferenceInvalid {
            reference: Echo::new("1048577"),
        },
        // Refused without `r`, the row is named as it would be.
        row("1048577", 7),
    ];
    assert_eq!(check(&xml), expected);
}

#[test]
fn a_cell_format_is_not_checked_when_the_styles_are_not_known() {
    let unknown = Counts {
        styles: None,
        ..COUNTS
    };
    let xml = worksheet(
        r#"<cols><col min="1" max="1" style="99"/></cols><sheetData><row r="1" s="99"><c r="A1" s="99"/></row></sheetData>"#,
    );
    assert_eq!(check_with(&xml, unknown, 100), []);
}

#[test]
fn an_inline_string_is_up_to_32767_utf16_code_units() {
    // Excel for Mac 16.114.3 offers to repair 32,768, cutting it, and
    // takes 32,767 as it is.
    let cell = |reference: &str, text: &str| {
        format!(r#"<c r="{reference}" t="inlineStr"><is><t>{text}</t></is></c>"#)
    };
    let xml = rows(&format!(
        r#"<row r="1">{}{}{}{}</row>"#,
        cell("A1", &"a".repeat(32_767)),
        cell("B1", &"a".repeat(32_768)),
        cell("C1", &"😀".repeat(16_384)),
        cell("D1", &"😀".repeat(16_383)),
    ));
    assert_eq!(
        check(&xml),
        [too_long("B1", 32_768), too_long("C1", 32_768)]
    );
}

#[test]
fn an_inline_string_counts_its_runs_and_its_escapes_undone() {
    // The runs count together; the phonetic runs are not the text. An
    // escape is one unit, and text in pieces counts whole.
    let half = "a".repeat(16_384);
    let escapes = "_x0041_".repeat(32_767);
    let xml = rows(&format!(
        r#"<row r="1"><c r="A1" t="inlineStr"><is><r><t>{half}</t></r><r><rPr/><t>{half}</t></r><rPh><t>{half}</t></rPh></is></c>
        <c r="B1" t="inlineStr"><is><t>{escapes}</t><rPh><t>{half}</t></rPh></is></c>
        <c r="C1" t="inlineStr"><is><t>{half}&amp;<![CDATA[{half}]]></t></is></c></row>"#
    ));
    assert_eq!(
        check(&xml),
        [too_long("A1", 32_768), too_long("C1", 32_769)]
    );
}

#[test]
fn a_string_that_is_no_inline_string_is_not_checked() {
    // Excel for Mac 16.114.3 keeps a formula's cached string of 32,768
    // whole.
    let long = "a".repeat(32_768);
    let xml = rows(&format!(
        r#"<row r="1"><c r="A1" t="str"><f>REPT("a",32768)</f><v>{long}</v></c>
        <c r="B1" t="s"><is><t>{long}</t></is><v>0</v></c><c r="C1"><is><t>{long}</t></is></c></row>"#
    ));
    assert_eq!(check(&xml), []);
}
