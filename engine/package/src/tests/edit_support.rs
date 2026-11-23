//! What a caller that edits a package asks of it: the keys of the names that
//! are taken but are no part, the key an `Override` has, the name of a
//! Relationships part, and whether the content types stream can type a part
//! added.

use super::builder::Item;
use super::opc::{DEFAULTS, clean, minimal, package_rels, part, read, types, with};
use crate::{PartName, collapse, override_key};

#[test]
fn the_keys_of_entries_that_are_no_part_are_listed() {
    let package = read(with(vec![
        part("xl/a.xml"),
        part("XL/A.XML"),
        part("xl/ok.xml"),
    ]))
    .0
    .unwrap();
    let keys: Vec<&str> = package.unknown_keys().collect();
    assert_eq!(keys, ["/xl/a.xml"]);
    assert_eq!(clean(minimal()).unknown_keys().count(), 0);
}

#[test]
fn an_override_key_is_the_key_the_reader_gives_the_name() {
    // A name collapsed, read as a part name and keyed.
    assert_eq!(override_key(" /XL/A.xml\n"), "/xl/a.xml");
    assert_eq!(override_key("/xl/%C3%A9.xml"), "/xl/é.xml");
    // A value that is no part name has the shadow key.
    assert_eq!(override_key("xl/New.xml"), "/xl/new.xml");
    // The reader refuses the part whose name the key names, whichever it is.
    let body = r#"<Override PartName="xl/new.xml" ContentType="application/xml"/>"#;
    let package = read(vec![
        types(&format!("{DEFAULTS}{body}")),
        package_rels(""),
        part("xl/workbook.xml"),
        part("xl/new.xml"),
    ])
    .0
    .unwrap();
    assert!(package.part("/xl/new.xml").unwrap().content_type_unknown());
}

#[test]
fn a_part_has_its_relationships_part_name_and_the_package_has_its_own() {
    let name = |value: &str| PartName::parse(value).unwrap();
    assert_eq!(
        name("/xl/workbook.xml").relationships_part().as_str(),
        "/xl/_rels/workbook.xml.rels"
    );
    assert_eq!(
        name("/A.xml").relationships_part().as_str(),
        "/_rels/A.xml.rels"
    );
    assert_eq!(
        name("/a/シ.xml").relationships_part().key(),
        "/a/_rels/シ.xml.rels"
    );
    assert!(
        name("/xl/workbook.xml")
            .relationships_part()
            .is_relationships()
    );
    assert_eq!(PartName::package_relationships().as_str(), "/_rels/.rels");
    assert!(PartName::package_relationships().is_relationships());
    // The Relationships part reads back as the part's.
    let rels = name("/xl/workbook.xml").relationships_part();
    assert_eq!(
        rels.relationships_source(),
        Some(crate::part_name::Source::Part(
            "/xl/workbook.xml".to_owned()
        ))
    );
}

#[test]
fn the_content_types_stream_is_given_when_it_was_read_and_every_mapping_said_what_it_names() {
    assert_eq!(clean(minimal()).content_types_entry(), Some(0));
    let without = vec![package_rels(""), part("xl/workbook.xml")];
    assert_eq!(read(without).0.unwrap().content_types_entry(), None);
    // Two streams, a stream that is not well-formed, one with a mapping that
    // names nothing.
    let two = with(vec![types(DEFAULTS)]);
    assert_eq!(read(two).0.unwrap().content_types_entry(), None);
    let mut broken = minimal();
    broken[0] = Item::stored("[Content_Types].xml", b"<Types");
    assert_eq!(read(broken).0.unwrap().content_types_entry(), None);
    let wild = vec![
        types(&format!(
            r#"{DEFAULTS}<Override ContentType="application/xml"/>"#
        )),
        package_rels(""),
        part("xl/workbook.xml"),
    ];
    assert_eq!(read(wild).0.unwrap().content_types_entry(), None);
}

#[test]
fn white_space_is_collapsed_as_xml_schema_does() {
    assert_eq!(collapse(" a \t b\r\n\nc "), "a b c");
    assert_eq!(collapse(""), "");
}
