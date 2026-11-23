//! The `Id` a relationship added takes, and that the `Target` it gets
//! resolves to the part when the package is read back.

use super::{RELS_NS, Text, contents, entries, media, open_loose, part, written_zip, zip};
use crate::{Link, LinkSource, PackageEdit};
use std::io::Cursor;

/// The edit of the tests' package whose workbook relationships hold `body`.
fn edit_with(body: &str) -> PackageEdit<'static, Cursor<Vec<u8>>> {
    let mut all = entries();
    all[4].1 = format!(r#"<Relationships xmlns="{RELS_NS}">{body}</Relationships>"#);
    PackageEdit::new(open_loose(zip(&all)))
}

fn link(source: LinkSource) -> Option<Link> {
    Some(Link {
        source,
        rel_type: "urn:t".to_owned(),
    })
}

fn external(id: &str) -> String {
    format!(
        r#"<Relationship Id="{id}" Type="urn:x" Target="https://example.com/{id}" TargetMode="External"/>"#
    )
}

#[test]
fn the_id_is_the_least_rid_number_no_relationship_of_the_source_has() {
    let body = format!(
        "{}{}{}{}",
        external("rId1"),
        external("rId3"),
        external("abc"),
        external("RID2")
    );
    let mut edit = edit_with(&body);
    let from = || link(LinkSource::Part(part("/xl/workbook.xml")));
    let mut ids = Vec::new();
    for index in 0..3 {
        let name = part(&format!("/new{index}.xml"));
        ids.push(
            edit.add(
                name,
                media("application/xml"),
                Box::new(Text("<x/>")),
                from(),
            )
            .unwrap(),
        );
    }
    let ids: Vec<Option<&str>> = ids.iter().map(Option::as_deref).collect();
    assert_eq!(ids, [Some("rId2"), Some("rId4"), Some("rId5")]);
    let bytes = written_zip(edit);
    let rels = &contents(&bytes)[4].1;
    assert!(
        rels.contains(r#"<Relationship Id="rId2" Type="urn:t" Target="../new0.xml"/>"#),
        "{rels}"
    );
    assert!(rels.ends_with(
        r#"<Relationship Id="rId5" Type="urn:t" Target="../new2.xml"/></Relationships>"#
    ));
}

#[test]
fn each_source_numbers_its_own_ids() {
    let mut edit = edit_with("");
    let id = |edit: &mut PackageEdit<'_, Cursor<Vec<u8>>>, name: &str, source: LinkSource| {
        edit.add(
            part(name),
            media("application/xml"),
            Box::new(Text("<x/>")),
            link(source),
        )
        .unwrap()
    };
    // The package's own relationships have rId1 and rId2; the sheet's has rId1.
    assert_eq!(
        id(&mut edit, "/a.xml", LinkSource::Package).as_deref(),
        Some("rId3")
    );
    assert_eq!(
        id(
            &mut edit,
            "/b.xml",
            LinkSource::Part(part("/xl/workbook.xml"))
        )
        .as_deref(),
        Some("rId1")
    );
    assert_eq!(
        id(
            &mut edit,
            "/c.xml",
            LinkSource::Part(part("/XL/worksheets/sheet1.xml"))
        )
        .as_deref(),
        Some("rId2")
    );
    assert_eq!(
        id(&mut edit, "/d.xml", LinkSource::Package).as_deref(),
        Some("rId4")
    );
    assert_eq!(
        id(
            &mut edit,
            "/e.xml",
            LinkSource::Part(part("/xl/workbook.xml"))
        )
        .as_deref(),
        Some("rId2")
    );
}

#[test]
fn the_target_of_each_relationship_resolves_to_the_part_added() {
    let mut edit = edit_with("");
    let names = [
        ("/xl/worksheets/sheet2.xml", "/xl/workbook.xml"),
        ("/xl/workbook2.xml", "/xl/workbook.xml"),
        ("/docProps/core.xml", "/xl/workbook.xml"),
        ("/xl/a:b.xml", "/xl/workbook.xml"),
        ("/シート/一.xml", "/xl/workbook.xml"),
        ("/xl/worksheets/a.xml", "/xl/worksheets/sheet1.xml"),
        ("/xl/c.xml", "/xl/worksheets/sheet1.xml"),
    ];
    for (name, source) in names {
        let link = link(LinkSource::Part(part(source)));
        edit.add(
            part(name),
            media("application/xml"),
            Box::new(Text("<x/>")),
            link,
        )
        .unwrap();
    }
    let package = open_loose(written_zip(edit));
    let targets = |source: &str| -> Vec<String> {
        let part = package.part(source).unwrap();
        part.relationships()
            .iter()
            .map(|relationship| match relationship.target() {
                xlsx_graft_package::Target::Internal(index) => {
                    package.parts()[*index].name().to_string()
                }
                xlsx_graft_package::Target::External(target) => target.clone(),
            })
            .collect()
    };
    assert_eq!(
        targets("/xl/workbook.xml"),
        [
            "/xl/worksheets/sheet2.xml",
            "/xl/workbook2.xml",
            "/docProps/core.xml",
            "/xl/a:b.xml",
            "/シート/一.xml"
        ]
    );
    assert_eq!(
        targets("/xl/worksheets/sheet1.xml"),
        ["https://example.com/", "/xl/worksheets/a.xml", "/xl/c.xml"]
    );
}
