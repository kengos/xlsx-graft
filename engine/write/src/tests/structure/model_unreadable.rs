//! `Workbook::read` on workbooks it gives `None` for: each pushes the
//! diagnostics named, by code and where it points, and no other.

use super::model::{
    NS, TABLE, WORKBOOK, WORKBOOK_RELS, bytes, one_sheet, open, package, read, read_workbook, rel,
    relationships, rels, root_rels, workbook,
};
use crate::WriteLimits;
use crate::structure::model::Workbook;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit};
use xlsx_graft_xml::XmlLimits;

/// `read_workbook` of the workbook part `xml`, which must give `None` and
/// the diagnostics `expected`.
fn refused(xml: &str, expected: &[&str]) {
    let (workbook, found) = read_workbook(xml);
    assert!(workbook.is_none(), "{xml}");
    assert_eq!(found, expected, "{xml}");
}

/// `xml_schema_invalid` in the workbook part, for `reason` and `name`.
fn schema(reason: &str, name: &str) -> String {
    format!("xml_schema_invalid {WORKBOOK} {reason} {name}")
}

/// A workbook part of the sheet `sheet`, in the root's own namespace.
fn with_sheet(sheet: &str) -> String {
    workbook(&format!("<sheets>{sheet}</sheets>"))
}

#[test]
fn a_root_that_is_not_a_workbook_in_spreadsheetml_is_refused() {
    refused(
        &format!(r#"<other xmlns="{NS}"/>"#),
        &[&schema("unexpected_element", "other")],
    );
    refused(
        r#"<workbook xmlns="urn:x"/>"#,
        &[&schema("unexpected_element", "workbook")],
    );
    refused("<workbook/>", &[&schema("unexpected_element", "workbook")]);
}

#[test]
fn a_workbook_without_a_sheet_is_refused() {
    refused(
        &format!(r#"<workbook xmlns="{NS}"/>"#),
        &[&schema("missing_element", "sheets")],
    );
    refused(
        &workbook("<calcPr/>"),
        &[&schema("missing_element", "sheets")],
    );
    refused(
        &workbook("<sheets/>"),
        &[&schema("missing_element", "sheet")],
    );
    refused(
        &workbook("<sheets></sheets>"),
        &[&schema("missing_element", "sheet")],
    );
}

#[test]
fn a_sheet_without_an_attribute_or_with_a_wrong_value_is_refused() {
    let strict = r#"<workbook xmlns="http://purl.oclc.org/ooxml/spreadsheetml/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
    for (sheet, reason, name) in [
        (
            r#"<sheet sheetId="1" r:id="rId1"/>"#,
            "missing_attribute",
            "name",
        ),
        (
            r#"<sheet name="S" r:id="rId1"/>"#,
            "missing_attribute",
            "sheetId",
        ),
        (
            r#"<sheet name="S" sheetId="1"/>"#,
            "missing_attribute",
            "r:id",
        ),
        (
            r#"<sheet name="S" sheetId="1" id="rId1"/>"#,
            "missing_attribute",
            "r:id",
        ),
        (
            r#"<sheet name="S" sheetId="1" xmlns:q="urn:q" q:id="rId1"/>"#,
            "missing_attribute",
            "r:id",
        ),
        (
            r#"<sheet name="S" sheetId="x" r:id="rId1"/>"#,
            "invalid_value",
            "sheetId",
        ),
        (
            r#"<sheet name="S" sheetId="-1" r:id="rId1"/>"#,
            "invalid_value",
            "sheetId",
        ),
        (
            r#"<sheet name="S" sheetId="4294967296" r:id="rId1"/>"#,
            "invalid_value",
            "sheetId",
        ),
        (
            r#"<sheet name="S" sheetId="" r:id="rId1"/>"#,
            "invalid_value",
            "sheetId",
        ),
        (
            r#"<sheet name="S" sheetId="1" state="gone" r:id="rId1"/>"#,
            "invalid_value",
            "state",
        ),
        (
            r#"<sheet name="S" sheetId="1" state="Hidden" r:id="rId1"/>"#,
            "invalid_value",
            "state",
        ),
    ] {
        refused(&with_sheet(sheet), &[&schema(reason, name)]);
    }
    // An `r:id` in the Transitional namespace in a Strict workbook.
    refused(strict, &[&schema("missing_attribute", "r:id")]);
}

#[test]
fn a_view_or_a_defined_name_with_a_wrong_value_is_refused() {
    let sheets = r#"<sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets>"#;
    for (after, reason, name) in [
        (
            r#"<definedNames><definedName name="a" localSheetId="x"/></definedNames>"#,
            "invalid_value",
            "localSheetId",
        ),
        (
            r#"<definedNames><definedName name="a" localSheetId="-1"/></definedNames>"#,
            "invalid_value",
            "localSheetId",
        ),
        (
            r#"<definedNames><definedName localSheetId="1"/></definedNames>"#,
            "missing_attribute",
            "name",
        ),
        (
            r#"<definedNames><definedName x:name="a" xmlns:x="urn:x"/></definedNames>"#,
            "missing_attribute",
            "name",
        ),
        (
            r#"<definedNames><definedName name="a"><b/></definedName></definedNames>"#,
            "unexpected_element",
            "b",
        ),
        (
            r#"<definedNames><definedName name="a">1<x:b xmlns:x="urn:x"/></definedName></definedNames>"#,
            "unexpected_element",
            "x:b",
        ),
        (
            r#"<definedNames><definedName name="a"><!-- c --><definedName name="b"/></definedName></definedNames>"#,
            "unexpected_element",
            "definedName",
        ),
        (
            r#"<bookViews><workbookView activeTab="-1"/></bookViews>"#,
            "invalid_value",
            "activeTab",
        ),
        (
            r#"<bookViews><workbookView firstSheet="a"/></bookViews>"#,
            "invalid_value",
            "firstSheet",
        ),
    ] {
        refused(
            &workbook(&format!("{sheets}{after}")),
            &[&schema(reason, name)],
        );
    }
}

#[test]
fn a_second_sheets_defined_names_book_views_or_calc_pr_is_refused() {
    for (second, name) in [
        (r#"<sheets/>"#, "sheets"),
        (r#"<definedNames/><definedNames/>"#, "definedNames"),
        (r#"<bookViews/><bookViews/>"#, "bookViews"),
        (r#"<calcPr/><calcPr/>"#, "calcPr"),
    ] {
        refused(&one_sheet(second), &[&schema("unexpected_element", name)]);
    }
}

#[test]
fn xml_that_cannot_be_read_pushes_the_xml_readers_diagnostic_at_the_workbook_part() {
    refused("<workbook", &[&format!("xml_malformed {WORKBOOK}")]);
    refused(
        "<!DOCTYPE workbook><workbook/>",
        &[&format!("xml_doctype {WORKBOOK}")],
    );
}

#[test]
fn a_workbook_part_over_the_limit_is_not_read() {
    let mut package = package(&root_rels(), &one_sheet(""), &rels(), &[]);
    let limits = WriteLimits {
        xml: XmlLimits {
            max_bytes: Limit::default_of(10),
            ..XmlLimits::default()
        },
        ..WriteLimits::default()
    };
    let (workbook, found) = read(&mut package, &limits);
    assert!(workbook.is_none());
    assert_eq!(found, [format!("xml_too_large {WORKBOOK}")]);
}

#[test]
fn a_package_without_one_workbook_part_is_not_read() {
    // No officeDocument: the package model reports it, not Workbook::read.
    let none =
        relationships(r#"<Relationship Id="rId1" Type="urn:other" Target="xl/workbook.xml"/>"#);
    let mut without = package(&none, &one_sheet(""), &rels(), &[]);
    let (workbook, found) = read(&mut without, &WriteLimits::default());
    assert!(workbook.is_none());
    assert_eq!(found, Vec::<String>::new());
    // Two officeDocument relationships that target different parts.
    let office =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
    let two = relationships(&format!(
        r#"<Relationship Id="rId1" Type="{office}" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="{office}" Target="xl/other.xml"/>"#
    ));
    let extra = [("xl/other.xml", "", "<workbook/>")];
    let mut two = package(&two, &one_sheet(""), &rels(), &extra);
    let (workbook, found) = read(&mut two, &WriteLimits::default());
    assert!(workbook.is_none());
    assert_eq!(found, ["relationship_type_duplicate /_rels/.rels#rId2"]);
}

#[test]
fn a_sheet_that_cannot_be_tied_to_a_part_pushes_the_reader_diagnostics() {
    let two = |a: &str, b: &str| {
        workbook(&format!(
            r#"<sheets><sheet name="{a}" sheetId="1" r:id="rId1"/><sheet name="{b}" sheetId="2" r:id="rId2"/></sheets>"#
        ))
    };
    let external = relationships(
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="https://example.com/" TargetMode="External"/>"#,
    );
    let styles = relationships(&format!(
        "{}{}",
        rel("rId1", "styles", 1),
        rel("rId2", "worksheet", 2)
    ));
    let cases = [
        // r:id names no relationship.
        (
            with_sheet(r#"<sheet name="S" sheetId="1" r:id="rId99"/>"#),
            rels(),
            vec![format!("sheet_relationship_missing {WORKBOOK}")],
        ),
        // The sheet's relationship targets a part outside the package.
        (
            one_sheet(""),
            external,
            vec![format!("sheet_relationship_missing {WORKBOOK_RELS}#rId1")],
        ),
        // The sheet's relationship is of the styles type, which is no sheet's type.
        (
            one_sheet(""),
            styles,
            vec![format!(
                "sheet_relationship_type_invalid {WORKBOOK_RELS}#rId1"
            )],
        ),
        // Two sheet names that differ only in letter case.
        (
            two("Data", "DATA"),
            rels(),
            vec![format!("sheet_name_duplicate {WORKBOOK}")],
        ),
        (
            two("É", "é"),
            rels(),
            vec![format!("sheet_name_duplicate {WORKBOOK}")],
        ),
        // Two sheets whose r:id target the same part.
        (
            workbook(
                r#"<sheets><sheet name="A" sheetId="1" r:id="rId1"/><sheet name="B" sheetId="2" r:id="rId1"/></sheets>"#,
            ),
            rels(),
            vec![format!("sheet_part_duplicate {WORKBOOK}")],
        ),
        // A duplicate name and a duplicate part, then a sheet whose r:id names no
        // relationship, in the order of the sheets.
        (
            workbook(
                r#"<sheets><sheet name="A" sheetId="1" r:id="rId1"/><sheet name="a" sheetId="2" r:id="rId1"/><sheet name="C" sheetId="3" r:id="rId99"/></sheets>"#,
            ),
            rels(),
            vec![
                format!("sheet_name_duplicate {WORKBOOK}"),
                format!("sheet_part_duplicate {WORKBOOK}"),
                format!("sheet_relationship_missing {WORKBOOK}"),
            ],
        ),
    ];
    for (xml, rels, expected) in cases {
        let mut package = package(&root_rels(), &xml, &rels, &[]);
        let (workbook, found) = read(&mut package, &WriteLimits::default());
        assert!(workbook.is_none(), "{xml}");
        assert_eq!(found, expected, "{xml}");
    }
}

#[test]
fn relationships_that_were_not_all_read_give_no_workbook() {
    let two = |a: &str, b: &str| {
        workbook(&format!(
            r#"<sheets><sheet name="{a}" sheetId="1" r:id="rId1"/><sheet name="{b}" sheetId="2" r:id="rId2"/></sheets>"#
        ))
    };
    // A relationship that is not a sheet's type, and one whose target is no
    // part, which the package leaves out and so reads no more than part of
    // the relationships.
    let partial = relationships(&format!(
        r#"{}<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/none.xml"/>"#,
        rel("rId1", "styles", 1)
    ));
    // Every sheet is tied to a part, but a relationship the package left out
    // may have the `Id` a new relationship would be given.
    let unseen = relationships(&format!(
        r#"{}<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/none.xml"/>"#,
        rel("rId1", "worksheet", 1)
    ));
    // Of these, only `sheet_relationship_missing` is left out: the package
    // reported why the relationships are not all there.
    let cases = [
        (one_sheet(""), unseen, vec![]),
        (one_sheet(""), "<Relationships".to_owned(), vec![]),
        (
            two("Data", "DATA"),
            "<Relationships".to_owned(),
            vec![format!("sheet_name_duplicate {WORKBOOK}")],
        ),
        (
            two("A", "B"),
            partial,
            vec![format!(
                "sheet_relationship_type_invalid {WORKBOOK_RELS}#rId1"
            )],
        ),
    ];
    for (xml, rels, expected) in cases {
        let mut built = package(&root_rels(), &xml, &rels, &[]);
        let (workbook, found) = read(&mut built, &WriteLimits::default());
        assert!(workbook.is_none(), "{xml}");
        assert_eq!(found, expected, "{xml}");
    }
}

#[test]
fn a_part_that_defines_more_names_than_the_limit_is_refused() {
    let names = r#"<definedName name="a"/>"#.repeat(65_537);
    let xml = one_sheet(&format!("<definedNames>{names}</definedNames>"));
    let mut built = package(&root_rels(), &xml, &rels(), &[]);
    let mut diagnostics = Diagnostics::new();
    let workbook = Workbook::read(&mut built, &WriteLimits::default(), &mut diagnostics);
    assert!(workbook.unwrap().is_none());
    let records = diagnostics.records();
    assert_eq!(records.len(), 1);
    // The count is of all the names, and the limit is the one `xlsx-graft-verify`
    // has by default.
    assert_eq!(
        records[0].diagnostic,
        Diagnostic::DefinedNamesTooMany {
            count: 65_537,
            limit: Limit::default_of(65_536),
        }
    );
    assert_eq!(
        records[0].location.part.as_ref().map(|part| part.as_str()),
        Some(WORKBOOK)
    );
}

#[test]
fn a_table_part_that_cannot_be_read_gives_no_workbook() {
    let table =
        r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="T"/>"#;
    let broken = [("xl/tables/table1.xml", TABLE, "<table")];
    let mut built = package(&root_rels(), &one_sheet(""), &rels(), &broken);
    let (workbook, found) = read(&mut built, &WriteLimits::default());
    assert!(workbook.is_none());
    assert_eq!(found, ["xml_malformed /xl/tables/table1.xml"]);
    // A table part that is malformed after its root start tag.
    let after_root = [(
        "xl/tables/table1.xml",
        TABLE,
        r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="T"><a></table>"#,
    )];
    let mut built = package(&root_rels(), &one_sheet(""), &rels(), &after_root);
    let (workbook, found) = read(&mut built, &WriteLimits::default());
    assert!(workbook.is_none());
    assert_eq!(found, ["xml_malformed /xl/tables/table1.xml"]);
    // The first table is read, the second is broken.
    let extra = [
        ("xl/tables/table1.xml", TABLE, table),
        ("xl/tables/table2.xml", TABLE, "<table"),
    ];
    let mut built = package(&root_rels(), &one_sheet(""), &rels(), &extra);
    let (workbook, found) = read(&mut built, &WriteLimits::default());
    assert!(workbook.is_none());
    assert_eq!(found, ["xml_malformed /xl/tables/table2.xml"]);
    // A table part whose entry is damaged: its content no longer matches
    // its CRC-32.
    let extra = [("xl/tables/table1.xml", TABLE, table)];
    let mut zip = bytes(&root_rels(), &one_sheet(""), &rels(), &extra);
    let at = (zip.windows(table.len())).position(|w| w == table.as_bytes());
    zip[at.unwrap() + table.find("\"T\"").unwrap() + 1] = b'U';
    let mut built = open(zip);
    let (workbook, found) = read(&mut built, &WriteLimits::default());
    assert!(workbook.is_none());
    assert_eq!(found, ["zip_crc_mismatch xl/tables/table1.xml"]);
}

#[test]
fn a_diagnostics_collector_that_keeps_no_record_still_gives_no_workbook_for_a_duplicate_sheet_name()
{
    // The collector keeps no record, so a count of its records cannot tell
    // that a diagnostic was pushed.
    let xml = workbook(
        r#"<sheets><sheet name="Data" sheetId="1" r:id="rId1"/><sheet name="DATA" sheetId="2" r:id="rId2"/></sheets>"#,
    );
    let mut built = package(&root_rels(), &xml, &rels(), &[]);
    let mut diagnostics = Diagnostics::with_limit(Limit::default_of(0));
    let workbook = Workbook::read(&mut built, &WriteLimits::default(), &mut diagnostics);
    assert!(workbook.unwrap().is_none());
    assert_eq!(diagnostics.dropped(), 1);
}
