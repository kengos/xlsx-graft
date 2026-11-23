//! `PackageEdit::add`: a part written at the end with an `Override`, with
//! and without a relationship to it, in the content types stream and
//! Relationships parts of every shape the package may have.

use super::{
    DECLARATION, RELS_NS, Refuse, TYPES_NS, Text, content, contents, edit, entries, finish, found,
    media, open, open_loose, part, reread, written_zip, zip,
};
use crate::{Link, LinkSource, PackageEdit};
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package, Target};

const SHEET: &str = "application/vnd.ms-excel.worksheet+xml";
const WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";

#[test]
fn a_part_added_is_written_after_the_others_with_an_override() {
    let mut edit = edit();
    let id = edit
        .add(
            part("/xl/worksheets/sheet2.xml"),
            media(SHEET),
            Box::new(Text("<sheet2/>")),
            None,
        )
        .unwrap();
    assert_eq!(id, None);
    let bytes = written_zip(edit);
    let all = contents(&bytes);
    assert_eq!(all[1..7], entries()[1..]);
    // The other entries are unchanged but the content types stream.
    let types = &entries()[0].1;
    let expected = types.replace(
        "</Types>",
        &format!(
            r#"<Override PartName="/xl/worksheets/sheet2.xml" ContentType="{SHEET}"/></Types>"#
        ),
    );
    assert_eq!(all[0].1, expected);
    assert_eq!(
        all[7],
        (
            "xl/worksheets/sheet2.xml".to_owned(),
            "<sheet2/>".to_owned()
        )
    );
    assert_eq!(all.len(), 8);
    // Read back: the part has its type and, with no relationship to it, is
    // unreferenced.
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(Cursor::new(bytes), &Limits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    assert_eq!(
        found(&diagnostics),
        ["part_unreferenced /xl/worksheets/sheet2.xml"]
    );
    let added = package.part("/xl/worksheets/sheet2.xml").unwrap();
    assert_eq!(added.content_type().unwrap().as_str(), SHEET);
}

#[test]
fn parts_added_are_written_in_the_order_they_were_added() {
    let mut edit = edit();
    for name in ["/b.xml", "/a.xml", "/c.xml"] {
        edit.add(
            part(name),
            media("application/xml"),
            Box::new(Text("<x/>")),
            None,
        )
        .unwrap();
    }
    let all = contents(&written_zip(edit));
    let names: Vec<&str> = all[7..].iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["b.xml", "a.xml", "c.xml"]);
}

#[test]
fn a_part_with_a_link_from_a_part_that_has_relationships_gets_a_relationship() {
    let mut edit = edit();
    let link = Link {
        source: LinkSource::Part(part("/xl/workbook.xml")),
        rel_type: WORKSHEET.to_owned(),
    };
    let id = edit
        .add(
            part("/xl/worksheets/sheet2.xml"),
            media(SHEET),
            Box::new(Text("<sheet2/>")),
            Some(link),
        )
        .unwrap();
    assert_eq!(id.as_deref(), Some("rId2"));
    let bytes = written_zip(edit);
    let expected = entries()[4].1.replace(
        "</Relationships>",
        &format!(r#"<Relationship Id="rId2" Type="{WORKSHEET}" Target="worksheets/sheet2.xml"/></Relationships>"#),
    );
    assert_eq!(content(&bytes, "xl/_rels/workbook.xml.rels"), expected);
    // Read back, no new Relationships part, and the new relationship
    // targets the new part.
    assert_eq!(contents(&bytes).len(), 8);
    let package = open(bytes);
    let workbook = package.part("/xl/workbook.xml").unwrap();
    let relationship = &workbook.relationships()[1];
    assert_eq!(relationship.id(), "rId2");
    assert_eq!(relationship.rel_type(), WORKSHEET);
    let Target::Internal(index) = relationship.target() else { panic!("{relationship:?}") };
    assert_eq!(
        package.parts()[*index].name().as_str(),
        "/xl/worksheets/sheet2.xml"
    );
}

#[test]
fn a_part_with_a_link_from_the_package_gets_a_relationship_in_the_package_relationships() {
    let mut edit = edit();
    let link = Link {
        source: LinkSource::Package,
        rel_type: "urn:extra".to_owned(),
    };
    let id = edit
        .add(
            part("/customXml/item1.xml"),
            media("application/xml"),
            Box::new(Text("<i/>")),
            Some(link),
        )
        .unwrap();
    assert_eq!(id.as_deref(), Some("rId3"));
    let bytes = written_zip(edit);
    let expected = entries()[1].1.replace(
        "</Relationships>",
        r#"<Relationship Id="rId3" Type="urn:extra" Target="customXml/item1.xml"/></Relationships>"#,
    );
    assert_eq!(content(&bytes, "_rels/.rels"), expected);
    open(bytes);
}

#[test]
fn a_link_from_a_part_with_no_relationships_part_makes_one_after_the_added_parts() {
    let mut edit = edit();
    let from = |name: &str| Link {
        source: LinkSource::Part(part(name)),
        rel_type: "urn:t".to_owned(),
    };
    // Two sources with none, the second linked first, and a part that has one.
    let one = edit
        .add(
            part("/media/one.xml"),
            media("application/xml"),
            Box::new(Text("<1/>")),
            Some(from("/docProps/app.xml")),
        )
        .unwrap();
    let two = edit
        .add(
            part("/media/two.xml"),
            media("application/xml"),
            Box::new(Text("<2/>")),
            Some(from("/xl/workbook.xml")),
        )
        .unwrap();
    let three = edit
        .add(
            part("/media/three.xml"),
            media("application/xml"),
            Box::new(Text("<3/>")),
            Some(from("/docProps/app.xml")),
        )
        .unwrap();
    assert_eq!(
        (one.as_deref(), two.as_deref(), three.as_deref()),
        (Some("rId1"), Some("rId2"), Some("rId2"))
    );
    let bytes = written_zip(edit);
    let all = contents(&bytes);
    let names: Vec<&str> = all[7..].iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "media/one.xml",
            "media/two.xml",
            "media/three.xml",
            "docProps/_rels/app.xml.rels"
        ]
    );
    let rels = format!(
        concat!(
            "{decl}",
            r#"<Relationships xmlns="{ns}">"#,
            r#"<Relationship Id="rId1" Type="urn:t" Target="../media/one.xml"/>"#,
            r#"<Relationship Id="rId2" Type="urn:t" Target="../media/three.xml"/>"#,
            "</Relationships>"
        ),
        decl = DECLARATION,
        ns = RELS_NS
    );
    assert_eq!(all[10].1, rels);
    // The new Relationships part has an Override, after those of the parts.
    assert!(all[0].1.ends_with(concat!(
        r#"<Override PartName="/docProps/_rels/app.xml.rels" "#,
        r#"ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>"#
    )));
    let package = open(bytes);
    let app = package.part("/docProps/app.xml").unwrap();
    assert_eq!(app.relationships().len(), 2);
    assert!(package.part("/docProps/_rels/app.xml.rels").is_some());
}

/// The edit of a package whose content types stream and relationships
/// parts are `types`, `package_rels` and `workbook_rels`, as written.
fn edit_of(
    types: &str,
    package_rels: &str,
    workbook_rels: &str,
) -> PackageEdit<'static, Cursor<Vec<u8>>> {
    let mut all = entries();
    all[0].1 = types.to_owned();
    all[1].1 = package_rels.to_owned();
    all[4].1 = workbook_rels.to_owned();
    PackageEdit::new(open_loose(zip(&all)))
}

#[test]
fn a_root_with_a_prefix_gets_its_children_with_the_same_prefix() {
    let office =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
    let types = format!(
        concat!(
            r#"<ct:Types xmlns:ct="{TYPES_NS}"><ct:Default Extension="xml" ContentType="application/xml"/>"#,
            r#"<ct:Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
            r#"</ct:Types>"#
        ),
        TYPES_NS = TYPES_NS
    );
    let package_rels = format!(
        r#"<r:Relationships xmlns:r="{RELS_NS}"><r:Relationship Id="rId1" Type="{office}" Target="xl/workbook.xml"/></r:Relationships>"#
    );
    let workbook_rels = format!(r#"<r:Relationships xmlns:r="{RELS_NS}"/>"#);
    let mut edit = edit_of(&types, &package_rels, &workbook_rels);
    let link = Link {
        source: LinkSource::Part(part("/xl/workbook.xml")),
        rel_type: "urn:t".to_owned(),
    };
    edit.add(
        part("/xl/a.xml"),
        media("application/xml"),
        Box::new(Text("<a/>")),
        Some(link),
    )
    .unwrap();
    let link = Link {
        source: LinkSource::Package,
        rel_type: "urn:p".to_owned(),
    };
    edit.add(
        part("/b.xml"),
        media("application/xml"),
        Box::new(Text("<b/>")),
        Some(link),
    )
    .unwrap();
    let bytes = written_zip(edit);
    let all = contents(&bytes);
    assert_eq!(
        all[0].1,
        types.replace(
            "</ct:Types>",
            concat!(
                r#"<ct:Override PartName="/xl/a.xml" ContentType="application/xml"/>"#,
                r#"<ct:Override PartName="/b.xml" ContentType="application/xml"/></ct:Types>"#
            )
        )
    );
    assert_eq!(
        all[1].1,
        package_rels.replace(
            "</r:Relationships>",
            r#"<r:Relationship Id="rId2" Type="urn:p" Target="b.xml"/></r:Relationships>"#
        )
    );
    // An empty root becomes a root with children.
    assert_eq!(
        all[4].1,
        format!(
            r#"<r:Relationships xmlns:r="{RELS_NS}"><r:Relationship Id="rId1" Type="urn:t" Target="a.xml"/></r:Relationships>"#
        )
    );
    let package = open_loose(bytes);
    assert_eq!(
        package
            .part("/xl/workbook.xml")
            .unwrap()
            .relationships()
            .len(),
        1
    );
}

#[test]
fn what_is_written_into_attributes_is_escaped_and_reads_back() {
    let mut edit = edit();
    let link = Link {
        source: LinkSource::Package,
        rel_type: r#"urn:a&b<"c">"#.to_owned(),
    };
    edit.add(
        part("/x&y.xml"),
        media("application/x-a;b=\"&\""),
        Box::new(Text("<x/>")),
        Some(link),
    )
    .unwrap();
    let bytes = written_zip(edit);
    assert!(content(&bytes, "[Content_Types].xml").contains(
        r#"<Override PartName="/x&amp;y.xml" ContentType="application/x-a;b=&quot;&amp;&quot;"/>"#
    ));
    let package = open(bytes);
    let relationship = &package.relationships()[2];
    assert_eq!(relationship.rel_type(), r#"urn:a&b<"c">"#);
    assert_eq!(
        package
            .part("/x&y.xml")
            .unwrap()
            .content_type()
            .unwrap()
            .as_str(),
        "application/x-a;b=\"&\""
    );
}

#[test]
fn a_part_refused_by_its_writer_gives_its_diagnostic_at_the_part_and_no_zip() {
    let mut edit = edit();
    edit.add(
        part("/new.xml"),
        media("application/xml"),
        Box::new(Refuse),
        None,
    )
    .unwrap();
    let (bytes, diagnostics) = finish(edit);
    assert!(bytes.is_none());
    assert_eq!(found(&diagnostics), ["xml_doctype /new.xml"]);
}

#[test]
fn links_from_a_source_spelled_in_two_letter_cases_make_one_relationships_part() {
    let mut edit = edit();
    let from = |name: &str| Link {
        source: LinkSource::Part(part(name)),
        rel_type: "urn:t".to_owned(),
    };
    let one = edit
        .add(
            part("/docProps/a.xml"),
            media("application/xml"),
            Box::new(Text("<a/>")),
            Some(from("/docProps/app.xml")),
        )
        .unwrap();
    let two = edit
        .add(
            part("/docProps/b.xml"),
            media("application/xml"),
            Box::new(Text("<b/>")),
            Some(from("/DOCPROPS/APP.XML")),
        )
        .unwrap();
    assert_eq!(
        (one.as_deref(), two.as_deref()),
        (Some("rId1"), Some("rId2"))
    );
    let bytes = written_zip(edit);
    let all = contents(&bytes);
    let rels: Vec<&str> = all
        .iter()
        .map(|(n, _)| n.as_str())
        .filter(|n| n.ends_with(".rels"))
        .collect();
    assert_eq!(rels.len(), 4);
    assert_eq!(all[0].1.matches("_rels/app.xml.rels").count(), 1);
    assert_eq!(all.last().unwrap().0, "docProps/_rels/app.xml.rels");
    assert_eq!(found(&reread(&bytes)), Vec::<String>::new());
}
