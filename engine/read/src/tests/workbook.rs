//! Finding the workbook and the parts it relates to by type.

use super::zip::Item;
use super::{
    OFFICE_DOCUMENT, SHARED_STRINGS, WORKBOOK_RELS, at, book, content_types, open, package_of, rel,
    rels, seen,
};
use crate::{SHARED_STRINGS as TYPES, Workbook};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo};
use xlsx_graft_package::Presence;

/// The Strict `sharedStrings` relationship type.
const STRICT_SHARED_STRINGS: &str =
    "http://purl.oclc.org/ooxml/officeDocument/relationships/sharedStrings";

/// The package relationship to the workbook.
fn main() -> String {
    rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml")
}

/// A part `name` that holds an empty element.
fn part(name: &str) -> Item {
    Item::stored(name, b"<a/>")
}

#[test]
fn the_workbook_is_the_target_of_officedocument() {
    let mut diagnostics = Diagnostics::new();
    let package = open(book("<si/>"), &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let name = package.parts()[workbook.part()].name().as_str();
    assert_eq!(name, "/xl/workbook.xml");
    let found = workbook.related(&package, &TYPES, &mut diagnostics);
    let Presence::Part(strings) = found else {
        panic!("{found:?}");
    };
    assert_eq!(
        package.parts()[strings].name().as_str(),
        "/xl/sharedStrings.xml"
    );
    assert_eq!(seen(diagnostics), vec![]);
}

#[test]
fn no_officedocument_is_no_workbook_and_nothing_more_is_reported() {
    let mut diagnostics = Diagnostics::new();
    let items = package_of(&rel("rId1", "urn:other", "xl/workbook.xml"), "", vec![]);
    let package = open(items, &mut diagnostics);
    let before = diagnostics.records().len();
    assert_eq!(Workbook::find(&package, &mut diagnostics), None);
    assert_eq!(diagnostics.records().len(), before);
}

#[test]
fn officedocument_to_two_parts_is_a_duplicate_and_no_workbook() {
    let mut diagnostics = Diagnostics::new();
    let main = format!("{}{}", main(), rel("rId2", OFFICE_DOCUMENT, "xl/other.xml"));
    let package = open(
        package_of(&main, "", vec![part("xl/other.xml")]),
        &mut diagnostics,
    );
    let mut diagnostics = Diagnostics::new();
    assert_eq!(Workbook::find(&package, &mut diagnostics), None);
    let rel_type = Echo::new(OFFICE_DOCUMENT);
    let diagnostic = Diagnostic::RelationshipTypeDuplicate { rel_type };
    assert_eq!(seen(diagnostics), vec![at(diagnostic, "/_rels/.rels#rId2")]);
}

#[test]
fn two_relationships_to_one_part_are_one() {
    let mut diagnostics = Diagnostics::new();
    let strict = "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument";
    let main = format!("{}{}", main(), rel("rId2", strict, "/xl/workbook.xml"));
    let related = format!(
        "{}{}{}",
        rel("rId1", SHARED_STRINGS, "sharedStrings.xml"),
        rel("rId2", "urn:other", "other.xml"),
        rel("rId3", SHARED_STRINGS, "./sharedStrings.xml"),
    );
    let more = vec![part("xl/sharedStrings.xml"), part("xl/other.xml")];
    let package = open(package_of(&main, &related, more), &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let found = workbook.related(&package, &TYPES, &mut diagnostics);
    assert!(matches!(found, Presence::Part(_)), "{found:?}");
    assert_eq!(seen(diagnostics), vec![]);
}

#[test]
fn a_related_type_to_two_parts_is_a_duplicate_and_unknown_across_strict_and_transitional() {
    let mut diagnostics = Diagnostics::new();
    let related = format!(
        "{}{}{}",
        rel("rId1", SHARED_STRINGS, "sharedStrings.xml"),
        rel("rId2", SHARED_STRINGS, "./sharedStrings.xml"),
        rel("rId3", STRICT_SHARED_STRINGS, "strings2.xml"),
    );
    let more = vec![part("xl/sharedStrings.xml"), part("xl/strings2.xml")];
    let package = open(package_of(&main(), &related, more), &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let found = workbook.related(&package, &TYPES, &mut diagnostics);
    assert_eq!(found, Presence::Unknown);
    let rel_type = Echo::new(STRICT_SHARED_STRINGS);
    let diagnostic = Diagnostic::RelationshipTypeDuplicate { rel_type };
    let place = format!("{WORKBOOK_RELS}#rId3");
    assert_eq!(seen(diagnostics), vec![at(diagnostic, &place)]);
}

#[test]
fn an_external_relationship_of_the_type_is_not_one() {
    let mut diagnostics = Diagnostics::new();
    let related = format!(
        r#"<Relationship Id="rId1" Type="{SHARED_STRINGS}" Target="http://example.com/s.xml" TargetMode="External"/>"#
    );
    let package = open(package_of(&main(), &related, vec![]), &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let found = workbook.related(&package, &TYPES, &mut diagnostics);
    assert_eq!(found, Presence::Absent);
    assert_eq!(seen(diagnostics), vec![]);
}

#[test]
fn none_found_where_one_was_left_out_is_unknown() {
    let mut diagnostics = Diagnostics::new();
    // Two relationships share an Id: neither is kept, so the workbook's
    // relationships are not all known.
    let related = format!(
        "{}{}",
        rel("rId1", SHARED_STRINGS, "sharedStrings.xml"),
        rel("rId1", "urn:other", "sharedStrings.xml"),
    );
    let more = vec![part("xl/sharedStrings.xml")];
    let package = open(package_of(&main(), &related, more), &mut diagnostics);
    let mut diagnostics = Diagnostics::new();
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let found = workbook.related(&package, &TYPES, &mut diagnostics);
    assert_eq!(found, Presence::Unknown);
    assert_eq!(seen(diagnostics), vec![]);
}

#[test]
fn the_workbook_of_another_package_relates_to_nothing_known() {
    let mut diagnostics = Diagnostics::new();
    // A package whose workbook is its fourth part.
    let big = vec![
        content_types(),
        part("a/1.xml"),
        part("a/2.xml"),
        rels("_rels/.rels", &main()),
        part("xl/workbook.xml"),
    ];
    let big = open(big, &mut diagnostics);
    let small = open(package_of(&main(), "", vec![]), &mut diagnostics);
    let far = Workbook::find(&big, &mut diagnostics).unwrap();
    assert_eq!(far.part(), 3);
    assert!(small.parts().len() <= 3);
    assert_eq!(
        far.related(&small, &TYPES, &mut diagnostics),
        Presence::Unknown
    );
}

#[test]
fn a_table_whose_relationship_is_a_duplicate_is_unknown() {
    let related = format!(
        "{}{}",
        rel("rId1", SHARED_STRINGS, "sharedStrings.xml"),
        rel("rId2", SHARED_STRINGS, "strings2.xml"),
    );
    let more = vec![part("xl/sharedStrings.xml"), part("xl/strings2.xml")];
    let (strings, seen) = super::read(package_of(&main(), &related, more));
    assert!(matches!(strings, crate::SharedStrings::Unknown));
    let rel_type = Echo::new(SHARED_STRINGS);
    let diagnostic = Diagnostic::RelationshipTypeDuplicate { rel_type };
    let place = format!("{WORKBOOK_RELS}#rId2");
    assert_eq!(seen, vec![at(diagnostic, &place)]);
}

#[test]
fn one_found_where_others_were_left_out_is_taken() {
    let mut diagnostics = Diagnostics::new();
    // A relationship to a part that is not there is left out.
    let related = format!(
        "{}{}",
        rel("rId1", SHARED_STRINGS, "sharedStrings.xml"),
        rel("rId2", "urn:other", "missing.xml"),
    );
    let more = vec![part("xl/sharedStrings.xml")];
    let package = open(package_of(&main(), &related, more), &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    assert!(!package.parts()[workbook.part()].relationships_known());
    let found = workbook.related(&package, &TYPES, &mut diagnostics);
    assert!(matches!(found, Presence::Part(_)), "{found:?}");
}
