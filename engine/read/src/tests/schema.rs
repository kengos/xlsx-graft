//! What the table is refused for as XML: one diagnostic, and the table
//! unknown.

use super::zip::Item;
use super::{NS, at, book, book_with, read};
use crate::SharedStrings;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};

/// Reads a table of `xml`, which must be refused with the one diagnostic
/// `xml_schema_invalid` at `offset` for `reason` naming `name`.
fn refused_as(xml: &str, offset: u64, reason: SchemaReason, name: &str) {
    let (strings, seen) = read(book_with(Item::stored(
        "xl/sharedStrings.xml",
        xml.as_bytes(),
    )));
    assert!(matches!(strings, SharedStrings::Unknown), "{xml}");
    let name = Echo::new(name);
    let diagnostic = Diagnostic::XmlSchemaInvalid {
        offset,
        reason,
        name,
    };
    assert_eq!(seen, vec![at(diagnostic, "/xl/sharedStrings.xml")], "{xml}");
}

/// [`refused_as`] for a Transitional `sst` holding `body`, `offset` counted
/// from the start of `body`.
fn body_refused_as(body: &str, offset: u64, reason: SchemaReason, name: &str) {
    let root = format!(r#"<sst xmlns="{NS}">"#);
    let xml = format!("{root}{body}</sst>");
    refused_as(&xml, root.len() as u64 + offset, reason, name);
}

#[test]
fn another_root_or_namespace_is_refused() {
    let element = SchemaReason::UnexpectedElement;
    refused_as(&format!(r#"<table xmlns="{NS}"/>"#), 0, element, "table");
    refused_as(r#"<sst xmlns="urn:other"/>"#, 0, element, "sst");
    refused_as("<sst/>", 0, element, "sst");
    refused_as(
        r#"<x:sst xmlns:x="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
        0,
        element,
        "x:sst",
    );
}

#[test]
fn a_child_in_another_namespace_than_the_root_is_refused() {
    let strict = "http://purl.oclc.org/ooxml/spreadsheetml/main";
    let body = format!(r#"<s:si xmlns:s="{strict}"/>"#);
    body_refused_as(&body, 0, SchemaReason::UnexpectedElement, "s:si");
    let body = r#"<si><t xmlns="urn:x">a</t></si>"#;
    body_refused_as(body, 4, SchemaReason::UnexpectedElement, "t");
}

#[test]
fn an_element_the_schema_does_not_have_there_is_refused() {
    let element = SchemaReason::UnexpectedElement;
    body_refused_as("<si><t>a</t></si><x/>", 17, element, "x");
    body_refused_as("<t>a</t>", 0, element, "t");
    body_refused_as("<si><b/></si>", 4, element, "b");
    body_refused_as("<si><t>a<b/></t></si>", 8, element, "b");
    body_refused_as("<si><r><x/><t/></r></si>", 7, element, "x");
}

#[test]
fn an_element_out_of_the_schema_order_or_repeated_is_refused() {
    let element = SchemaReason::UnexpectedElement;
    body_refused_as("<extLst/><si/>", 9, element, "si");
    body_refused_as("<extLst/><extLst/>", 9, element, "extLst");
    body_refused_as("<si><r><t/></r><t/></si>", 15, element, "t");
    body_refused_as("<si><t/><t/></si>", 8, element, "t");
    body_refused_as(
        "<si><phoneticPr fontId=\"0\"/><rPh/></si>",
        28,
        element,
        "rPh",
    );
    body_refused_as("<si><rPh/><r><t/></r></si>", 10, element, "r");
    body_refused_as("<si><r><t/><rPr/></r></si>", 11, element, "rPr");
    body_refused_as("<si><r><t/><t/></r></si>", 11, element, "t");
}

#[test]
fn a_run_without_its_text_is_refused() {
    let missing = SchemaReason::MissingElement;
    body_refused_as("<si><r/></si>", 4, missing, "t");
    body_refused_as("<si><r><rPr><b/></rPr></r></si>", 4, missing, "t");
}

#[test]
fn an_attribute_the_schema_does_not_have_is_refused() {
    let attribute = SchemaReason::UnexpectedAttribute;
    refused_as(&format!(r#"<sst xmlns="{NS}" x="1"/>"#), 0, attribute, "x");
    body_refused_as(r#"<si a="1"/>"#, 0, attribute, "a");
    body_refused_as(r#"<si><r b="1"><t/></r></si>"#, 4, attribute, "b");
    body_refused_as(r#"<si><t space="preserve"/></si>"#, 4, attribute, "space");
    body_refused_as(
        r#"<si xmlns:mc="urn:mc"><t mc:Ignorable="x"/></si>"#,
        22,
        attribute,
        "mc:Ignorable",
    );
}

#[test]
fn text_where_only_elements_may_stand_is_refused() {
    let text = SchemaReason::UnexpectedText;
    body_refused_as("a", 0, text, "sst");
    body_refused_as("<si> <![CDATA[x]]></si>", 5, text, "si");
    body_refused_as("<si><r>&amp;<t/></r></si>", 7, text, "r");
    // White space written as a reference or a CDATA section is text.
    body_refused_as("<si>&#32;<t/></si>", 4, text, "si");
    body_refused_as("&#x9;<si/>", 0, text, "sst");
    body_refused_as("<si><![CDATA[ ]]><t/></si>", 4, text, "si");
}

#[test]
fn white_space_written_as_such_or_an_empty_cdata_stands_between_elements() {
    let body = "\r\n\t <si> <t>a&#32;</t>\n<![CDATA[]]></si>\n";
    assert_eq!(super::clean(body), ["a "]);
}

#[test]
fn what_a_skipped_element_holds_is_not_checked() {
    let body = concat!(
        r#"<si><t>a</t><rPh any="1">text<x><y/></x></rPh>"#,
        r#"<phoneticPr><z>&amp;</z></phoneticPr></si>"#,
        r#"<si><r><rPr unknown="1"><x>text</x></rPr><t>b</t></r></si>"#,
        "<extLst>text<a/></extLst>",
    );
    let (mut strings, seen) = read(book(body));
    assert_eq!(seen, vec![]);
    assert_eq!(super::values(&mut strings), ["a", "b"]);
}

#[test]
fn malformed_xml_is_refused_by_the_reader_alone() {
    let (strings, seen) = read(book("<si><t>a</si>"));
    assert!(matches!(strings, SharedStrings::Unknown));
    assert_eq!(seen.len(), 1);
    assert!(
        matches!(seen[0].0, Diagnostic::XmlMalformed { .. }),
        "{seen:?}"
    );
    assert_eq!(seen[0].1, "/xl/sharedStrings.xml");
}
