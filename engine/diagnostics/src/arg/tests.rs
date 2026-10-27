//! Unit tests for the argument wire keys.

use super::wire_key;

#[test]
fn a_one_word_field_keeps_its_name() {
    assert_eq!(wire_key("dropped"), "dropped");
}

#[test]
fn a_snake_case_field_becomes_camel_case() {
    assert_eq!(wire_key("relationship_id"), "relationshipId");
    assert_eq!(wire_key("max_entry_name_bytes"), "maxEntryNameBytes");
}
