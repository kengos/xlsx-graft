//! What the checks are handed from the parts read before the sheets: the
//! cell formats a cell's `s` is checked against, and the tables, in the
//! order a sheet's `tableParts` names them.

use super::{MAIN, TABLE, codes, workbook, worksheet};
use crate::tests::zip::Item;
use crate::tests::{Book, WORKSHEET, check, run};
use crate::{Limits, Rules, VerifyError};
use xlsx_graft_diagnostics::{Diagnostic, Echo, Limit};

/// A workbook of one sheet holding `body`, whose sheet relates to a table
/// part for each of `tables` (its `displayName`, `id` and `ref`, each
/// written as an attribute when given).
fn with_tables(body: &str, tables: &[(&str, &str, &str)]) -> Book {
    let sheet = "/xl/worksheets/sheet1.xml";
    let mut book = (Book::new().main(
        "application/xml",
        &workbook(r#"<sheet name="S" sheetId="1" r:id="rId1"/>"#, ""),
    ))
    .part(sheet, WORKSHEET, &worksheet(body))
    .rel("/xl/workbook.xml", "worksheet", sheet);
    for (n, (name, id, range)) in tables.iter().enumerate() {
        let attribute = |key: &str, value: &str| {
            if value.is_empty() {
                String::new()
            } else {
                format!(r#" {key}="{value}""#)
            }
        };
        let xml = format!(
            r#"<table xmlns="{MAIN}"{}{}{}/>"#,
            attribute("displayName", name),
            attribute("id", id),
            attribute("ref", range),
        );
        let part = format!("/xl/tables/table{}.xml", n + 1);
        book = (book.part(&part, TABLE, &xml)).rel(sheet, "table", &part);
    }
    book
}

/// `table_name_duplicate` of `name` at the part of table `n`, after `other`.
fn duplicate(n: u32, name: &str, other: &str) -> (Diagnostic, String) {
    (
        Diagnostic::TableNameDuplicate {
            name: Echo::new(name),
            other: Echo::new(other),
            defined_name: false,
        },
        format!("/xl/tables/table{n}.xml"),
    )
}

#[test]
fn tables_are_checked_in_the_order_the_table_parts_name_them() {
    let tables = [("T", "1", "A1:A2"), ("t", "2", "C1:C2")];
    let parts = |ids: &str| {
        let parts: String = (ids.split(' '))
            .map(|id| format!(r#"<tablePart r:id="{id}"/>"#))
            .collect();
        with_tables(&format!("<tableParts>{parts}</tableParts>"), &tables)
    };
    // Excel for Mac renames the later of two tables of one name.
    assert_eq!(check(parts("rId1 rId2")), [duplicate(2, "t", "T")]);
    assert_eq!(check(parts("rId2 rId1")), [duplicate(1, "T", "t")]);
    // One the `tableParts` does not name comes after those it does, in
    // the order of the parts.
    assert_eq!(check(parts("rId2")), [duplicate(1, "T", "t")]);
    assert_eq!(check(with_tables("", &tables)), [duplicate(2, "t", "T")]);
}

#[test]
fn the_tables_of_a_sheet_not_read_come_in_the_order_of_the_parts() {
    // The sheet relates to table 2 first; its part cannot be read, so its
    // `tableParts` names neither.
    let sheet = "/xl/worksheets/sheet1.xml";
    let table = |id: u32, name: &str| {
        format!(r#"<table xmlns="{MAIN}" id="{id}" displayName="{name}" ref="A1:A2"/>"#)
    };
    let book = (with_tables("", &[]))
        .part("/xl/tables/table1.xml", TABLE, &table(1, "T"))
        .part("/xl/tables/table2.xml", TABLE, &table(2, "t"))
        .rel(sheet, "table", "/xl/tables/table2.xml")
        .rel(sheet, "table", "/xl/tables/table1.xml")
        .replace(Item::stored(&sheet[1..], b"<worksheet/>").with_method(99));
    let found = check(book);
    assert_eq!(
        codes(&found),
        [
            ("zip_unsupported_compression", &sheet[1..]),
            ("table_range_overlap", "/xl/tables/table2.xml"),
            ("table_name_duplicate", "/xl/tables/table2.xml"),
        ]
    );
}

#[test]
fn a_table_without_what_is_checked_is_not_checked() {
    let none = [
        ("T", "1", "A1:A2"),
        ("T", "", "A1:A2"),
        ("T", "x", "A1:A2"),
        ("T", "4294967296", "A1:A2"),
        ("", "1", "A1:A2"),
        ("T", "1", ""),
    ];
    assert_eq!(check(with_tables("", &none)), []);
}

#[test]
fn a_table_two_sheets_relate_to_is_no_one_sheets() {
    // Two overlapping tables, one of them also related from a second
    // sheet: no sheet holds both, so they do not overlap; their ids are
    // still the workbook's.
    let tables = [("T", "1", "A1:B2"), ("U", "1", "B2:C3")];
    let second = "/xl/worksheets/sheet2.xml";
    let sheets =
        r#"<sheet name="S" sheetId="1" r:id="rId1"/><sheet name="R" sheetId="2" r:id="rId2"/>"#;
    let book = (with_tables("", &tables).main("application/xml", &workbook(sheets, "")))
        .part(second, WORKSHEET, &worksheet(""))
        .rel("/xl/workbook.xml", "worksheet", second)
        .rel(second, "table", "/xl/tables/table1.xml");
    let found = check(book);
    assert_eq!(
        codes(&found),
        [("table_id_duplicate", "/xl/tables/table1.xml")]
    );
}

/// A workbook whose sheet's `A1` has the format `s`, and whose styles
/// part, if any, is `styles`.
fn with_style(s: u32, styles: Option<&str>) -> Book {
    let rows = format!(r#"<sheetData><row r="1"><c r="A1" s="{s}"/></row></sheetData>"#);
    let book = with_tables(&rows, &[]);
    match styles {
        Some(xml) => book.related("/xl/styles.xml", xml),
        None => book,
    }
}

#[test]
fn a_cell_format_is_checked_against_the_styles_cell_formats() {
    let two = format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs><xf/><xf/></cellXfs></styleSheet>"#);
    assert_eq!(check(with_style(1, Some(&two))), []);
    // Excel for Mac asks to repair a cell whose format is not there.
    let found = check(with_style(2, Some(&two)));
    assert_eq!(
        found[0].0,
        Diagnostic::CellStyleMissing {
            reference: Echo::new("A1"),
            index: 2,
            count: 2,
        }
    );
    // No styles part, or one the styles reader refuses: not checked.
    assert_eq!(check(with_style(2, None)), []);
    let refused = format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs/></styleSheet>"#);
    assert_eq!(check(with_style(2, Some(&refused))), []);
}

#[test]
fn a_failure_of_the_host_while_reading_the_styles_is_an_error() {
    let xml = format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs><xf/>MARK</cellXfs></styleSheet>"#);
    let zip = with_style(0, Some(&xml)).zip();
    let mark = zip.windows(4).position(|w| w == b"MARK").unwrap() as u64;
    let start = mark - format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs><xf/>"#).len() as u64;
    let error = run(
        zip,
        &Limits::default(),
        &Rules::default(),
        (start, start + 1),
    )
    .unwrap_err();
    assert!(matches!(error, VerifyError::Package(_)), "{error:?}");
}

#[test]
fn styles_too_large_to_read_are_reported_and_not_checked_against() {
    let xfs = "<xf/>".repeat(400);
    let styles = format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs>{xfs}</cellXfs></styleSheet>"#);
    let mut limits = Limits::default();
    // Room for every other part, not for the styles.
    limits.package.xml.max_bytes = Limit::default_of(styles.len() as u64 - 1);
    let book = with_style(500, Some(&styles));
    let found = run(book.zip(), &limits, &Rules::default(), (0, 0)).unwrap();
    assert_eq!(codes(&found), [("xml_too_large", "/xl/styles.xml")]);
}

#[test]
fn styles_past_the_cell_formats_limit_are_reported_and_not_checked_against() {
    let styles =
        format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs><xf/><xf/><xf/></cellXfs></styleSheet>"#);
    let zip = with_style(9, Some(&styles)).zip();
    let found = |max| {
        let limits = Limits {
            max_cell_formats: Limit::default_of(max),
            ..Limits::default()
        };
        run(zip.clone(), &limits, &Rules::default(), (0, 0)).unwrap()
    };
    assert_eq!(
        codes(&found(3)),
        [("cell_style_missing", "/xl/worksheets/sheet1.xml")]
    );
    assert_eq!(
        codes(&found(2)),
        [("cell_formats_too_many", "/xl/styles.xml")]
    );
}

#[test]
fn styles_past_the_number_formats_limit_are_reported_and_not_checked_against() {
    let formats: String = (164..167)
        .map(|id| format!(r#"<numFmt numFmtId="{id}" formatCode="0"/>"#))
        .collect();
    let styles = format!(
        r#"<styleSheet xmlns="{MAIN}"><numFmts>{formats}</numFmts><cellXfs><xf/></cellXfs></styleSheet>"#
    );
    let zip = with_style(9, Some(&styles)).zip();
    let found = |max| {
        let limits = Limits {
            max_number_formats: Limit::default_of(max),
            ..Limits::default()
        };
        run(zip.clone(), &limits, &Rules::default(), (0, 0)).unwrap()
    };
    assert_eq!(
        codes(&found(3)),
        [("cell_style_missing", "/xl/worksheets/sheet1.xml")]
    );
    assert_eq!(
        codes(&found(2)),
        [("number_formats_too_many", "/xl/styles.xml")]
    );
}
