//! What markup compatibility the shared strings table takes: the
//! namespaces `mc:Ignorable` names passed over, and any other markup
//! compatibility refused.

use super::zip::Item;
use super::{NS, at, book_with, clean, read};
use crate::SharedStrings;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};

/// The markup compatibility namespace.
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// A Transitional `sst` declaring `mc` and `v`, with `attributes` and
/// holding `body`.
fn sst(attributes: &str, body: &str) -> String {
    format!(r#"<sst xmlns="{NS}" xmlns:mc="{MC}" xmlns:v="urn:v"{attributes}>{body}</sst>"#)
}

/// Reads a table of `xml`, which must be refused with the one diagnostic
/// `xml_schema_invalid` at the first `marker` in it, for `reason` naming
/// `name`.
fn refused_at(xml: &str, marker: &str, reason: SchemaReason, name: &str) {
    let part = Item::stored("xl/sharedStrings.xml", xml.as_bytes());
    let (strings, seen) = read(book_with(part));
    assert!(matches!(strings, SharedStrings::Unknown), "{xml}");
    let diagnostic = Diagnostic::XmlSchemaInvalid {
        offset: xml.find(marker).unwrap() as u64,
        reason,
        name: Echo::new(name),
    };
    assert_eq!(seen, vec![at(diagnostic, "/xl/sharedStrings.xml")], "{xml}");
}

#[test]
fn a_namespace_named_ignorable_is_passed_over_where_it_is_named() {
    // On the root, as a part that carries it does.
    let body = concat!(
        r#"<si v:at="1"><t>a</t><v:x><y/></v:x></si>"#,
        r#"<v:si><t>skipped</t></v:si><si><r><t v:at="1">b</t></r></si>"#,
    );
    let xml = sst(r#" v:at="1" mc:Ignorable="v""#, body);
    let part = Item::stored("xl/sharedStrings.xml", xml.as_bytes());
    let (mut strings, seen) = read(book_with(part));
    assert_eq!(seen, vec![]);
    assert_eq!(super::values(&mut strings), ["a", "b"]);
    // Inside an element.
    let body = format!(
        r#"<si xmlns:mc="{MC}" xmlns:v="urn:v" mc:Ignorable="v"><t v:at="1">a</t><v:x/></si>"#
    );
    assert_eq!(clean(&body), ["a"]);
}

#[test]
fn a_namespace_named_ignorable_is_not_ignorable_past_its_element() {
    let body = r#"<si mc:Ignorable="v"><t>a</t></si><si><v:x/></si>"#;
    let xml = sst("", body);
    refused_at(&xml, "<v:x/>", SchemaReason::UnexpectedElement, "v:x");
    let body = r#"<si mc:Ignorable="v"><t>a</t></si><si v:at="1"/>"#;
    let xml = sst("", body);
    refused_at(
        &xml,
        r#"<si v:at"#,
        SchemaReason::UnexpectedAttribute,
        "v:at",
    );
}

#[test]
fn a_namespace_understood_named_ignorable_is_still_read() {
    let body = format!(r#"<si xmlns:s="{NS}" mc:Ignorable="s xml"><s:t>a</s:t></si>"#);
    let xml = sst("", &body);
    let part = Item::stored("xl/sharedStrings.xml", xml.as_bytes());
    let (mut strings, seen) = read(book_with(part));
    assert_eq!(seen, vec![]);
    assert_eq!(super::values(&mut strings), ["a"]);
    // Its attributes are no more ignorable than they were.
    let xml = xml.replace("<s:t>", r#"<s:t s:at="1">"#);
    let attribute = SchemaReason::UnexpectedAttribute;
    refused_at(&xml, "<s:t", attribute, "s:at");
}

#[test]
fn a_namespace_is_ignorable_by_its_name_not_its_prefix() {
    let body = r#"<si mc:Ignorable="v"><t>a</t><v:x xmlns:v="urn:w"/></si>"#;
    let xml = sst("", body);
    refused_at(&xml, "<v:x", SchemaReason::UnexpectedElement, "v:x");
}

#[test]
fn markup_compatibility_beyond_ignorable_is_refused() {
    let invalid = SchemaReason::InvalidValue;
    // A prefix not declared, a name rather than a prefix, and markup
    // compatibility itself.
    for value in ["x14ac", "v:junk", "mc"] {
        let xml = sst(&format!(r#" mc:Ignorable="{value}""#), "");
        refused_at(&xml, "<sst", invalid, "mc:Ignorable");
    }
    let xml = sst("", r#"<si mc:Ignorable="u"/>"#);
    refused_at(&xml, "<si", invalid, "mc:Ignorable");
    let xml = sst(r#" mc:ProcessContent="v""#, "");
    let attribute = SchemaReason::UnexpectedAttribute;
    refused_at(&xml, "<sst", attribute, "mc:ProcessContent");
    let body = "<mc:AlternateContent><mc:Fallback/></mc:AlternateContent>";
    let xml = sst(r#" mc:Ignorable="v""#, body);
    let element = SchemaReason::UnexpectedElement;
    refused_at(&xml, "<mc:Alt", element, "mc:AlternateContent");
}
