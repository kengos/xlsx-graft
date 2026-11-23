//! `PackageEdit::remove`: the part, its Relationships part, their
//! `Override`s and the relationships that target it left out.

use super::{
    Copy, Text, content, contents, edit, entries, found, media, open, open_loose, part, reread,
    written_zip, zip,
};
use crate::{Link, LinkSource, PackageEdit};
use std::cell::Cell;
use std::rc::Rc;

fn names(bytes: &[u8]) -> Vec<String> {
    contents(bytes).into_iter().map(|(name, _)| name).collect()
}

#[test]
fn a_part_removed_leaves_out_its_entries_overrides_and_the_relationships_to_it() {
    let mut edit = edit();
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    let bytes = written_zip(edit);
    // Its entry and its Relationships part are gone; the others are as they were.
    let expected: Vec<String> = entries()
        .into_iter()
        .map(|(name, _)| name)
        .take(5)
        .collect();
    assert_eq!(names(&bytes), expected);
    let all = contents(&bytes);
    assert_eq!(all[1..4], entries()[1..4]);
    let types = entries()[0].1.replace(
        r#"<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.ms-excel.worksheet+xml"/>"#,
        "",
    );
    assert_eq!(all[0].1, types);
    // The relationship to the part is left out whole, as a start and an end tag.
    let rels = entries()[4].1.replace(
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"></Relationship>"#,
        "",
    );
    assert_eq!(all[4].1, rels);
    let package = open_loose(bytes);
    assert!(package.part("/xl/worksheets/sheet1.xml").is_none());
    assert!(
        package
            .part("/xl/workbook.xml")
            .unwrap()
            .relationships()
            .is_empty()
    );
}

#[test]
fn a_relationship_in_the_package_relationships_to_the_part_removed_is_left_out() {
    let mut edit = edit();
    edit.remove(&part("/docProps/app.xml")).unwrap();
    let bytes = written_zip(edit);
    assert_eq!(
        content(&bytes, "_rels/.rels")
            .matches("<Relationship ")
            .count(),
        1
    );
    assert!(!content(&bytes, "[Content_Types].xml").contains("app.xml"));
    let package = open(bytes);
    assert_eq!(package.relationships().len(), 1);
}

#[test]
fn removing_a_source_and_what_it_targets_leaves_out_both_without_changing_the_others() {
    let mut edit = edit();
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    edit.remove(&part("/xl/workbook.xml")).unwrap();
    let bytes = written_zip(edit);
    assert_eq!(
        names(&bytes),
        ["[Content_Types].xml", "_rels/.rels", "docProps/app.xml"]
    );
    let rels = content(&bytes, "_rels/.rels");
    assert_eq!(rels.matches("<Relationship ").count(), 1);
    assert!(rels.contains("docProps/app.xml"));
}

#[test]
fn a_part_a_removed_part_targets_is_kept() {
    let mut edit = edit();
    edit.remove(&part("/xl/workbook.xml")).unwrap();
    let bytes = written_zip(edit);
    assert!(names(&bytes).contains(&"xl/worksheets/sheet1.xml".to_owned()));
    assert!(content(&bytes, "[Content_Types].xml").contains("sheet1.xml"));
    // Its Relationships part and the workbook's are gone with their parts.
    assert_eq!(names(&bytes).len(), 5);
}

#[test]
fn a_removed_part_drops_its_rewriter_and_a_replace_of_it_is_refused() {
    let calls = Rc::new(Cell::new(0));
    let mut edit = edit();
    edit.replace(
        &part("/xl/worksheets/sheet1.xml"),
        Box::new(Copy(calls.clone())),
    )
    .unwrap();
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    assert!(
        edit.replace(&part("/xl/worksheets/sheet1.xml"), Box::new(Text("<x/>")))
            .is_err()
    );
    written_zip(edit);
    assert_eq!(calls.get(), 0);
}

#[test]
fn a_link_from_a_part_removed_is_not_written_and_the_part_added_is_written() {
    let mut edit = edit();
    let from = |name: &str| Link {
        source: LinkSource::Part(part(name)),
        rel_type: "urn:t".to_owned(),
    };
    // A source with a Relationships part, and one with none.
    edit.add(
        part("/a.xml"),
        media("application/xml"),
        Box::new(Text("<a/>")),
        Some(from("/xl/worksheets/sheet1.xml")),
    )
    .unwrap();
    edit.add(
        part("/b.xml"),
        media("application/xml"),
        Box::new(Text("<b/>")),
        Some(from("/docProps/app.xml")),
    )
    .unwrap();
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    edit.remove(&part("/docProps/app.xml")).unwrap();
    let bytes = written_zip(edit);
    let last: Vec<String> = names(&bytes).into_iter().rev().take(2).collect();
    assert_eq!(last, ["b.xml", "a.xml"]);
    assert!(
        !names(&bytes)
            .iter()
            .any(|name| name.contains("_rels/app.xml.rels"))
    );
    assert_eq!(
        found(&reread(&bytes)),
        ["part_unreferenced /a.xml", "part_unreferenced /b.xml"]
    );
}

#[test]
fn the_names_of_a_part_removed_and_its_relationships_part_can_be_a_folder_of_a_part_added() {
    let mut edit = edit();
    // The names stay taken until the part is removed.
    for name in ["/xl/worksheets/sheet1.xml/x.xml", "/xl/worksheets/_rels"] {
        assert!(
            edit.add(
                part(name),
                media("application/xml"),
                Box::new(Text("<x/>")),
                None
            )
            .is_err()
        );
    }
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    for name in ["/xl/worksheets/sheet1.xml/x.xml", "/xl/worksheets/_rels"] {
        edit.add(
            part(name),
            media("application/xml"),
            Box::new(Text("<x/>")),
            None,
        )
        .unwrap();
    }
    let bytes = written_zip(edit);
    assert_eq!(names(&bytes).len(), 7);
}

#[test]
fn a_relationship_whose_id_has_white_space_around_it_is_left_out_with_its_target() {
    let mut all = entries();
    all[4].1 = all[4].1.replace(r#"Id="rId1""#, r#"Id=" rId1 ""#);
    let mut edit = PackageEdit::new(open(zip(&all)));
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    let bytes = written_zip(edit);
    assert!(!content(&bytes, "xl/_rels/workbook.xml.rels").contains("sheet1.xml"));
    assert_eq!(found(&reread(&bytes)), Vec::<String>::new());
}
