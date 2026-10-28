//! Mapping zip entries to parts: names that are not part names, folders,
//! names that are the same part or derived from another, and what a
//! package's parts give back.

use super::builder::Item;
use super::opc::{DEFAULTS, at, clean, found, minimal, part, types, with};
use crate::{Incoming, RelationshipSource, Target};
use std::io::Read;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Limit, PartNameReason};

/// `entry_name_invalid` for `reason`.
fn invalid(reason: PartNameReason) -> Diagnostic {
    Diagnostic::EntryNameInvalid { reason }
}

/// `part_name_duplicate`, naming `other`.
fn duplicate(other: &str) -> Diagnostic {
    Diagnostic::PartNameDuplicate {
        other: Echo::new(other),
    }
}

/// `item`, named `name` in both headers.
fn named(mut item: Item, name: &[u8]) -> Item {
    item.local.name = name.to_vec();
    item.central.name = name.to_vec();
    item
}

#[test]
fn entries_whose_names_are_not_part_names_are_not_parts() {
    let seen = found(with(vec![
        part("a\\b.xml"),
        part("dir/"),
        Item::stored("folder/", b""),
        part("%41.xml"),
        part("[trash]/0000.dat"),
        named(part("x"), b"\xff.xml"),
        part("a..xml/b."),
    ]));
    assert_eq!(
        seen,
        vec![
            at(invalid(PartNameReason::InvalidCharacter), "a\\b.xml"),
            at(invalid(PartNameReason::EmptySegment), "dir/"),
            at(invalid(PartNameReason::EncodedUnreserved), "%41.xml"),
            at(
                invalid(PartNameReason::InvalidCharacter),
                "[trash]/0000.dat"
            ),
            at(invalid(PartNameReason::NotUtf8), "\u{fffd}.xml"),
            at(invalid(PartNameReason::EndsWithDot), "a..xml/b."),
        ]
    );
}

#[test]
fn names_that_are_the_same_part_are_none_of_them_parts() {
    let (package, seen) =
        super::opc::read(with(vec![part("XL/workbook.xml"), part("xl/workbook.xml")]));
    // The workbook the package relationships target is refused, not missing.
    assert_eq!(
        seen,
        vec![
            at(duplicate("xl/workbook.xml"), "XL/workbook.xml"),
            at(duplicate("xl/workbook.xml"), "xl/workbook.xml"),
        ]
    );
    let package = package.unwrap();
    let names: Vec<&str> = package.parts().iter().map(|p| p.name().as_str()).collect();
    assert_eq!(names, ["/_rels/.rels"]);
    assert!(package.part("/xl/workbook.xml").is_none());
}

#[test]
fn a_name_derived_from_another_is_not_a_part_and_the_other_is() {
    let seen = found(with(vec![part("xl/Workbook.xml/sheet1.xml")]));
    let derived = Diagnostic::PartNameDerived {
        other: Echo::new("/xl/workbook.xml"),
    };
    assert_eq!(seen, vec![at(derived, "/xl/Workbook.xml/sheet1.xml")]);
}

#[test]
fn a_second_content_types_stream_refuses_both_and_types_no_part() {
    let mut items = minimal();
    items.push(named(types(DEFAULTS), b"[content_types].XML"));
    items.push(part("a.bin"));
    let (package, seen) = super::opc::read(items);
    assert!(
        package
            .unwrap()
            .parts()
            .iter()
            .all(|p| p.content_type().is_none())
    );
    assert_eq!(
        seen,
        vec![
            at(duplicate("[Content_Types].xml"), "[content_types].XML"),
            at(Diagnostic::PartUnreferenced {}, "/a.bin"),
        ]
    );
}

#[test]
fn an_entry_the_zip_refused_reports_nothing_more_but_its_name_still_counts() {
    let unsupported = Diagnostic::ZipUnsupportedCompression { method: 12 };
    let seen = found(with(vec![
        part("a\\b.xml").with_method(12),
        part("xl/WORKBOOK.xml").with_method(12),
        part("xl/workbook.xml/derived.xml").with_method(12),
        part("unreferenced.xml").with_method(12),
    ]));
    assert_eq!(
        seen,
        vec![
            at(unsupported.clone(), "a\\b.xml"),
            at(unsupported.clone(), "xl/WORKBOOK.xml"),
            at(unsupported.clone(), "xl/workbook.xml/derived.xml"),
            at(unsupported, "unreferenced.xml"),
            // The readable one says why it is not a part.
            at(duplicate("xl/WORKBOOK.xml"), "xl/workbook.xml"),
        ]
    );
}

#[test]
fn a_package_gives_back_its_parts_and_their_relationships() {
    let mut package = clean(minimal());
    let names: Vec<&str> = package.parts().iter().map(|p| p.name().as_str()).collect();
    assert_eq!(names, ["/_rels/.rels", "/xl/workbook.xml"]);
    let workbook = package.part("/XL/Workbook.xml").unwrap();
    assert_eq!(package.part_named(workbook.name()), Some(workbook));
    assert_eq!(workbook.entry(), 2);
    assert_eq!(
        workbook.content_type().unwrap().essence(),
        "application/xml"
    );
    assert!(workbook.relationships().is_empty());
    let incoming = Incoming {
        source: RelationshipSource::Package,
        index: 0,
    };
    assert_eq!(workbook.incoming(), [incoming]);
    assert!(package.part("no/slash").is_none());
    assert!(package.part("/missing.xml").is_none());
    let relationship = &package.relationships()[0];
    assert_eq!(relationship.id(), "rId1");
    assert_eq!(relationship.rel_type(), super::opc::OFFICE_DOCUMENT);
    assert_eq!(relationship.target(), &Target::Internal(1));
    assert_eq!(package.archive().entries().len(), 3);
    let mut diagnostics = Diagnostics::new();
    let mut bytes = Vec::new();
    let mut reader = package.open_part(1, &mut diagnostics).unwrap().unwrap();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"<a/>");
    assert!(package.open_part(2, &mut diagnostics).unwrap().is_none());
    let read = package.read_xml_part(1, Limit::default_of(4), &mut diagnostics);
    assert_eq!(read.unwrap(), Some(b"<a/>".to_vec()));
    assert!(diagnostics.records().is_empty());
    let read = package.read_xml_part(1, Limit::default_of(3), &mut diagnostics);
    assert_eq!(read.unwrap(), None);
    let read = package.read_xml_part(2, Limit::default_of(4), &mut diagnostics);
    assert_eq!(read.unwrap(), None);
    let limit = Limit::default_of(3);
    let too_large = Diagnostic::XmlTooLarge { size: 4, limit };
    let seen: Vec<_> = (diagnostics.finish().into_iter())
        .map(|record| {
            (
                record.diagnostic,
                record.location.part.map(|p| p.as_str().to_owned()),
            )
        })
        .collect();
    assert_eq!(seen, [(too_large, Some(String::from("/xl/workbook.xml")))]);
}

#[test]
fn a_duplicate_is_not_reported_again_as_derived() {
    let seen = found(with(vec![
        part("xl/workbook.xml/a"),
        part("XL/workbook.xml/A"),
    ]));
    let duplicate = duplicate("xl/workbook.xml/a");
    assert_eq!(seen, vec![at(duplicate, "XL/workbook.xml/A")]);
}
