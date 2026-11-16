//! Unit tests for what `Extent` keeps of a `dimension`: the first
//! characters only, in a string whose capacity is no more than they need.

use super::{Extent, MAX_DIMENSION_CHARS};
use xlsx_graft_diagnostics::Limit;
use xlsx_graft_xml::events::BytesStart;

/// The `dimension` an extent keeps for the `ref` `text`.
fn kept(text: &str) -> Option<String> {
    let start = BytesStart::from_content(format!(r#"dimension ref="{text}""#), 9);
    let mut extent = Extent::new(Limit::default_of(1));
    extent.dimension(&start);
    extent.dimension
}

#[test]
fn a_long_ref_is_kept_as_its_first_characters_with_a_capacity_of_at_most_4_bytes_each() {
    let long = "あ".repeat(MAX_DIMENSION_CHARS * 50);
    let text = kept(&long).unwrap();
    assert_eq!(text.chars().count(), MAX_DIMENSION_CHARS);
    assert!(
        text.capacity() <= 4 * MAX_DIMENSION_CHARS,
        "{}",
        text.capacity()
    );
    assert_eq!(kept("A1:B2").as_deref(), Some("A1:B2"));
    assert_eq!(
        kept(&"A".repeat(MAX_DIMENSION_CHARS)).unwrap().len(),
        MAX_DIMENSION_CHARS
    );
}
