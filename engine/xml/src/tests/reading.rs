//! What the reader admits: a well-formed part reads to `Eof` with its events
//! intact, and a reader that has failed stays failed.

use super::{read, utf16};
use crate::events::Event;
use crate::name::{Namespace, ResolveResult};
use crate::{XmlLimits, XmlReader, XmlVersion};
use xlsx_graft_diagnostics::{Diagnostic, Limit};

const RELS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<!-- written by hand -->
<?keep this?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="t&amp;u" Target="xl/workbook.xml"/>
  <Note><![CDATA[<raw>]]> &lt;&#65;&#x42; </Note>
</Relationships>
"#;

#[test]
fn a_well_formed_part_reads_to_its_end() {
    assert_eq!(read(RELS), Ok(21));
}

#[test]
fn events_carry_the_parts_content() {
    let mut reader = XmlReader::open(RELS, &XmlLimits::default()).unwrap();
    let mut buf = Vec::new();
    let mut names = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).unwrap() {
            Event::Start(e) | Event::Empty(e) => {
                names.push(e.name().as_ref().to_owned());
            }
            Event::Eof => break,
            _ => {}
        }
    }
    assert_eq!(names, ["Relationships", "Relationship", "Note"]);
}

#[test]
fn a_utf16_attribute_value_arrives_as_utf8() {
    let bytes = utf16(
        r#"<?xml version="1.0" encoding="UTF-16"?><a b="é𝄞"/>"#,
        false,
    );
    let mut reader = XmlReader::open(&bytes, &XmlLimits::default()).unwrap();
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf).unwrap(),
        Event::Decl(_)
    ));
    let Event::Empty(element) = reader.read_event_into(&mut buf).unwrap() else {
        panic!("expected the empty element");
    };
    let value = element
        .attributes()
        .next()
        .unwrap()
        .unwrap()
        .normalized_value(XmlVersion::Implicit1_0)
        .unwrap();
    assert_eq!(value, "é𝄞");
}

#[test]
fn a_failed_reader_returns_its_diagnostic_again() {
    let mut reader = XmlReader::open(b"<a></b>", &XmlLimits::default()).unwrap();
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf).unwrap(),
        Event::Start(_)
    ));
    let first = reader.read_event_into(&mut buf).unwrap_err();
    assert!(matches!(first, Diagnostic::XmlMalformed { .. }));
    assert_eq!(reader.read_event_into(&mut buf).unwrap_err(), first);
    assert_eq!(reader.read_event_into(&mut buf).unwrap_err(), first);
}

#[test]
fn the_default_limits_are_the_documented_values() {
    let limits = XmlLimits::default();
    assert_eq!(limits.max_depth, Limit::default_of(256));
    assert_eq!(limits.max_attributes, Limit::default_of(1_024));
    assert_eq!(limits.max_namespaces, Limit::default_of(256));
    assert_eq!(limits.max_bytes, Limit::default_of(16 * 1024 * 1024));
}

#[test]
fn names_resolve_to_the_namespaces_in_scope() {
    let xml = br#"<p:a xmlns:p="urn:p" xmlns="urn:d"><b p:c="1" d="2"/><q:e/></p:a>"#;
    let mut reader = XmlReader::open(xml, &XmlLimits::default()).unwrap();
    let mut buf = Vec::new();
    let Event::Start(a) = reader.read_event_into(&mut buf).unwrap() else {
        panic!("expected the root");
    };
    let (namespace, local) = reader.resolve_element(a.name());
    assert_eq!(
        (namespace, local.as_ref()),
        (ResolveResult::Bound(Namespace("urn:p")), "a")
    );
    let Event::Empty(b) = reader.read_event_into(&mut buf).unwrap() else {
        panic!("expected b");
    };
    let (namespace, _) = reader.resolve_element(b.name());
    assert_eq!(namespace, ResolveResult::Bound(Namespace("urn:d")));
    let mut attributes = b.attributes().map(Result::unwrap);
    let (namespace, local) = reader.resolve_attribute(attributes.next().unwrap().key);
    assert_eq!(
        (namespace, local.as_ref()),
        (ResolveResult::Bound(Namespace("urn:p")), "c")
    );
    let (namespace, _) = reader.resolve_attribute(attributes.next().unwrap().key);
    assert_eq!(namespace, ResolveResult::Unbound);
    let Event::Empty(e) = reader.read_event_into(&mut buf).unwrap() else {
        panic!("expected q:e");
    };
    let (namespace, _) = reader.resolve_element(e.name());
    assert!(matches!(namespace, ResolveResult::Unknown(_)));
}

#[test]
fn the_offset_is_where_the_event_read_last_starts() {
    let mut reader = XmlReader::open(b"<a><b/>text</a>", &XmlLimits::default()).unwrap();
    let mut buf = Vec::new();
    assert_eq!(reader.offset(), 0);
    let offsets: Vec<u64> = (0..4)
        .map(|_| {
            reader.read_event_into(&mut buf).unwrap();
            reader.offset()
        })
        .collect();
    assert_eq!(offsets, [0, 3, 7, 11]);
}
