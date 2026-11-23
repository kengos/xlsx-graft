//! `SheetCells` over small worksheets: a cell of column A given to each row
//! the sheet has, with or without a cell there, and to each row it lacks;
//! what it leaves as it is; and the sheets it refuses.

use super::SheetCells;
use xlsx_graft_cell::CellValue;
use xlsx_graft_write::{PartRewriter, WriteError};

const HEAD: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="B2:C3"/><sheetData>"#;
const TAIL: &str = r#"</sheetData><pageMargins left="0.7"/></worksheet>"#;

/// The cells `cells` writes: the cell formats 7, 8 and 9 on A1, A2 and A3.
const A1: &str = r#"<c r="A1" s="7" t="inlineStr"><is><t>a</t></is></c>"#;
const A2: &str = r#"<c r="A2" s="8"><v>1.5</v></c>"#;
const A3: &str = r#"<c r="A3" s="9" t="inlineStr"><is><t>c</t></is></c>"#;

/// A worksheet whose `sheetData` holds `rows`.
fn sheet(rows: &str) -> String {
    format!("{HEAD}{rows}{TAIL}")
}

/// What `SheetCells` writes from `xml` for the three cells.
fn rewrite_with(xml: &str, cells: Vec<(u32, CellValue<'static>)>) -> Result<String, WriteError> {
    let mut out = Vec::new();
    SheetCells::new(cells).rewrite(&mut xml.as_bytes(), &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

fn three() -> Vec<(u32, CellValue<'static>)> {
    vec![
        (7, CellValue::String("a")),
        (8, CellValue::Number(1.5)),
        (9, CellValue::String("c")),
    ]
}

fn rewrite(xml: &str) -> String {
    rewrite_with(xml, three()).unwrap()
}

/// Why `SheetCells` refuses `xml`.
fn refused(xml: &str) -> String {
    match rewrite_with(xml, three()) {
        Err(WriteError::Io(error)) => error.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn sheet_data_that_is_empty_is_opened_and_given_a_row_for_each_cell() {
    let rows = format!(r#"<row r="1">{A1}</row><row r="2">{A2}</row><row r="3">{A3}</row>"#);
    assert_eq!(rewrite(&sheet("")), sheet(&rows));
    let empty = sheet("").replace("<sheetData></sheetData>", "<sheetData/>");
    assert_eq!(rewrite(&empty), sheet(&rows));
    let prefixed = r#"<worksheet><sheetData x="1"/></worksheet>"#;
    let expected = format!(
        r#"<worksheet><sheetData x="1">{}</sheetData></worksheet>"#,
        rows
    );
    assert_eq!(rewrite(prefixed), expected);
}

#[test]
fn a_row_the_sheet_lacks_is_added_before_the_next_row_or_the_end() {
    let xml =
        sheet(r#"<row r="2" spans="2:2"><c r="B2"/></row><row r="5"><c r="A5" s="1"/></row>"#);
    let expected = sheet(&format!(
        r#"<row r="1">{A1}</row><row r="2">{A2}<c r="B2"/></row><row r="3">{A3}</row><row r="5"><c r="A5" s="1"/></row>"#
    ));
    assert_eq!(rewrite(&xml), expected);
    let xml = sheet(r#"<row r="1"><c r="A1"/></row>"#);
    let expected = sheet(&format!(
        r#"<row r="1"><c r="A1" s="7"/></row><row r="2">{A2}</row><row r="3">{A3}</row>"#
    ));
    assert_eq!(rewrite(&xml), expected);
}

#[test]
fn a_cell_of_column_a_keeps_its_value_and_has_its_style_replaced_or_added() {
    let xml = sheet(concat!(
        r#"<row r="1" spans="1:2"><c r="A1" s="4" t="s"><v>0</v></c><c r="B1"/></row>"#,
        r#"<row r="2"><c r="A2" t="str"><f>1+1</f><v>2</v></c></row>"#,
        r#"<row r="3"><c r="a3" s="5"/></row>"#,
    ));
    let expected = sheet(concat!(
        r#"<row r="1" spans="1:2"><c r="A1" t="s" s="7"><v>0</v></c><c r="B1"/></row>"#,
        r#"<row r="2"><c r="A2" t="str" s="8"><f>1+1</f><v>2</v></c></row>"#,
        r#"<row r="3"><c r="a3" s="9"/></row>"#,
    ));
    assert_eq!(rewrite(&xml), expected);
}

#[test]
fn a_row_without_a_cell_of_column_a_gets_one_first_and_loses_its_spans() {
    let xml = sheet(concat!(
        r#"<row r="1" spans="2:3" ht="20"><c r="B1"><v>5</v></c></row>"#,
        r#"<row r="2" spans="2:3"/>"#,
        r#"<row r="3" spans="2:3"></row>"#,
    ));
    let expected = sheet(&format!(
        concat!(
            r#"<row r="1" ht="20">{}<c r="B1"><v>5</v></c></row>"#,
            r#"<row r="2">{}</row><row r="3">{}</row>"#,
        ),
        A1, A2, A3
    ));
    assert_eq!(rewrite(&xml), expected);
}

#[test]
fn what_is_in_a_row_before_its_first_element_stays_in_its_place() {
    let xml = sheet(
        "<row r=\"1\"> <!--x--> <c r=\"A1\"/></row><row r=\"2\"> <!--y--> <c r=\"B2\"/></row><row r=\"3\"><extLst/></row>",
    );
    let expected = sheet(&format!(
        "<row r=\"1\"> <!--x--> {}</row><row r=\"2\"> <!--y--> {}<c r=\"B2\"/></row><row r=\"3\">{}<extLst/></row>",
        r#"<c r="A1" s="7"/>"#, A2, A3
    ));
    assert_eq!(rewrite(&xml), expected);
}

#[test]
fn the_children_of_a_row_are_written_as_they_are_and_the_row_ends_at_its_own_end() {
    let xml = sheet(concat!(
        r#"<row r="1"><c r="A1" t="s"><v>0</v></c><extLst><ext><x/></ext></extLst></row>"#,
        r#"<row r="2"><c r="B2"/><extLst><ext/></extLst></row>"#,
        r#"<row r="3"/>"#,
    ));
    let expected = sheet(&format!(
        concat!(
            r#"<row r="1"><c r="A1" t="s" s="7"><v>0</v></c><extLst><ext><x/></ext></extLst></row>"#,
            r#"<row r="2">{}<c r="B2"/><extLst><ext/></extLst></row><row r="3">{}</row>"#,
        ),
        A2, A3
    ));
    assert_eq!(rewrite(&xml), expected);
}

#[test]
fn rows_after_the_last_cell_and_what_is_outside_sheet_data_are_not_touched() {
    let cells = vec![(7, CellValue::String("a"))];
    let rest = r#"<row r="2" spans="1:2"><c r="B2"/></row> <row r="9"/><!--z-->"#;
    let xml = format!(
        "<worksheet>\n <sheetData>{}{rest}</sheetData><x/></worksheet>",
        r#"<row r="1"/>"#
    );
    let expected = format!(
        "<worksheet>\n <sheetData><row r=\"1\">{A1}</row>{rest}</sheetData><x/></worksheet>"
    );
    assert_eq!(rewrite_with(&xml, cells).unwrap(), expected);
    let chart = r#"<chartsheet><drawing r:id="rId1"/></chartsheet>"#;
    assert_eq!(rewrite(chart), chart);
}

#[test]
fn a_row_or_a_first_cell_without_an_r_that_parses_is_refused() {
    let row = r#"<sheetData><row><c r="A1"/></row></sheetData>"#;
    assert_eq!(refused(row), "a <row> has no r attribute");
    let cell = sheet(r#"<row r="1"><c><v>1</v></c></row>"#);
    assert_eq!(refused(&cell), "a <c> has no r attribute");
    let words = sheet(r#"<row r="one"/>"#);
    assert!(refused(&words).contains("invalid digit"));
    let reference = sheet(r#"<row r="1"><c r="$A$1"/></row>"#);
    assert!(refused(&reference).contains("not in the form of the value"));
}

#[test]
fn a_sheet_data_with_a_namespace_prefix_is_refused() {
    let xml = r#"<x:worksheet xmlns:x="urn:x"><x:sheetData/></x:worksheet>"#;
    assert_eq!(refused(xml), "sheetData has a namespace prefix");
    let xml = r#"<x:worksheet xmlns:x="urn:x"><x:sheetData></x:sheetData></x:worksheet>"#;
    assert_eq!(refused(xml), "sheetData has a namespace prefix");
}

#[test]
fn a_value_a_cell_cannot_hold_is_refused() {
    let nan = vec![(7, CellValue::Number(f64::NAN))];
    for xml in [
        sheet(""),
        sheet(r#"<row r="1"/>"#),
        sheet(r#"<row r="1"></row>"#),
        sheet(r#"<row r="2"/>"#),
    ] {
        let found = rewrite_with(&xml, nan.clone());
        assert!(matches!(found, Err(WriteError::Io(_))), "{xml}");
    }
}

#[test]
fn xml_that_is_not_well_formed_is_an_invalid_part() {
    let found = rewrite_with("<worksheet><sheetData>", three());
    assert!(matches!(found, Err(WriteError::Invalid(_))), "{found:?}");
}

#[test]
fn an_element_in_sheet_data_that_is_not_a_row_is_written_as_it_is() {
    let xml = sheet(r#"<foo></foo><foo/><row r="1"/>"#);
    let expected = sheet(&format!(
        r#"<foo></foo><foo/><row r="1">{A1}</row><row r="2">{A2}</row><row r="3">{A3}</row>"#
    ));
    assert_eq!(rewrite(&xml), expected);
}

#[test]
fn a_row_nested_in_the_children_of_a_row_is_written_as_it_is() {
    let nested = "<a><b></b><row/></a>";
    let xml = sheet(&format!(
        r#"<row r="1"><c r="B1">{nested}</c></row><row r="2"><c r="A2">{nested}</c></row>"#
    ));
    let expected = sheet(&format!(
        r#"<row r="1">{A1}<c r="B1">{nested}</c></row><row r="2"><c r="A2" s="8">{nested}</c></row><row r="3">{A3}</row>"#
    ));
    assert_eq!(rewrite(&xml), expected);
}
