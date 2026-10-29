//! What a sheet is reported for and still listed: a name or a part another
//! sheet has, and an `r:id` that leads to no sheet's part.

use super::super::{NS, WORKBOOK_RELS, at, rel};
use super::{Item, PART, R, WORKSHEET, book, book_with, read, sheets, worksheet};
use xlsx_graft_diagnostics::{Diagnostic, Echo};

/// A `workbook` of `sheets` (name, `r:id`) whose relationships are
/// `related`, with worksheets 1 to `count`: no more than are targeted, or
/// the package model reports the rest unreferenced.
fn linked(listed: &[(&str, &str)], related: &str, count: u32) -> Vec<Item> {
    let xml = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}">{}</workbook>"#,
        sheets(listed)
    );
    book_with(&xml, related, (1..=count).map(worksheet).collect())
}

/// Whether each sheet of what is read has a part, in order.
fn parted(part: Option<crate::WorkbookPart>) -> Vec<bool> {
    let part = part.unwrap();
    (part.sheets().iter())
        .map(|sheet| sheet.part().is_some())
        .collect()
}

#[test]
fn a_name_another_sheet_has_letter_case_aside_is_reported_at_each_later_one() {
    let listed = [
        ("Ärger", "rId1"),
        ("äRGER", "rId2"),
        ("Other", "rId3"),
        ("ärger", "rId4"),
    ];
    let related: String = (1..=4)
        .map(|n| {
            rel(
                &format!("rId{n}"),
                WORKSHEET,
                &format!("worksheets/sheet{n}.xml"),
            )
        })
        .collect();
    let (part, seen) = read(linked(&listed, &related, 4));
    let duplicate = |name: &str| Diagnostic::SheetNameDuplicate {
        name: Echo::new(name),
        other: Echo::new("Ärger"),
    };
    assert_eq!(
        seen,
        vec![at(duplicate("äRGER"), PART), at(duplicate("ärger"), PART)]
    );
    assert_eq!(parted(part), vec![true; 4]);
}

#[test]
fn a_part_another_sheet_is_is_reported_whether_by_one_id_or_two() {
    let related = [
        rel("rId1", WORKSHEET, "worksheets/sheet1.xml"),
        rel("rId2", WORKSHEET, "worksheets/sheet1.xml"),
    ]
    .concat();
    let (part, seen) = read(linked(
        &[("A", "rId1"), ("B", "rId1"), ("C", "rId2")],
        &related,
        1,
    ));
    let duplicate = |name: &str| Diagnostic::SheetPartDuplicate {
        name: Echo::new(name),
        other: Echo::new("A"),
    };
    assert_eq!(
        seen,
        vec![at(duplicate("B"), PART), at(duplicate("C"), PART)]
    );
    let part = part.unwrap();
    let indexes: Vec<_> = (part.sheets().iter())
        .map(|sheet| sheet.part().map(|part| part.index()))
        .collect();
    assert_eq!(indexes, vec![indexes[0]; 3]);
}

#[test]
fn an_id_that_names_no_relationship_to_a_part_leaves_the_sheet_without_one() {
    let related = [
        rel("rId1", WORKSHEET, "worksheets/sheet1.xml"),
        r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="https://example.com/a.xml" TargetMode="External"/>"#.to_owned(),
    ]
    .concat();
    let (part, seen) = read(linked(
        &[("A", "rId1"), ("B", "rId2"), ("C", "rId9")],
        &related,
        1,
    ));
    let missing = |id: &str| Diagnostic::SheetRelationshipMissing {
        relationship_id: Echo::new(id),
    };
    // The external one is reported at its relationship.
    let rels = format!("{WORKBOOK_RELS}#rId2");
    assert_eq!(
        seen,
        vec![at(missing("rId2"), &rels), at(missing("rId9"), PART)]
    );
    assert_eq!(parted(part), vec![true, false, false]);
}

#[test]
fn a_relationship_left_out_already_is_not_reported_again() {
    // The package model leaves out a relationship to no part, and reports
    // it: the workbook's relationships are then not all known.
    let related = rel("rId1", WORKSHEET, "worksheets/nowhere.xml");
    let (part, seen) = read(linked(&[("A", "rId1"), ("B", "rId7")], &related, 0));
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(
        matches!(seen[0].0, Diagnostic::RelationshipTargetMissing { .. }),
        "{seen:?}"
    );
    assert_eq!(parted(part), vec![false, false]);
}

#[test]
fn a_relationship_not_of_a_sheet_is_reported_at_the_relationship() {
    let styles = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
    let related = [
        rel("rId1", styles, "worksheets/sheet1.xml"),
        rel("rId2", WORKSHEET, "worksheets/sheet2.xml"),
    ]
    .concat();
    let (part, seen) = read(linked(&[("A", "rId1"), ("B", "rId2")], &related, 2));
    let invalid = Diagnostic::SheetRelationshipTypeInvalid {
        rel_type: Echo::new(styles),
    };
    assert_eq!(seen, vec![at(invalid, &format!("{WORKBOOK_RELS}#rId1"))]);
    assert_eq!(parted(part), vec![false, true]);
}

#[test]
fn sheets_of_the_book_helper_link_to_their_worksheets() {
    let (part, seen) = read(book(&sheets(&[("A", "rId1"), ("B", "rId2")])));
    assert_eq!(seen, vec![]);
    assert_eq!(parted(part), vec![true, true]);
}
