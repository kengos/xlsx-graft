//! What the workbook part is refused for as XML: one diagnostic, and the
//! part not read.

use super::super::rel;
use super::super::{NS, STRICT_NS, at};
use super::{PART, R, WORKSHEET, book_with, clean, read, worksheet};
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};

/// The markup compatibility namespace.
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
/// One sheet, as [`super::book`] links it.
const SHEETS: &str = r#"<sheets><sheet name="A" sheetId="1" r:id="rId1"/></sheets>"#;

/// Reads a workbook part of `xml`, which must be refused with the one
/// diagnostic `xml_schema_invalid` at `offset` for `reason` naming `name`.
fn refused_as(xml: &str, offset: u64, reason: SchemaReason, name: &str) {
    let (part, seen) = read(book_with(xml, "", vec![]));
    assert_eq!(part, None, "{xml}");
    let name = Echo::new(name);
    let diagnostic = Diagnostic::XmlSchemaInvalid {
        offset,
        reason,
        name,
    };
    assert_eq!(seen, vec![at(diagnostic, PART)], "{xml}");
}

/// [`refused_as`] for a Transitional `workbook` with `r` bound holding
/// `body`, `offset` counted from the start of `body`.
fn body_refused_as(body: &str, offset: u64, reason: SchemaReason, name: &str) {
    let root = format!(r#"<workbook xmlns="{NS}" xmlns:r="{R}" xmlns:mc="{MC}">"#);
    let xml = format!("{root}{body}</workbook>");
    refused_as(&xml, root.len() as u64 + offset, reason, name);
}

#[test]
fn another_root_or_namespace_is_refused() {
    let element = SchemaReason::UnexpectedElement;
    refused_as(
        &format!(r#"<styleSheet xmlns="{NS}"/>"#),
        0,
        element,
        "styleSheet",
    );
    refused_as(r#"<workbook xmlns="urn:other"/>"#, 0, element, "workbook");
    // A child in the other kind of SpreadsheetML.
    let xml = format!(r#"<workbook xmlns="{NS}"><s:sheets xmlns:s="{STRICT_NS}"/></workbook>"#);
    refused_as(&xml, 19 + NS.len() as u64, element, "s:sheets");
}

#[test]
fn a_child_out_of_order_or_unknown_is_refused() {
    let element = SchemaReason::UnexpectedElement;
    let sheets = SHEETS.len() as u64;
    body_refused_as(
        &format!("{SHEETS}<workbookPr/>"),
        sheets,
        element,
        "workbookPr",
    );
    body_refused_as(&format!("{SHEETS}{SHEETS}"), sheets, element, "sheets");
    body_refused_as("<sheets><other/></sheets>", 8, element, "other");
    body_refused_as("<sheet/>", 0, element, "sheet");
    // Markup compatibility other than `mc:Ignorable` is skipped right
    // inside `workbook` alone.
    body_refused_as(
        "<sheets><mc:AlternateContent/></sheets>",
        8,
        element,
        "mc:AlternateContent",
    );
    body_refused_as("<mc:Choice/>", 0, element, "mc:Choice");
}

#[test]
fn a_required_element_left_out_is_refused() {
    let missing = SchemaReason::MissingElement;
    // Reported where `workbook` starts.
    let xml = format!(r#"<workbook xmlns="{NS}"><workbookPr/></workbook>"#);
    refused_as(&xml, 0, missing, "sheets");
    // A child that comes after `sheets` does not stand for it.
    let xml = format!(r#"<workbook xmlns="{NS}"><calcPr/></workbook>"#);
    refused_as(&xml, 0, missing, "sheets");
    body_refused_as("<sheets></sheets>", 0, missing, "sheet");
    body_refused_as("<sheets/>", 0, missing, "sheet");
}

#[test]
fn a_sheet_without_a_required_attribute_is_refused() {
    let missing = SchemaReason::MissingAttribute;
    for (sheet, name) in [
        (r#"<sheet sheetId="1" r:id="rId1"/>"#, "name"),
        (r#"<sheet name="A" r:id="rId1"/>"#, "sheetId"),
        (r#"<sheet name="A" sheetId="1"/>"#, "r:id"),
    ] {
        body_refused_as(&format!("<sheets>{sheet}</sheets>"), 8, missing, name);
    }
}

#[test]
fn a_value_its_type_does_not_allow_is_refused() {
    let invalid = SchemaReason::InvalidValue;
    for (sheet, name) in [
        (r#"<sheet name="A" sheetId="-1" r:id="rId1"/>"#, "sheetId"),
        (
            r#"<sheet name="A" sheetId="4294967296" r:id="rId1"/>"#,
            "sheetId",
        ),
        (
            r#"<sheet name="A" sheetId="1" state="Hidden" r:id="rId1"/>"#,
            "state",
        ),
    ] {
        body_refused_as(&format!("<sheets>{sheet}</sheets>"), 8, invalid, name);
    }
    body_refused_as(r#"<workbookPr date1904="yes"/>"#, 0, invalid, "date1904");
    let root = format!(r#"<workbook xmlns="{NS}" conformance=" strict"/>"#);
    refused_as(&root, 0, invalid, "conformance");
}

#[test]
fn an_attribute_the_schema_does_not_give_is_refused() {
    let unexpected = SchemaReason::UnexpectedAttribute;
    let in_sheets = |sheet: &str, name: &str| {
        body_refused_as(&format!("<sheets>{sheet}</sheets>"), 8, unexpected, name);
    };
    in_sheets(r#"<sheet name="A" sheetId="1" id="rId1"/>"#, "id");
    in_sheets(
        r#"<sheet name="A" sheetId="1" r:id="rId1" hidden="1"/>"#,
        "hidden",
    );
    // An `r:id` in the relationships namespace of the other kind.
    let strict = "http://purl.oclc.org/ooxml/officeDocument/relationships";
    in_sheets(
        &format!(r#"<sheet xmlns:s="{strict}" name="A" sheetId="1" s:id="rId1"/>"#),
        "s:id",
    );
    in_sheets(
        r#"<sheet name="A" sheetId="1" r:id="rId1" r:name="A"/>"#,
        "r:name",
    );
    body_refused_as(r#"<sheets count="1"/>"#, 0, unexpected, "count");
    body_refused_as(r#"<workbookPr name="A"/>"#, 0, unexpected, "name");
    let xml = format!(r#"<workbook xmlns="{NS}" date1904="1"/>"#);
    refused_as(&xml, 0, unexpected, "date1904");
}

#[test]
fn text_where_only_elements_stand_is_refused_but_inside_a_skipped_one() {
    let text = SchemaReason::UnexpectedText;
    body_refused_as("x", 0, text, "workbook");
    body_refused_as("<workbookPr>x</workbookPr>", 12, text, "workbookPr");
    body_refused_as("<sheets>&#32;</sheets>", 8, text, "sheets");
    body_refused_as(
        r#"<sheets><sheet name="A" sheetId="1" r:id="rId1"><![CDATA[ ]]></sheet></sheets>"#,
        48,
        text,
        "sheet",
    );
    let skipped = format!(
        "<bookViews><workbookView>x<extLst/></workbookView></bookViews>{SHEETS}<![CDATA[]]>\
         <definedNames><definedName name='a'>A!$1:$1</definedName></definedNames>"
    );
    assert_eq!(clean(&skipped).sheets().len(), 1);
}

#[test]
fn markup_compatibility_skips_what_mc_ignorable_names_and_alternate_content() {
    // A `mc:AlternateContent` right inside `workbook` is skipped at any
    // stage, and does not move it on.
    let body = format!(
        r#"<workbookPr xmlns:x="urn:x" mc:Ignorable="x" x:a="1"><x:b><y/></x:b></workbookPr>
<mc:AlternateContent><mc:Choice Requires="x"><sheets/></mc:Choice></mc:AlternateContent>
<sheets>{}</sheets>
<mc:AlternateContent/>"#,
        r#"<sheet xmlns:z="urn:z" mc:Ignorable="z" z:c="1" name="A" sheetId="1" r:id="rId1"/>"#
    );
    let xml = format!(r#"<workbook xmlns="{NS}" xmlns:r="{R}" xmlns:mc="{MC}">{body}</workbook>"#);
    let related = rel("rId1", WORKSHEET, "worksheets/sheet1.xml");
    let (part, seen) = read(book_with(&xml, &related, vec![worksheet(1)]));
    assert_eq!(seen, vec![]);
    assert_eq!(part.unwrap().sheets().len(), 1);
    // A namespace not named ignorable is refused.
    body_refused_as(
        r#"<x:b xmlns:x="urn:x"/>"#,
        0,
        SchemaReason::UnexpectedElement,
        "x:b",
    );
    // As is a `mc:Ignorable` that names a prefix not declared.
    body_refused_as(
        &format!(
            r#"<sheets mc:Ignorable="q">{}</sheets>"#,
            &SHEETS[8..SHEETS.len() - 9]
        ),
        0,
        SchemaReason::InvalidValue,
        "mc:Ignorable",
    );
}
