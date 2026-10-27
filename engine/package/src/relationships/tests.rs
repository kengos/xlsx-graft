//! Unit tests for resolving a relationship's target against its source
//! part's name (ECMA-376-2:2021 §6.4.3).

use super::resolve;
use xlsx_graft_diagnostics::PartNameReason;

/// The part name `target` resolves to from `base`, and whether a `..` was
/// dropped above the root.
fn resolved(base: &str, target: &str) -> (String, bool) {
    let resolved = resolve(base, target).unwrap();
    (resolved.name.as_str().to_owned(), resolved.above_root)
}

#[test]
fn targets_resolve_as_the_standard_examples_do() {
    let base = "/a/b/foo.xml";
    assert_eq!(resolved(base, "/b/bar.xml"), ("/b/bar.xml".into(), false));
    assert_eq!(resolved(base, "bar.xml"), ("/a/b/bar.xml".into(), false));
    assert_eq!(resolved(base, "./bar.xml"), ("/a/b/bar.xml".into(), false));
    assert_eq!(resolved(base, "../bar.xml"), ("/a/bar.xml".into(), false));
    assert_eq!(resolved("/", "bar.xml"), ("/bar.xml".into(), false));
    assert_eq!(resolved("/", "../bar.xml"), ("/bar.xml".into(), true));
    assert_eq!(
        resolved(base, "../../../../c/./d.xml"),
        ("/c/d.xml".into(), true)
    );
}

#[test]
fn a_fragment_and_a_query_are_dropped_and_an_empty_reference_names_the_base() {
    assert_eq!(
        resolved("/a/b.xml", "c.xml#x?y"),
        ("/a/c.xml".into(), false)
    );
    assert_eq!(
        resolved("/a/b.xml", "c.xml?y#x"),
        ("/a/c.xml".into(), false)
    );
    assert_eq!(resolved("/a/b.xml", ""), ("/a/b.xml".into(), false));
    assert_eq!(resolved("/a/b.xml", "#only"), ("/a/b.xml".into(), false));
    assert_eq!(resolved("/a/b.xml", "%41.xml"), ("/a/A.xml".into(), false));
    assert_eq!(
        resolved("/a/b.xml", "%2E%2e/c.xml"),
        ("/c.xml".into(), false)
    );
}

#[test]
fn targets_that_name_no_part_say_why() {
    let reason = |base: &str, target: &str| resolve(base, target).unwrap_err();
    assert_eq!(
        reason("/a.xml", "//host/b.xml"),
        PartNameReason::HasAuthority
    );
    assert_eq!(
        reason("/a.xml", "http://example.com/b.xml"),
        PartNameReason::HasScheme
    );
    assert_eq!(reason("/a.xml", "c:/b.xml"), PartNameReason::HasScheme);
    assert_eq!(reason("/", ""), PartNameReason::EmptySegment);
    assert_eq!(reason("/a/b.xml", ".."), PartNameReason::EmptySegment);
    assert_eq!(reason("/a/b.xml", "."), PartNameReason::EmptySegment);
    assert_eq!(
        reason("/a/b.xml", "c\\d.xml"),
        PartNameReason::InvalidCharacter
    );
    assert_eq!(reason("/a/b.xml", "c%2e"), PartNameReason::EndsWithDot);
}
