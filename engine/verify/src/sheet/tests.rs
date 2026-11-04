//! Tests for the sheet checks, driven through [`SheetCheck`] on worksheet
//! XML read as `verify` reads a part. `places.rs` covers where rows and
//! cells stand, `values.rs` the shared strings, styles and inline strings
//! cells refer to, `merges.rs` the merged cells, and this file what the
//! checks read at all.
//!
//! What Excel for Mac 16.114.3 did as it opened a workbook with each
//! problem is said beside the test that holds it.

mod merges;
mod places;
mod values;

use super::{Counts, SheetCheck};
use crate::parts::resolve;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Limit};
use xlsx_graft_xml::events::Event;
use xlsx_graft_xml::{XmlLimits, XmlStreamReader};

/// The Transitional SpreadsheetML namespace.
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// 36 shared strings and 7 cell formats, as in the workbook Excel opened.
const COUNTS: Counts = Counts {
    strings: Some(36),
    styles: Some(7),
};

/// A worksheet in the Transitional namespace holding `body`.
fn worksheet(body: &str) -> String {
    format!(r#"<worksheet xmlns="{MAIN}">{body}</worksheet>"#)
}

/// A worksheet whose `sheetData` holds `rows`.
fn rows(rows: &str) -> String {
    worksheet(&format!("<sheetData>{rows}</sheetData>"))
}

/// What the checks find in `xml` against `counts`, keeping `merged`
/// merged ranges.
fn check_with(xml: &str, counts: Counts, merged: u64) -> Vec<Diagnostic> {
    let mut diagnostics = Diagnostics::new();
    let mut check = SheetCheck::new(
        "/xl/worksheets/sheet1.xml",
        counts,
        Limit::default_of(merged),
        &mut diagnostics,
    );
    let mut xml = XmlStreamReader::new(xml.as_bytes(), &XmlLimits::default());
    let mut buf = Vec::new();
    loop {
        let event = xml.read_event_into(&mut buf).unwrap();
        let opened = matches!(event, Event::Start(_));
        match event {
            Event::Start(start) | Event::Empty(start) => {
                let (resolved, local) = xml.resolve_element(start.name());
                check.start(&resolved, local.as_ref(), &start, opened);
            }
            Event::End(_) => check.end(),
            Event::Text(text) => check.text(|| text.xml10_content()),
            Event::CData(data) => check.text(|| data.xml10_content()),
            Event::GeneralRef(reference) => check.text(|| resolve(&reference).into()),
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    check.finish();
    let records = diagnostics.finish();
    for record in &records {
        assert_eq!(
            record.location.part,
            Some(Echo::new("/xl/worksheets/sheet1.xml"))
        );
    }
    records
        .into_iter()
        .map(|record| record.diagnostic)
        .collect()
}

/// What the checks find in `xml` against [`COUNTS`].
fn check(xml: &str) -> Vec<Diagnostic> {
    check_with(xml, COUNTS, 100)
}

/// `cell_reference_invalid` at `reference`.
fn invalid(reference: &str) -> Diagnostic {
    Diagnostic::CellReferenceInvalid {
        reference: Echo::new(reference),
    }
}

/// `cell_order_invalid` at `reference`, after `previous`.
fn order(reference: &str, previous: &str) -> Diagnostic {
    Diagnostic::CellOrderInvalid {
        reference: Echo::new(reference),
        previous: Echo::new(previous),
    }
}

#[test]
fn a_sheet_excel_opens_as_it_is_raises_nothing() {
    // Each at the edge Excel took without a word: the last row and
    // column, the last cell format and shared string, an inline string of
    // 32,767 units, and merged ranges that touch at an edge and a corner.
    let long = "a".repeat(32_767);
    let xml = worksheet(&format!(
        r#"<dimension ref="A1"/><cols><col min="1" max="2" style="6"/></cols>
        <sheetData>
          <row r="1" s="6" customFormat="1"><c r="A1" s="6" t="s"><v>35</v></c><c t="inlineStr"><is><t>{long}</t></is></c></row>
          <row><c r="XFD2"><v>1</v></c></row>
          <row r="1048576"><c r="A1048576"><v>1</v></c></row>
        </sheetData>
        <mergeCells count="3"><mergeCell ref="G1:H2"/><mergeCell ref="I3:J4"/><mergeCell ref="I1:J2"/></mergeCells>"#
    ));
    assert_eq!(check(&xml), []);
}

#[test]
fn every_problem_is_found_not_only_the_first() {
    let xml = worksheet(
        r#"<sheetData>
          <row r="2"><c r="B2" s="99"><v>1</v></c><c r="A2"/><c r="C2" t="s"><v>36</v></c></row>
          <row r="1"><c r="A1"/></row>
          <row r="1048577"/>
        </sheetData>
        <mergeCells><mergeCell ref="G1:H2"/><mergeCell ref="H2:I3"/></mergeCells>"#,
    );
    let expected = [
        Diagnostic::CellStyleMissing {
            reference: Echo::new("B2"),
            index: 99,
            count: 7,
        },
        order("A2", "B2"),
        Diagnostic::SharedStringMissing { index: 36 },
        order("1", "2"),
        invalid("1048577"),
        Diagnostic::MergedCellsOverlap {
            reference: Echo::new("H2:I3"),
            other: Echo::new("G1:H2"),
        },
    ];
    assert_eq!(check(&xml), expected);
}

#[test]
fn the_strict_namespace_is_read_as_the_transitional_one_is() {
    let xml = r#"<x:worksheet xmlns:x="http://purl.oclc.org/ooxml/spreadsheetml/main">
        <x:sheetData><x:row r="2"/><x:row r="1"/></x:sheetData></x:worksheet>"#;
    assert_eq!(check(xml), [order("1", "2")]);
}

#[test]
fn what_is_not_where_the_schema_puts_it_is_passed_over() {
    // Another namespace, an extension list, a cell's formula and a row
    // outside `sheetData` hold nothing the checks read; neither does a
    // root that is no worksheet.
    let xml = worksheet(
        r#"<sheetData><row r="2"><o:c xmlns:o="urn:other" r="A1" s="99"/>
          <c r="B2" t="s"><f>36</f><v>0</v></c></row>
          <row r="3"><c r="A3"><extLst><row r="1"/></extLst></c></row></sheetData>
        <extLst><ext><sheetData><row r="1"/></sheetData></ext></extLst>
        <row r="1"/><mergeCell ref="A1:B2"/><mergeCell ref="A1:B2"/>
        <cols><col min="1" max="1" style="99"><extLst/></col></cols>"#,
    );
    assert_eq!(
        check(&xml),
        [Diagnostic::ColumnStyleMissing {
            columns: Echo::new("A:A"),
            index: 99,
            count: 7
        }]
    );
    let root = format!(
        r#"<chartsheet xmlns="{MAIN}"><sheetData><row r="2"/><row r="1"/></sheetData></chartsheet>"#
    );
    assert_eq!(check(&root), []);
    let foreign = r#"<worksheet xmlns="urn:other"><sheetData><row r="2"/><row r="1"/></sheetData></worksheet>"#;
    assert_eq!(check(foreign), []);
    let unbound = "<worksheet><sheetData><row r=\"2\"/><row r=\"1\"/></sheetData></worksheet>";
    assert_eq!(check(unbound), []);
}

#[test]
fn attributes_are_read_unprefixed() {
    let xml = rows(r#"<row xmlns:z="urn:z" z:r="5" r="1"><c z:s="99" s="1"/></row>"#);
    assert_eq!(check(&xml), []);
}
