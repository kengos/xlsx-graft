//! Tests for what Excel repairs, through `verify`: the checks of the
//! package, the workbook and its tables, each worksheet and the shared
//! strings table, run together on one workbook (here), and what the
//! sheets' checks are handed from the parts read before them, the cell
//! formats and the tables' order (`repairs/tables.rs`). What Excel does
//! with each problem is Excel for Mac 16.114.3's, opening the workbook.

mod tables;

use super::zip::{self, Item};
use super::{Book, WORKSHEET, check, run};
use crate::{Limits, Rules};
use xlsx_graft_diagnostics::{Diagnostic, Echo, Limit};

/// The Transitional SpreadsheetML namespace.
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// The Transitional relationships namespace, of an `r:id`.
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// A table's media type.
const TABLE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";

/// A workbook part whose `sheets` hold `sheets` and whose `definedNames`
/// hold `names`.
fn workbook(sheets: &str, names: &str) -> String {
    format!(
        r#"<workbook xmlns="{MAIN}" xmlns:r="{REL}"><sheets>{sheets}</sheets><definedNames>{names}</definedNames></workbook>"#
    )
}

/// A worksheet holding `body`.
fn worksheet(body: &str) -> String {
    format!(r#"<worksheet xmlns="{MAIN}" xmlns:r="{REL}">{body}</worksheet>"#)
}

/// The code of each of `seen`, with where it points.
fn codes(seen: &[(Diagnostic, String)]) -> Vec<(&'static str, &str)> {
    (seen.iter())
        .map(|(diagnostic, place)| (diagnostic.code().as_str(), place.as_str()))
        .collect()
}

/// A workbook damaged in every way the checks find, but for the limits,
/// a part without a media type added as it is zipped.
fn damaged() -> Book {
    let sheets = r#"<sheet name="One" sheetId="1" r:id="rId1"/><sheet name="ONE" sheetId="2" r:id="rId2"/><sheet name="Bad[" sheetId="2" r:id="rId3"/><sheet name="Gone" sheetId="4" r:id="rId9"/>"#;
    let names = r#"<definedName name="1x">One!$A$1</definedName><definedName name="Far" localSheetId="9">One!$A$1</definedName>"#;
    let long = "a".repeat(32_768);
    let first = worksheet(&format!(
        r#"<sheetData><row r="2"><c r="B2" t="s"><v>5</v></c><c r="A2" s="9"/><c r="C2" t="inlineStr"><is><t>{long}</t></is></c></row><row r="1"/></sheetData><mergeCells><mergeCell ref="A1:B2"/><mergeCell ref="B2:C3"/></mergeCells>"#
    ));
    let table = |id: u32, name: &str, range: &str| {
        format!(
            r#"<table xmlns="{MAIN}" id="{id}" name="{name}" displayName="{name}" ref="{range}"/>"#
        )
    };
    let third =
        worksheet(r#"<tableParts><tablePart r:id="rId1"/><tablePart r:id="rId2"/></tableParts>"#);
    let strings = format!(r#"<sst xmlns="{MAIN}"><si><t>{long}</t></si><si><t>b</t></si></sst>"#);
    let styles = format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs><xf/></cellXfs></styleSheet>"#);
    let wb = "/xl/workbook.xml";
    let sheet = |n: u32| format!("/xl/worksheets/sheet{n}.xml");
    (Book::new().main("application/xml", &workbook(sheets, names)))
        .part(&sheet(1), WORKSHEET, &first)
        .part(
            &sheet(2),
            WORKSHEET,
            &worksheet("<sheetData>&#1;</sheetData>"),
        )
        .part(&sheet(3), WORKSHEET, &third)
        .part("/xl/tables/table1.xml", TABLE, &table(1, "T1", "A1:B2"))
        .part("/xl/tables/table2.xml", TABLE, &table(1, "t1", "B2:C3"))
        .part("/xl/sharedStrings.xml", "application/xml", &strings)
        .part("/xl/styles.xml", "application/xml", &styles)
        .rel(wb, "worksheet", &sheet(1))
        .rel(wb, "worksheet", &sheet(2))
        .rel(wb, "worksheet", &sheet(3))
        .rel(wb, "sharedStrings", "/xl/sharedStrings.xml")
        .rel(wb, "styles", "/xl/styles.xml")
        .rel(wb, "worksheet", "/xl/worksheets/missing.xml")
        .rel(&sheet(3), "table", "/xl/tables/table1.xml")
        .rel(&sheet(3), "table", "/xl/tables/table2.xml")
}

#[test]
fn every_problem_of_a_workbook_is_found_each_once() {
    let mut items = damaged().items();
    items.push(Item::stored("xl/extra.bin", b"x"));
    let found = run(
        zip::build(items),
        &Limits::default(),
        &Rules::default(),
        (0, 0),
    )
    .unwrap();
    let (wb, s1) = ("/xl/workbook.xml", "/xl/worksheets/sheet1.xml");
    let table2 = "/xl/tables/table2.xml";
    assert_eq!(
        codes(&found),
        [
            // The package: Excel for Mac asks to repair the first two.
            ("part_content_type_missing", "/xl/extra.bin"),
            (
                "relationship_target_missing",
                "/xl/_rels/workbook.xml.rels#rId6"
            ),
            ("part_unreferenced", "/xl/extra.bin"),
            // The workbook part's sheets, as the workbook reader reads
            // them. Gone's `r:id` names no relationship, but may name the
            // one left out for its missing target, so it is not reported
            // again.
            ("sheet_name_duplicate", wb),
            // The shared strings table, read before the sheets.
            ("shared_string_too_long", "/xl/sharedStrings.xml"),
            // The first sheet, read as a stream: every problem, not the
            // first alone.
            ("shared_string_missing", s1),
            ("cell_order_invalid", s1),
            ("cell_style_missing", s1),
            ("inline_string_too_long", s1),
            ("cell_order_invalid", s1),
            ("merged_cells_overlap", s1),
            // The second, which stops being read where XML stops.
            ("xml_malformed", "/xl/worksheets/sheet2.xml"),
            // The workbook's sheets, names and tables.
            ("sheet_name_invalid", wb),
            ("sheet_id_duplicate", wb),
            ("defined_name_invalid", wb),
            ("defined_name_sheet_index_invalid", wb),
            ("table_range_overlap", table2),
            ("table_name_duplicate", table2),
            ("table_id_duplicate", table2),
        ]
    );
    assert_eq!(
        found[5].0,
        Diagnostic::SharedStringMissing { index: 5 },
        "the table holds 2"
    );
    assert_eq!(
        found[7].0,
        Diagnostic::CellStyleMissing {
            reference: Echo::new("A2"),
            index: 9,
            count: 1,
        }
    );
}

#[test]
fn a_sheet_past_the_merged_cells_limit_is_refused() {
    let merges = r#"<mergeCells><mergeCell ref="A1:B2"/><mergeCell ref="C1:D2"/></mergeCells>"#;
    let sheet = "/xl/worksheets/sheet1.xml";
    let book = (Book::new().main(
        "application/xml",
        &workbook(r#"<sheet name="S" sheetId="1" r:id="rId1"/>"#, ""),
    ))
    .part(sheet, WORKSHEET, &worksheet(merges))
    .rel("/xl/workbook.xml", "worksheet", sheet);
    let limits = |max| Limits {
        max_merged_cells: Limit::default_of(max),
        ..Limits::default()
    };
    let zip = book.zip();
    let at = |max| run(zip.clone(), &limits(max), &Rules::default(), (0, 0)).unwrap();
    assert_eq!(at(2), []);
    assert_eq!(
        at(1),
        [(
            Diagnostic::MergedCellsTooMany {
                count: 2,
                limit: Limit::default_of(1),
            },
            sheet.to_owned(),
        )]
    );
}

#[test]
fn a_workbook_part_that_breaks_its_schema_has_no_sheets_to_check() {
    // No `sheets`: the workbook reader refuses the part, and neither its
    // names nor any part as a sheet is checked.
    let names = r#"<definedNames><definedName name="1x">1</definedName></definedNames>"#;
    let xml = format!(r#"<workbook xmlns="{MAIN}">{names}</workbook>"#);
    let sheet = "/xl/worksheets/sheet1.xml";
    let rows = r#"<sheetData><row r="2"/><row r="1"/></sheetData>"#;
    let book = (Book::new().main("application/xml", &xml))
        .part(sheet, WORKSHEET, &worksheet(rows))
        .rel("/xl/workbook.xml", "worksheet", sheet);
    assert_eq!(check(book), []);
}

#[test]
fn a_part_the_sheets_do_not_name_is_not_checked_as_a_worksheet() {
    let sheet = "/xl/worksheets/sheet1.xml";
    let rows = r#"<sheetData><row r="2"/><row r="1"/></sheetData>"#;
    let named = |sheets: &str| {
        (Book::new().main("application/xml", &workbook(sheets, "")))
            .part(sheet, WORKSHEET, &worksheet(rows))
            .rel("/xl/workbook.xml", "worksheet", sheet)
    };
    let found = check(named(r#"<sheet name="S" sheetId="1" r:id="rId9"/>"#));
    assert_eq!(
        codes(&found),
        [("sheet_relationship_missing", "/xl/workbook.xml")]
    );
    let found = check(named(r#"<sheet name="S" sheetId="1" r:id="rId1"/>"#));
    assert_eq!(codes(&found), [("cell_order_invalid", sheet)]);
}

#[test]
fn a_worksheet_s_text_counts_however_it_is_written() {
    let a = "a".repeat(32_766);
    let sheet = "/xl/worksheets/sheet1.xml";
    let rows = format!(
        r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t><![CDATA[{a}]]>&amp;&#97;</t></is></c></row></sheetData>"#
    );
    let book = (Book::new().main(
        "application/xml",
        &workbook(r#"<sheet name="S" sheetId="1" r:id="rId1"/>"#, ""),
    ))
    .part(sheet, WORKSHEET, &worksheet(&rows))
    .rel("/xl/workbook.xml", "worksheet", sheet);
    assert_eq!(
        check(book),
        [(
            Diagnostic::InlineStringTooLong {
                reference: Echo::new("A1"),
                length: 32_768,
            },
            sheet.to_owned(),
        )]
    );
}
