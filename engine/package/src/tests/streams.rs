//! The XML streams against their schemas: a root that is not the schema's,
//! attributes, elements and text where they may not stand, the attributes a
//! relationship must have and may not, a duplicate `Id`, and the
//! relationship limit.

use super::builder::Item;
use super::limit;
use super::opc::{
    DEFAULTS, OFFICE_DOCUMENT, RELS_NS, TYPES_NS, at, found, minimal, package_rels, part,
    read_with, rel,
};
use crate::{Limits, Target};
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};

/// An `xml_schema_invalid` without its offset, and where it points.
type Placed = ((SchemaReason, String), String);

/// `xml_schema_invalid`, its offset left out.
fn schema(reason: SchemaReason, name: &str) -> (SchemaReason, String) {
    (reason, name.to_owned())
}

/// The `xml_schema_invalid` diagnostics in `seen`, without their offsets,
/// with where each points; and how many other diagnostics there were.
fn schemas(seen: Vec<(Diagnostic, String)>) -> (Vec<Placed>, usize) {
    let mut others = 0;
    let found = seen
        .into_iter()
        .filter_map(|(diagnostic, place)| match diagnostic {
            Diagnostic::XmlSchemaInvalid { reason, name, .. } => {
                Some(((reason, name.as_str().to_owned()), place))
            }
            _ => {
                others += 1;
                None
            }
        })
        .collect();
    (found, others)
}

/// The minimal package with `stream` as its content types stream.
fn with_types(stream: &str) -> Vec<Item> {
    let mut items = minimal();
    items[0] = Item::stored("[Content_Types].xml", stream.as_bytes());
    items
}

/// The minimal package whose package relationships are `body`, after the
/// main document's relationship.
fn with_rels(body: &str) -> Vec<Item> {
    let mut items = minimal();
    items[1] = package_rels(body);
    items
}

const STREAM: &str = "[Content_Types].xml";

#[test]
fn a_root_of_another_name_or_namespace_ends_the_reading() {
    let wrong = |stream: &str| schemas(found(with_types(stream)));
    let unexpected = SchemaReason::UnexpectedElement;
    // The stream is not read, so no part is reported for having no type.
    assert_eq!(
        wrong(&format!(r#"<Typez xmlns="{TYPES_NS}"/>"#)),
        (vec![(schema(unexpected, "Typez"), STREAM.into())], 0)
    );
    assert_eq!(
        wrong(r#"<Types xmlns="urn:other"/>"#),
        (vec![(schema(unexpected, "Types"), STREAM.into())], 0)
    );
    assert_eq!(
        wrong("<Types/>"),
        (vec![(schema(unexpected, "Types"), STREAM.into())], 0)
    );
    // Relationships not read: neither the main document nor an
    // unreferenced part is known.
    let mut items = with_rels("");
    items[1] = Item::stored("_rels/.rels", b"<Relationships/>");
    items.push(part("a.xml"));
    assert_eq!(
        schemas(found(items)),
        (
            vec![(schema(unexpected, "Relationships"), "/_rels/.rels".into())],
            0
        )
    );
}

#[test]
fn what_may_not_stand_in_the_root_or_a_child_is_reported_and_left_out() {
    let stream = format!(
        r#"<t:Types xmlns="{TYPES_NS}" xmlns:t="{TYPES_NS}" xmlns:x="urn:x" a="1" x:b="2">
             text{DEFAULTS}&amp;<![CDATA[]]>
             <Default Extension="bin" ContentType="a/b" c="3"/>
             <Default Extension="bin" ContentType="a/b"><X/></Default>
             <Default Extension="bin" ContentType="a/b"><X><Y/>deep</X></Default>
             <Default Extension="bin" ContentType="a/b">text</Default>
             <Default Extension="bin" ContentType="a/b"><![CDATA[x]]></Default>
             <Default Extension="bin" ContentType="a/b">&amp;</Default>
             <Unknown><Default/>text</Unknown>
             <x:Default Extension="bin" ContentType="a/b"/>
             <t:Default xmlns:y="urn:y" Extension="dat" ContentType="a/b"> <![CDATA[]]> </t:Default>
           </t:Types>"#
    );
    let mut items = with_types(&stream);
    items.push(part("a.bin"));
    items.push(part("a.dat"));
    let (found, others) = schemas(found(items));
    let place = |s| (s, STREAM.to_owned());
    use SchemaReason::*;
    assert_eq!(
        found,
        vec![
            place(schema(UnexpectedAttribute, "a")),
            place(schema(UnexpectedAttribute, "x:b")),
            place(schema(UnexpectedText, "Types")),
            place(schema(UnexpectedText, "Types")),
            place(schema(UnexpectedAttribute, "c")),
            place(schema(UnexpectedElement, "X")),
            place(schema(UnexpectedElement, "X")),
            place(schema(UnexpectedText, "Default")),
            place(schema(UnexpectedText, "Default")),
            place(schema(UnexpectedText, "Default")),
            place(schema(UnexpectedElement, "Unknown")),
            place(schema(UnexpectedElement, "x:Default")),
        ]
    );
    // Two parts are unreferenced. A refused child may have typed a.bin, so
    // it is not reported; a.dat has its type.
    assert_eq!(others, 2);
}

#[test]
fn a_relationship_must_have_its_attributes_and_no_others() {
    let body = concat!(
        r#"<Relationship Type="urn:t" Target="a.xml"/>"#,
        r#"<Relationship Id="r3" Target="a.xml"/>"#,
        r#"<Relationship Id="r4" Type="urn:t"/>"#,
        r#"<Relationship Id="r5" Type="urn:t" Target="a.xml" TargetMode="internal"/>"#,
        r#"<Relationship Id="r6" Type="urn:t" Target="a.xml" xml:base="/x/"/>"#,
        r#"<Relationship Id="r7" Type="urn:t" Target="a.xml" TargetMode="Internal">note</Relationship>"#,
        r#"<Relationship Id="r8" Type="urn:t" Target="http://a/" TargetMode="External"/>"#,
    );
    let mut items = with_rels(body);
    items.push(part("a.xml"));
    let (package, seen) = super::opc::read(items);
    let (found, others) = schemas(seen);
    let place = |s| (s, "/_rels/.rels".to_owned());
    use SchemaReason::*;
    assert_eq!(
        found,
        vec![
            place(schema(MissingAttribute, "Id")),
            place(schema(MissingAttribute, "Type")),
            place(schema(MissingAttribute, "Target")),
            place(schema(InvalidValue, "TargetMode")),
            place(schema(UnexpectedAttribute, "xml:base")),
        ]
    );
    assert_eq!(others, 0);
    let package = package.unwrap();
    let kept: Vec<(&str, &Target)> = (package.relationships().iter())
        .map(|r| (r.id(), r.target()))
        .collect();
    assert_eq!(kept.len(), 3);
    assert_eq!(kept[1].0, "r7");
    assert_eq!(kept[2], ("r8", &Target::External("http://a/".to_owned())));
}

#[test]
fn relationships_that_share_an_id_are_none_of_them_read() {
    let body = format!(
        "{}{}{}",
        rel("r2", "a.xml"),
        rel("r2", "b.xml"),
        rel("r2", "c.xml")
    );
    let mut items = with_rels(&body);
    items.extend([part("a.xml"), part("b.xml"), part("c.xml")]);
    let seen = found(items);
    // A relationship left out may have targeted them: none is reported.
    let duplicate = Diagnostic::RelationshipIdDuplicate {};
    assert_eq!(
        seen,
        vec![
            at(duplicate.clone(), "/_rels/.rels#r2"),
            at(duplicate, "/_rels/.rels#r2"),
        ]
    );
}

#[test]
fn a_package_relationship_left_out_reports_no_main_document_or_unreferenced_part() {
    let main = format!(
        r#"<Relationship Id="rId1" Type="{OFFICE_DOCUMENT}" Target="xl/workbook.xml" foo="x"/>"#
    );
    let mut items = minimal();
    items[1] = super::opc::rels("_rels/.rels", &main);
    let (found, others) = schemas(found(items));
    let unexpected = schema(SchemaReason::UnexpectedAttribute, "foo");
    assert_eq!(found, vec![(unexpected, "/_rels/.rels".into())]);
    assert_eq!(others, 0);
}

#[test]
fn ids_and_uris_are_collapsed_and_an_id_must_be_a_name() {
    let body = concat!(
        r#"<Relationship Id=" r2 " Type=" urn:t " Target="  a.xml "/>"#,
        r#"<Relationship Id="1" Type="urn:t" Target="a.xml"/>"#,
        r#"<Relationship Id="r:3" Type="urn:t" Target="a.xml"/>"#,
        r#"<Relationship Id="" Type="urn:t" Target="a.xml"/>"#,
        r#"<Relationship Id="_é-4.x" Type="urn:t" Target="a.xml"/>"#,
    );
    let mut items = with_rels(body);
    items[0] = super::opc::types(&format!(
        r#"{DEFAULTS}<Override PartName=" /a.xml " ContentType="a/b"/>"#
    ));
    items.push(part("a.xml"));
    let (package, seen) = super::opc::read(items);
    let (found, others) = schemas(seen);
    let invalid = (
        schema(SchemaReason::InvalidValue, "Id"),
        "/_rels/.rels".to_owned(),
    );
    assert_eq!(found, vec![invalid.clone(), invalid.clone(), invalid]);
    assert_eq!(others, 0);
    let package = package.unwrap();
    let ids: Vec<&str> = package.relationships().iter().map(|r| r.id()).collect();
    assert_eq!(ids, ["rId1", "r2", "_é-4.x"]);
    assert_eq!(package.relationships()[1].rel_type(), "urn:t");
    let a = package.part("/a.xml").unwrap();
    assert_eq!(a.content_type().unwrap().essence(), "a/b");
    assert_eq!(a.incoming().len(), 2);
}

#[test]
fn a_relationships_part_is_held_to_the_relationship_limit() {
    let body = format!(
        "{}{}",
        rel("r2", "xl/workbook.xml"),
        rel("r3", "xl/workbook.xml")
    );
    let mut limits = Limits {
        max_relationships: limit(3),
        ..Limits::default()
    };
    let (package, seen) = read_with(with_rels(&body), &limits);
    assert_eq!(seen, vec![]);
    assert_eq!(package.unwrap().relationships().len(), 3);
    limits.max_relationships = limit(2);
    let mut items = with_rels(&body);
    items.push(part("a.xml"));
    let (package, seen) = read_with(items, &limits);
    let too_many = Diagnostic::RelationshipsTooMany {
        count: 3,
        limit: limit(2),
    };
    assert_eq!(seen, vec![at(too_many, "/_rels/.rels")]);
    assert!(package.unwrap().relationships().is_empty());
}

#[test]
fn a_relationships_part_the_xml_reader_refuses_is_not_read() {
    let doctype = format!(
        r#"<!DOCTYPE r><Relationships xmlns="{RELS_NS}"><Relationship Id="rId1" Type="{OFFICE_DOCUMENT}" Target="xl/workbook.xml"/></Relationships>"#
    );
    let mut items = minimal();
    items[1] = Item::stored("_rels/.rels", doctype.as_bytes());
    items.push(part("a.xml"));
    let seen = found(items);
    assert_eq!(
        seen,
        vec![at(Diagnostic::XmlDoctype { offset: 0 }, "/_rels/.rels")]
    );
}

#[test]
fn a_stream_the_xml_reader_cannot_open_is_not_read() {
    let mut items = minimal();
    items[0] = Item::stored(
        "[Content_Types].xml",
        br#"<?xml version="1.0" encoding="UTF-7"?><Types/>"#,
    );
    let seen = found(items);
    let unsupported = Diagnostic::XmlEncodingUnsupported {
        encoding: xlsx_graft_diagnostics::Echo::new("UTF-7"),
        reason: xlsx_graft_diagnostics::EncodingReason::OtherEncoding,
    };
    assert_eq!(seen, vec![at(unsupported, STREAM)]);
}
