//! Tests for `around`, which `XmlRewriter::write` uses to find the `encoding`
//! value in a declaration's text. Its error, for a value that is not part of
//! the text, cannot be returned through the public API: quick-xml always
//! returns the value as a slice of the declaration's text.

use super::around;
use std::io;

#[test]
fn the_text_before_and_after_a_slice_is_found_by_its_address() {
    let outer = "xml encoding='UTF-16' x";
    let (before, after) = around(outer, &outer[14..20]).unwrap();
    assert_eq!((before, after), ("xml encoding='", "' x"));
    // The whole text, and an empty slice at its end.
    assert_eq!(around(outer, outer).unwrap(), ("", ""));
    assert_eq!(around(outer, &outer[outer.len()..]).unwrap(), (outer, ""));
}

#[test]
fn text_that_is_not_a_slice_of_the_other_is_refused() {
    let outer = "xml encoding='UTF-16'";
    let elsewhere = String::from("UTF-16");
    let error = around(outer, &elsewhere).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    // A slice of the same string that starts before `outer` is not a slice of
    // `outer` either.
    let longer = format!("{outer}{outer}");
    let error = around(&longer[outer.len()..], &longer[..outer.len()]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    // A slice that starts inside `outer` and ends past its end.
    let error = around(&outer[..5], &outer[3..10]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}
