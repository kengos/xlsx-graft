//! Unit tests for `Location`: which keys it serializes, and that each value
//! is echoed safely.

use super::Location;

fn json(location: &Location) -> String {
    serde_json::to_string(location).unwrap()
}

#[test]
fn an_empty_location_serializes_as_an_empty_object() {
    assert_eq!(json(&Location::default()), "{}");
}

#[test]
fn every_key_serializes_in_camel_case() {
    let location = Location::default()
        .with_entry("xl/_rels/workbook.xml.rels")
        .with_part("/xl/workbook.xml")
        .with_relationship_id("rId1")
        .with_rels_part("/xl/_rels/workbook.xml.rels");
    assert_eq!(
        json(&location),
        r#"{"entry":"xl/_rels/workbook.xml.rels","part":"/xl/workbook.xml","relationshipId":"rId1","relsPart":"/xl/_rels/workbook.xml.rels"}"#
    );
}

#[test]
fn only_the_keys_set_are_serialized() {
    assert_eq!(
        json(&Location::default().with_part("/xl/workbook.xml")),
        r#"{"part":"/xl/workbook.xml"}"#
    );
}

#[test]
fn values_are_echoed() {
    let location = Location::default().with_entry("a\nb");
    assert_eq!(location.entry.unwrap().as_str(), "ab");
}

#[test]
fn an_entry_name_in_bytes_is_echoed_lossily() {
    let location = Location::default().with_entry_bytes(b"xl/\xff\nsheet.xml");
    assert_eq!(location.entry.unwrap().as_str(), "xl/\u{fffd}sheet.xml");
}
