//! The relationships graph: sources and targets resolved, what is refused
//! on either end, the main document, parts nothing targets, and the
//! incoming relationships of each part.

use super::opc::{
    OFFICE_DOCUMENT, RELS_NS, at, clean, found, minimal, package_rels, part, rel, rels, with,
};
use crate::{Incoming, PartName, Presence, RelationshipSource, Target};
use xlsx_graft_diagnostics::{Diagnostic, Echo, PartNameReason, SchemaReason};

/// The workbook's relationships, `body`.
fn workbook_rels(body: &str) -> super::builder::Item {
    rels("xl/_rels/workbook.xml.rels", body)
}

/// Where a diagnostic about the relationship `id` of the workbook points.
fn in_workbook(id: &str) -> String {
    format!("/xl/_rels/workbook.xml.rels#{id}")
}

#[test]
fn targets_resolve_against_the_source_and_link_both_ways() {
    let body = format!(
        "{}{}{}",
        rel("rId1", "worksheets/sheet1.xml"),
        rel("rId2", "/xl/styles.xml"),
        rel("rId3", "../xl/./worksheets/sheet1.xml#A1"),
    );
    let package = clean(with(vec![
        workbook_rels(&body),
        part("xl/worksheets/sheet1.xml"),
        part("xl/styles.xml"),
    ]));
    let workbook = 1;
    let names: Vec<&str> = package.parts().iter().map(|p| p.name().as_str()).collect();
    assert_eq!(names[workbook], "/xl/workbook.xml");
    let targets: Vec<&str> = (package.parts()[workbook].relationships().iter())
        .map(|r| match r.target() {
            Target::Internal(index) => package.parts()[*index].name().as_str(),
            Target::External(target) => target,
        })
        .collect();
    assert_eq!(
        targets,
        [
            "/xl/worksheets/sheet1.xml",
            "/xl/styles.xml",
            "/xl/worksheets/sheet1.xml"
        ]
    );
    let from_workbook = |index| Incoming {
        source: RelationshipSource::Part(workbook),
        index,
    };
    let sheet = package.part("/xl/worksheets/sheet1.xml").unwrap();
    assert_eq!(sheet.incoming(), [from_workbook(0), from_workbook(2)]);
}

#[test]
fn a_target_that_is_not_a_part_name_or_names_no_part_is_reported() {
    let body = format!(
        "{}{}{}{}",
        rel("rId1", "http://example.com/a.xml"),
        rel("rId2", "missing.xml"),
        rel("rId3", "../../../styles.xml"),
        rel("rId4", "%5C.xml"),
    );
    let (package, seen) = super::opc::read(with(vec![workbook_rels(&body), part("styles.xml")]));
    assert_eq!(
        seen,
        vec![
            at(
                Diagnostic::RelationshipTargetInvalid {
                    target: Echo::new("http://example.com/a.xml"),
                    reason: PartNameReason::HasScheme,
                },
                &in_workbook("rId1")
            ),
            at(
                Diagnostic::RelationshipTargetMissing {
                    part_name: Echo::new("/xl/missing.xml"),
                },
                &in_workbook("rId2")
            ),
            at(
                Diagnostic::RelationshipTargetAboveRoot {
                    target: Echo::new("../../../styles.xml"),
                },
                &in_workbook("rId3")
            ),
            at(
                Diagnostic::RelationshipTargetInvalid {
                    target: Echo::new("%5C.xml"),
                    reason: PartNameReason::EncodedSlash,
                },
                &in_workbook("rId4")
            ),
        ]
    );
    // Only the relationship whose target is a part is kept.
    let package = package.unwrap();
    let workbook = package.part("/xl/workbook.xml").unwrap();
    let ids: Vec<&str> = workbook.relationships().iter().map(|r| r.id()).collect();
    assert_eq!(ids, ["rId3"]);
}

#[test]
fn relationships_from_or_to_a_relationships_part_are_refused() {
    let to_rels = rel("rId2", "_rels/.rels");
    let from_rels = rels("_rels/_rels/.rels.rels", &rel("rId9", "../xl/workbook.xml"));
    let (package, seen) = super::opc::read(vec![
        minimal().remove(0),
        package_rels(&to_rels),
        part("xl/workbook.xml"),
        from_rels,
    ]);
    let package = package.unwrap();
    let ids: Vec<&str> = package.relationships().iter().map(|r| r.id()).collect();
    assert_eq!(ids, ["rId1"]);
    let rels = package.part("/_rels/.rels").unwrap();
    assert!(rels.relationships().is_empty() && rels.incoming().is_empty());
    // Both lists left a relationship out.
    assert!(!package.relationships_known() && !rels.relationships_known());
    assert_eq!(
        seen,
        vec![
            at(Diagnostic::RelationshipRelsPart {}, "/_rels/.rels#rId2"),
            at(
                Diagnostic::RelationshipRelsPart {},
                "/_rels/_rels/.rels.rels#"
            ),
        ]
    );
}

#[test]
fn a_relationships_part_without_its_source_is_not_read() {
    let seen = found(with(vec![
        rels("xl/_rels/gone.xml.rels", &rel("rId1", "workbook.xml")),
        rels("a/_rels/.rels", ""),
        part("a.xml"),
    ]));
    let missing = Diagnostic::RelationshipsSourceMissing {};
    // Not every Relationships part was read, so `a.xml` is not reported.
    assert_eq!(
        seen,
        vec![
            at(missing.clone(), "/xl/_rels/gone.xml.rels"),
            at(missing, "/a/_rels/.rels"),
        ]
    );
}

#[test]
fn a_refused_source_or_target_is_not_reported_again() {
    let body = rel("rId1", "Dup.xml");
    let seen = found(with(vec![
        part("xl/dup.xml"),
        part("xl/DUP.xml"),
        rels("xl/_rels/dup.xml.rels", ""),
        workbook_rels(&body),
    ]));
    let duplicate = Diagnostic::PartNameDuplicate {
        other: Echo::new("xl/dup.xml"),
    };
    assert_eq!(seen, vec![at(duplicate, "xl/DUP.xml")]);
}

#[test]
fn the_package_relationships_must_name_a_main_document() {
    let missing = Diagnostic::MainDocumentMissing {};
    let unreferenced = || at(Diagnostic::PartUnreferenced {}, "/xl/workbook.xml");
    let mut items = minimal();
    items[1] = rels("_rels/.rels", "");
    assert_eq!(found(items), vec![at(missing.clone(), ""), unreferenced()]);
    let mut items = minimal();
    items.remove(1);
    assert_eq!(found(items), vec![at(missing.clone(), ""), unreferenced()]);
    let external = format!(
        r#"<Relationship Id="rId1" Type="{OFFICE_DOCUMENT}" Target="http://a/" TargetMode="External"/>"#
    );
    let mut items = minimal();
    items[1] = rels("_rels/.rels", &external);
    assert_eq!(found(items), vec![at(missing, ""), unreferenced()]);
    let strict = r#"<Relationship Id="rId1" Type="http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument" Target="xl/workbook.xml"/>"#;
    let mut items = minimal();
    items[1] = rels("_rels/.rels", strict);
    assert_eq!(found(items), vec![]);
}

#[test]
fn package_relationships_that_cannot_be_read_leave_the_main_document_unreported() {
    let mut items = minimal();
    items[1] = items[1].clone().with_method(12);
    items.push(part("a.xml"));
    let unsupported = Diagnostic::ZipUnsupportedCompression { method: 12 };
    assert_eq!(found(items), vec![at(unsupported, "_rels/.rels")]);
    let mut items = minimal();
    items.push(part("_RELS/.rels"));
    let duplicate = Diagnostic::PartNameDuplicate {
        other: Echo::new("_rels/.rels"),
    };
    assert_eq!(found(items), vec![at(duplicate, "_RELS/.rels")]);
}

#[test]
fn a_part_no_relationship_targets_is_a_notice() {
    let seen = found(with(vec![part("a.xml"), rels("_rels/a.xml.rels", "")]));
    assert_eq!(seen, vec![at(Diagnostic::PartUnreferenced {}, "/a.xml")]);
}

#[test]
fn an_entry_named_like_relationships_but_not_a_part_hides_what_it_would_reach() {
    let windows = rels("xl\\_rels\\workbook.xml.rels", &rel("rId1", "styles.xml"));
    let seen = found(with(vec![windows, part("xl/styles.xml")]));
    let invalid = Diagnostic::EntryNameInvalid {
        reason: PartNameReason::InvalidCharacter,
    };
    assert_eq!(seen, vec![at(invalid, "xl\\_rels\\workbook.xml.rels")]);
    let mut items = minimal();
    items[1] = rels("_rels\\.rels", "");
    let invalid = Diagnostic::EntryNameInvalid {
        reason: PartNameReason::InvalidCharacter,
    };
    assert_eq!(found(items), vec![at(invalid, "_rels\\.rels")]);
}

#[test]
fn a_refused_name_that_cannot_be_the_package_relationships_leaves_its_absence_reported() {
    let mut items = minimal();
    items.remove(1);
    items.push(rels("xl/_rels/a b.xml.rels", ""));
    items.push(rels("xl/_rels/x.xml.rels", ""));
    items.push(rels("XL/_rels/x.xml.rels", ""));
    let invalid = Diagnostic::EntryNameInvalid {
        reason: PartNameReason::InvalidCharacter,
    };
    let duplicate = Diagnostic::PartNameDuplicate {
        other: Echo::new("xl/_rels/x.xml.rels"),
    };
    assert_eq!(
        found(items),
        vec![
            at(invalid, "xl/_rels/a b.xml.rels"),
            at(duplicate, "XL/_rels/x.xml.rels"),
            at(Diagnostic::MainDocumentMissing {}, ""),
        ]
    );
}

#[test]
fn what_is_wrong_on_the_root_alone_leaves_no_relationship_out() {
    let xml = format!(
        r#"<Relationships xmlns="{RELS_NS}" a="1">text<Relationship Id="rId1" Type="{OFFICE_DOCUMENT}" Target="xl/workbook.xml"/></Relationships>"#
    );
    let mut items = minimal();
    items[1] = super::builder::Item::stored("_rels/.rels", xml.as_bytes());
    items.push(part("a.xml"));
    let seen = found(items);
    let kinds: Vec<_> = (seen.iter().take(2))
        .map(|(diagnostic, place)| match diagnostic {
            Diagnostic::XmlSchemaInvalid { reason, name, .. } => {
                (*reason, name.as_str(), place.as_str())
            }
            other => panic!("expected xml_schema_invalid, got {other:?}"),
        })
        .collect();
    let rels = "/_rels/.rels";
    assert_eq!(
        kinds,
        [
            (SchemaReason::UnexpectedAttribute, "a", rels),
            (SchemaReason::UnexpectedText, "Relationships", rels),
        ]
    );
    assert_eq!(seen[2..], [at(Diagnostic::PartUnreferenced {}, "/a.xml")]);
}

#[test]
fn the_package_is_held_to_the_total_relationship_limit() {
    let items = || {
        with(vec![
            workbook_rels(&format!("{}{}", rel("r1", "a.xml"), rel("r2", "a.xml"))),
            part("xl/a.xml"),
            rels("xl/_rels/a.xml.rels", &rel("r1", "workbook.xml")),
        ])
    };
    let limits = |total| crate::Limits {
        max_total_relationships: super::limit(total),
        ..crate::Limits::default()
    };
    let (_, seen) = super::opc::read_with(items(), &limits(4));
    assert_eq!(seen, vec![]);
    let (package, seen) = super::opc::read_with(items(), &limits(2));
    let too_many = Diagnostic::RelationshipsTotalTooMany {
        count: 3,
        limit: super::limit(2),
    };
    assert_eq!(
        seen,
        vec![at(too_many.clone(), "/xl/_rels/workbook.xml.rels")]
    );
    let package = package.unwrap();
    assert!(
        package
            .part("/xl/workbook.xml")
            .unwrap()
            .relationships()
            .is_empty()
    );
    assert!(
        package
            .part("/xl/a.xml")
            .unwrap()
            .relationships()
            .is_empty()
    );
    // A part after the limit is not read: its duplicate `Id` is not seen.
    let mut later = items();
    later[5] = rels(
        "xl/_rels/a.xml.rels",
        &format!("{}{}", rel("r1", "w.xml"), rel("r1", "w.xml")),
    );
    let (_, seen) = super::opc::read_with(later, &limits(2));
    assert_eq!(seen, vec![at(too_many, "/xl/_rels/workbook.xml.rels")]);
}

#[test]
fn a_target_or_source_an_invalid_entry_name_may_have_been_is_not_missing() {
    let body = format!(
        "{}{}",
        rel("rId1", "worksheets/sheet1.xml"),
        rel("rId2", "a%20b.xml")
    );
    let seen = found(with(vec![
        workbook_rels(&body),
        part("xl\\worksheets\\sheet1.xml"),
        part("xl/a b.xml"),
        part("xl\\c.xml"),
        rels("xl/_rels/c.xml.rels", ""),
    ]));
    let invalid = Diagnostic::EntryNameInvalid {
        reason: PartNameReason::InvalidCharacter,
    };
    assert_eq!(
        seen,
        vec![
            at(invalid.clone(), "xl\\worksheets\\sheet1.xml"),
            at(invalid.clone(), "xl/a b.xml"),
            at(invalid, "xl\\c.xml"),
        ]
    );
}

#[test]
fn a_package_tells_a_part_that_may_be_there_from_one_that_is_not() {
    let mut items = with(vec![part("xl/a b.xml")]);
    items.push(part("xl/dup.xml"));
    items.push(part("xl/DUP.xml"));
    let (package, _) = super::opc::read(items);
    let package = package.unwrap();
    let find = |name: &str| package.find(&PartName::parse(name).unwrap());
    assert_eq!(find("/xl/workbook.xml"), Presence::Part(1));
    assert_eq!(find("/xl/a%20B.xml"), Presence::Unknown);
    assert_eq!(find("/xl/Dup.xml"), Presence::Unknown);
    assert_eq!(find("/xl/other.xml"), Presence::Absent);
}

#[test]
fn a_part_tells_whether_its_relationships_are_all_there() {
    let known = |items| {
        let (package, _) = super::opc::read(items);
        let package = package.unwrap();
        let workbook = package
            .part("/xl/workbook.xml")
            .unwrap()
            .relationships_known();
        (package.relationships_known(), workbook)
    };
    // No Relationships part: it has none, and that is known.
    assert_eq!(known(minimal()), (true, true));
    assert_eq!(
        known(with(vec![workbook_rels(&rel("rId1", "workbook.xml"))])),
        (true, true)
    );
    // A target that names no part leaves the list short.
    assert_eq!(
        known(with(vec![workbook_rels(&rel("rId1", "gone.xml"))])),
        (true, false)
    );
    // Not read, or may be there.
    let unreadable = workbook_rels("").with_method(12);
    assert_eq!(known(with(vec![unreadable])), (true, false));
    assert_eq!(
        known(with(vec![rels("xl\\_rels\\workbook.xml.rels", "")])),
        (true, false)
    );
    let mut items = minimal();
    items[1] = rels("_rels/.rels", "<Relationship/>");
    assert!(!known(items).0);
}

#[test]
fn shadow_keys_see_through_dot_segments_and_interleaved_pieces() {
    let seen = found(with(vec![
        workbook_rels(&rel("rId1", "styles.xml")),
        part("xl/./styles.xml"),
        part("[Content_Types].xml/[0].piece"),
    ]));
    let codes: Vec<&str> = seen.iter().map(|(d, _)| d.code().as_str()).collect();
    assert_eq!(codes, ["entry_name_invalid", "entry_name_invalid"]);
    let mut items = minimal();
    items[0].local.name = b"./[Content_Types].xml".to_vec();
    items[0].central.name = b"./[Content_Types].xml".to_vec();
    items[1].local.name = b"_rels/.rels/[0].Piece".to_vec();
    items[1].central.name = b"_rels/.rels/[0].Piece".to_vec();
    let codes: Vec<String> = found(items)
        .iter()
        .map(|(d, _)| d.code().to_string())
        .collect();
    // The package relationships may be there: nothing more is reported.
    assert_eq!(codes, ["entry_name_invalid", "entry_name_invalid"]);
    // An interleaved content types stream, with no stream of its own.
    let mut items = minimal();
    items[0].local.name = b"[Content_Types].xml/[0].piece".to_vec();
    items[0].central.name = b"[Content_Types].xml/[0].piece".to_vec();
    let seen = found(items);
    let codes: Vec<&str> = seen.iter().map(|(d, _)| d.code().as_str()).collect();
    assert_eq!(codes, ["entry_name_invalid"]);
}
