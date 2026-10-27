//! DTDs and entities: a DOCTYPE in any form is `xml_doctype`, no entity is
//! ever expanded, and only the predefined entities and character references
//! to characters XML allows are admitted.

use super::{malformed_at, read, utf16};
use xlsx_graft_diagnostics::Diagnostic;

const DECL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;

#[test]
fn an_internal_entity_declaration_is_refused_before_it_is_used() {
    let bytes = br#"<!DOCTYPE a [<!ENTITY x "y">]><a>&x;</a>"#;
    assert_eq!(read(bytes), Err(Diagnostic::XmlDoctype { offset: 0 }));
}

#[test]
fn a_doctype_after_the_declaration_is_found_at_its_offset() {
    let bytes = format!(r#"{DECL}<!DOCTYPE a [<!ENTITY x "y">]><a/>"#);
    let offset = u64::try_from(DECL.len()).unwrap();
    assert_eq!(
        read(bytes.as_bytes()),
        Err(Diagnostic::XmlDoctype { offset })
    );
}

#[test]
fn an_external_entity_is_refused() {
    let system = br#"<!DOCTYPE a SYSTEM "http://example.com/a.dtd"><a/>"#;
    assert_eq!(read(system), Err(Diagnostic::XmlDoctype { offset: 0 }));
    let entity = br#"<!DOCTYPE a [<!ENTITY x SYSTEM "file:///etc/passwd">]><a>&x;</a>"#;
    assert_eq!(read(entity), Err(Diagnostic::XmlDoctype { offset: 0 }));
}

#[test]
fn a_doctype_in_any_letter_case_is_refused() {
    assert_eq!(
        read(b"<!doctype a><a/>"),
        Err(Diagnostic::XmlDoctype { offset: 0 })
    );
    assert_eq!(
        read(b"<!DocType a><a/>"),
        Err(Diagnostic::XmlDoctype { offset: 0 })
    );
}

#[test]
fn nested_entity_expansion_is_refused_unexpanded() {
    let laughs = br#"<!DOCTYPE a [
  <!ENTITY l "lol">
  <!ENTITY l1 "&l;&l;&l;&l;&l;&l;&l;&l;&l;&l;">
  <!ENTITY l2 "&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;">
]><a>&l2;</a>"#;
    assert_eq!(read(laughs), Err(Diagnostic::XmlDoctype { offset: 0 }));
}

#[test]
fn an_entity_declaration_outside_a_doctype_is_malformed() {
    malformed_at(br#"<!ENTITY x "y"><a/>"#);
}

#[test]
fn an_undeclared_entity_is_malformed() {
    let (offset, detail) = malformed_at(b"<a>&x;</a>");
    assert_eq!(
        (offset, detail.as_str()),
        (3, "a reference to an undeclared entity")
    );
    malformed_at(br#"<a b="&x;"/>"#);
}

#[test]
fn predefined_entities_and_character_references_are_admitted() {
    assert_eq!(
        read(b"<a>&lt;&gt;&amp;&apos;&quot;&#65;&#x1F600;</a>"),
        Ok(9)
    );
    assert_eq!(read(br#"<a b="&lt;&#65;"/>"#), Ok(1));
}

#[test]
fn a_reference_to_a_character_xml_does_not_allow_is_malformed() {
    let (offset, detail) = malformed_at(b"<a>&#1;</a>");
    assert_eq!(
        (offset, detail.as_str()),
        (3, "a character XML does not allow (U+0001)")
    );
    let (_, detail) = malformed_at(br#"<a b="&#x1;"/>"#);
    assert_eq!(detail, "a character XML does not allow (U+0001)");
    let (offset, _) = malformed_at(b"<a>&#0;</a>");
    assert_eq!(offset, 3);
    malformed_at(b"<a>&#xZZ;</a>");
}

#[test]
fn a_doctype_inside_or_after_the_root_is_refused_where_it_stands() {
    assert_eq!(
        read(b"<a><!DOCTYPE b></a>"),
        Err(Diagnostic::XmlDoctype { offset: 3 })
    );
    assert_eq!(
        read(b"<a/><!DOCTYPE b>"),
        Err(Diagnostic::XmlDoctype { offset: 4 })
    );
}

#[test]
fn a_reference_to_a_surrogate_is_malformed() {
    malformed_at(b"<a>&#xD800;</a>");
    malformed_at(br#"<a b="&#xD800;"/>"#);
}

#[test]
fn a_doctype_after_a_byte_order_mark_is_at_the_texts_start() {
    let bytes = b"\xef\xbb\xbf<!DOCTYPE a><a/>";
    assert_eq!(read(bytes), Err(Diagnostic::XmlDoctype { offset: 0 }));
}

#[test]
fn a_doctype_in_utf16_is_refused_at_its_utf8_offset() {
    for big in [false, true] {
        let bare = utf16(r#"<!DOCTYPE a [<!ENTITY x "y">]><a>&x;</a>"#, big);
        assert_eq!(read(&bare), Err(Diagnostic::XmlDoctype { offset: 0 }));
        let declared = utf16(
            r#"<?xml version="1.0" encoding="UTF-16"?><!DOCTYPE a><a/>"#,
            big,
        );
        assert_eq!(read(&declared), Err(Diagnostic::XmlDoctype { offset: 39 }));
    }
}
