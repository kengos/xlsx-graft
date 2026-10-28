//! What `Ignorable` takes in, refuses, and drops, and how often it
//! resolves a prefix.

use super::{Ignorable, MARKUP_COMPATIBILITY, Refused};
use std::cell::Cell;

/// The namespaces a reader of these tests understands.
const UNDERSTOOD: [&str; 1] = ["urn:s"];

/// Resolves `v`, `w` and `x` (to `v`'s namespace), `mc`, `s`, `xml` and
/// `xmlns`.
fn resolve(prefix: &str) -> Option<&'static str> {
    match prefix {
        "v" | "x" => Some("urn:v"),
        "w" => Some("urn:w"),
        "mc" => Some(MARKUP_COMPATIBILITY),
        "s" => Some("urn:s"),
        "xml" => Some("http://www.w3.org/XML/1998/namespace"),
        "xmlns" => Some("http://www.w3.org/2000/xmlns/"),
        _ => None,
    }
}

#[test]
fn the_namespaces_named_are_held_once_until_their_element_is_left() {
    let mut ignorable = Ignorable::new(&UNDERSTOOD);
    ignorable.enter();
    ignorable.name("v w v x", resolve).unwrap();
    assert!(ignorable.contains("urn:v") && ignorable.contains("urn:w"));
    ignorable.enter();
    // Named again inside: not held twice, and still there once left.
    ignorable.name(" w\tv\n", resolve).unwrap();
    assert_eq!(ignorable.namespaces.len(), 2);
    ignorable.leave();
    assert!(ignorable.contains("urn:w"));
    ignorable.leave();
    assert!(!ignorable.contains("urn:v") && !ignorable.contains("urn:w"));
    // Leaving more than was entered changes nothing.
    ignorable.leave();
    assert!(ignorable.namespaces.is_empty());
}

#[test]
fn leaving_an_inner_element_drops_what_it_named_and_no_more() {
    let mut ignorable = Ignorable::new(&UNDERSTOOD);
    ignorable.enter();
    ignorable.name("v", resolve).unwrap();
    ignorable.enter();
    ignorable.enter();
    ignorable.name("w", resolve).unwrap();
    ignorable.leave();
    assert!(ignorable.contains("urn:v") && !ignorable.contains("urn:w"));
    ignorable.leave();
    assert!(ignorable.contains("urn:v"));
    ignorable.leave();
    assert!(!ignorable.contains("urn:v"));
}

#[test]
fn a_namespace_understood_is_taken_and_still_read() {
    let mut ignorable = Ignorable::new(&UNDERSTOOD);
    ignorable.enter();
    ignorable.name("s xml xmlns v", resolve).unwrap();
    let named: Vec<&String> = ignorable.namespaces.iter().collect();
    assert_eq!(named, ["urn:v"]);
    for understood in ["urn:s", "http://www.w3.org/XML/1998/namespace"] {
        assert!(!ignorable.contains(understood));
    }
}

/// Names `value` once, counting how often a prefix is resolved.
fn resolutions(value: &str) -> (Result<(), Refused>, u32) {
    let calls = Cell::new(0);
    let counted = |prefix: &str| {
        calls.set(calls.get() + 1);
        resolve(prefix)
    };
    let mut ignorable = Ignorable::new(&UNDERSTOOD);
    ignorable.enter();
    let named = ignorable.name(value, counted);
    (named, calls.get())
}

#[test]
fn a_prefix_named_again_is_resolved_once_and_the_first_unresolved_stops() {
    assert_eq!(resolutions(&"v w ".repeat(10_000)), (Ok(()), 2));
    let undeclared: String = (0..10_000).map(|n| format!("u{n} ")).collect();
    assert_eq!(resolutions(&format!("v {undeclared}")), (Err(Refused), 2));
}

#[test]
fn a_prefix_undeclared_not_a_prefix_or_markup_compatibility_is_refused() {
    for value in ["u", "v u", "v:junk", "mc", "s mc"] {
        assert_eq!(resolutions(value).0, Err(Refused), "{value}");
    }
    assert_eq!(resolutions("").0, Ok(()));
}

#[test]
fn only_mc_ignorable_is_the_attribute() {
    assert!(Ignorable::is_attribute(
        Some(MARKUP_COMPATIBILITY),
        "Ignorable"
    ));
    assert!(!Ignorable::is_attribute(
        Some(MARKUP_COMPATIBILITY),
        "ProcessContent"
    ));
    assert!(!Ignorable::is_attribute(Some("urn:v"), "Ignorable"));
    assert!(!Ignorable::is_attribute(None, "Ignorable"));
}
