//! What the styles are refused for as XML: one diagnostic, and the styles
//! unknown.

use super::super::{NS, at};
use super::{PART, part, read, styled_with};
use crate::{FormatKind, Styles};
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};

/// The markup compatibility namespace.
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// Reads styles of `xml`, which must be refused with the one diagnostic
/// `xml_schema_invalid` at `offset` for `reason` naming `name`.
fn refused_as(xml: &str, offset: u64, reason: SchemaReason, name: &str) {
    let (styles, seen) = read(styled_with(part(xml)));
    assert_eq!(styles, Styles::Unknown, "{xml}");
    let name = Echo::new(name);
    let diagnostic = Diagnostic::XmlSchemaInvalid {
        offset,
        reason,
        name,
    };
    assert_eq!(seen, vec![at(diagnostic, PART)], "{xml}");
}

/// [`refused_as`] for a Transitional `styleSheet` holding `body`, `offset`
/// counted from the start of `body`.
fn body_refused_as(body: &str, offset: u64, reason: SchemaReason, name: &str) {
    let root = format!(r#"<styleSheet xmlns="{NS}">"#);
    let xml = format!("{root}{body}</styleSheet>");
    refused_as(&xml, root.len() as u64 + offset, reason, name);
}

#[test]
fn another_root_or_namespace_is_refused() {
    let element = SchemaReason::UnexpectedElement;
    refused_as(&format!(r#"<sst xmlns="{NS}"/>"#), 0, element, "sst");
    refused_as(
        r#"<styleSheet xmlns="urn:other"/>"#,
        0,
        element,
        "styleSheet",
    );
    refused_as("<styleSheet/>", 0, element, "styleSheet");
    let xml = format!(r#"<styleSheet xmlns="{NS}"><o:cellXfs xmlns:o="urn:o"/></styleSheet>"#);
    let root = format!(r#"<styleSheet xmlns="{NS}">"#).len() as u64;
    refused_as(&xml, root, element, "o:cellXfs");
}

#[test]
fn a_child_unknown_out_of_order_or_repeated_is_refused() {
    let element = SchemaReason::UnexpectedElement;
    body_refused_as("<table/>", 0, element, "table");
    body_refused_as("<cellXfs><xf/></cellXfs><numFmts/>", 24, element, "numFmts");
    body_refused_as("<fonts/><fonts/>", 8, element, "fonts");
    body_refused_as("<numFmts><xf/></numFmts>", 9, element, "xf");
    body_refused_as(
        r#"<numFmts><numFmt numFmtId="1" formatCode="0"><a/></numFmt></numFmts>"#,
        45,
        element,
        "a",
    );
    body_refused_as("<cellXfs><numFmt/></cellXfs>", 9, element, "numFmt");
    body_refused_as("<cellXfs><xf><font/></xf></cellXfs>", 13, element, "font");
    body_refused_as(
        "<cellXfs><xf><protection/><alignment/></xf></cellXfs>",
        26,
        element,
        "alignment",
    );
}

#[test]
fn an_attribute_the_schema_does_not_give_is_refused() {
    let attribute = SchemaReason::UnexpectedAttribute;
    body_refused_as(r#"<numFmts size="1"/>"#, 0, attribute, "size");
    body_refused_as(
        r#"<cellXfs><xf numFmt="1"/></cellXfs>"#,
        9,
        attribute,
        "numFmt",
    );
    body_refused_as(
        r#"<numFmts><numFmt numFmtId="1" formatCode="0" count="1"/></numFmts>"#,
        9,
        attribute,
        "count",
    );
    body_refused_as(
        r#"<cellXfs xmlns:o="urn:o" o:count="1"><xf/></cellXfs>"#,
        0,
        attribute,
        "o:count",
    );
    // An `Ignorable` outside markup compatibility is not `mc:Ignorable`.
    body_refused_as(r#"<numFmts Ignorable="x"/>"#, 0, attribute, "Ignorable");
    let root = format!(r#"<styleSheet xmlns="{NS}" count="1"/>"#);
    refused_as(&root, 0, attribute, "count");
}

#[test]
fn a_number_format_without_its_id_or_code_is_refused() {
    let missing = SchemaReason::MissingAttribute;
    body_refused_as(
        r#"<numFmts><numFmt formatCode="0"/></numFmts>"#,
        9,
        missing,
        "numFmtId",
    );
    body_refused_as(
        r#"<numFmts><numFmt numFmtId="164"/></numFmts>"#,
        9,
        missing,
        "formatCode",
    );
}

#[test]
fn a_number_format_id_that_is_not_an_unsigned_int_is_refused() {
    let invalid = SchemaReason::InvalidValue;
    for id in ["", "x", "-1", "1.0", "4294967296"] {
        body_refused_as(
            &format!(r#"<cellXfs><xf numFmtId="{id}"/></cellXfs>"#),
            9,
            invalid,
            "numFmtId",
        );
        body_refused_as(
            &format!(r#"<numFmts><numFmt numFmtId="{id}" formatCode="0"/></numFmts>"#),
            9,
            invalid,
            "numFmtId",
        );
    }
}

#[test]
fn cell_formats_without_a_format_are_refused() {
    let missing = SchemaReason::MissingElement;
    body_refused_as("<cellXfs/>", 0, missing, "xf");
    body_refused_as("<numFmts/><cellXfs> </cellXfs>", 10, missing, "xf");
}

#[test]
fn text_where_only_elements_stand_is_refused() {
    let text = SchemaReason::UnexpectedText;
    body_refused_as("x", 0, text, "styleSheet");
    body_refused_as("<numFmts>x</numFmts>", 9, text, "numFmts");
    body_refused_as(
        r#"<numFmts><numFmt numFmtId="1" formatCode="0">x</numFmt></numFmts>"#,
        45,
        text,
        "numFmt",
    );
    body_refused_as("<cellXfs><xf/>&amp;</cellXfs>", 14, text, "cellXfs");
    body_refused_as("<cellXfs><xf><![CDATA[x]]></xf></cellXfs>", 13, text, "xf");
    // White space written as a reference or a CDATA section is text.
    body_refused_as("<cellXfs>&#32;<xf/></cellXfs>", 9, text, "cellXfs");
    body_refused_as("<cellXfs><xf/><![CDATA[ ]]></cellXfs>", 14, text, "cellXfs");
}

#[test]
fn white_space_written_as_such_or_an_empty_cdata_stands_between_elements() {
    let body = "\r\n\t <cellXfs>\n<![CDATA[]]><xf/> </cellXfs>\n";
    assert_eq!(super::kinds(body), [FormatKind::General]);
}

#[test]
fn markup_compatibility_beyond_ignorable_is_refused() {
    let root = format!(r#"<styleSheet xmlns="{NS}" xmlns:mc="{MC}""#);
    let at = root.len() as u64 + 1;
    let refused = |rest: &str, offset: u64, reason, name: &str| {
        refused_as(&format!("{root}{rest}"), offset, reason, name);
    };
    // A prefix `mc:Ignorable` names must be declared.
    refused(
        r#" mc:Ignorable="x14ac"/>"#,
        0,
        SchemaReason::InvalidValue,
        "mc:Ignorable",
    );
    refused(
        r#" mc:Ignorable="mc"><mc:AlternateContent/></styleSheet>"#,
        0,
        SchemaReason::InvalidValue,
        "mc:Ignorable",
    );
    // A prefix, not a name.
    refused(
        r#" xmlns:v="urn:v" mc:Ignorable="v:junk"/>"#,
        0,
        SchemaReason::InvalidValue,
        "mc:Ignorable",
    );
    refused(
        r#" mc:ProcessContent="x"/>"#,
        0,
        SchemaReason::UnexpectedAttribute,
        "mc:ProcessContent",
    );
    refused(
        "><mc:AlternateContent/></styleSheet>",
        at,
        SchemaReason::UnexpectedElement,
        "mc:AlternateContent",
    );
    // What one element names ignorable is not ignorable past its end.
    let body = concat!(
        r#"<numFmts xmlns:v="urn:v" mc:Ignorable="v"/>"#,
        r#"<cellXfs xmlns:v="urn:v"><v:x/></cellXfs></styleSheet>"#,
    );
    let offset = at + body.find("<v:x/>").unwrap() as u64;
    refused(
        &format!(">{body}"),
        offset,
        SchemaReason::UnexpectedElement,
        "v:x",
    );
}
