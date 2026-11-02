//! Unit tests for what the part check works out from names: which media
//! types are XML, and the Relationships part of a part.

use super::{is_xml, rels_of};

#[test]
fn the_relationships_part_of_a_part_is_beside_it() {
    assert_eq!(rels_of("/xl/workbook.xml"), "/xl/_rels/workbook.xml.rels");
}

#[test]
fn a_media_type_is_xml_by_its_suffix_or_as_one_of_the_xml_types() {
    assert!(is_xml("application/vnd.ms-excel.macrosheet+xml"));
    assert!(is_xml("text/xml"));
    assert!(!is_xml("application/vnd.ms-office.activex"));
}
