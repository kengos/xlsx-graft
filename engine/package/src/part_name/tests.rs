//! Unit tests for `PartName`: the grammar, the IRI form, the key two
//! equivalent names share, and what a name says about Relationships parts.

use super::{PartName, Source, decode_unreserved, shadow_key};
use xlsx_graft_diagnostics::PartNameReason;

/// Why `name` is not a part name.
fn refused(name: &str) -> PartNameReason {
    PartName::parse(name).unwrap_err()
}

#[test]
fn names_that_follow_the_grammar_parse_as_written() {
    for name in [
        "/a",
        "/xl/workbook.xml",
        "/a/b-c_d~e.f/g!$&'()*+,;=:@",
        "/%20space",
        "/C:/drive.xml",
        "/%FF",
        "/é/\u{10000}",
    ] {
        assert_eq!(PartName::parse(name).unwrap().as_str(), name);
    }
}

#[test]
fn names_that_break_the_grammar_say_why() {
    assert_eq!(refused("a/b"), PartNameReason::MissingLeadingSlash);
    assert_eq!(refused(""), PartNameReason::MissingLeadingSlash);
    for name in ["/", "//a", "/a//b", "/a/"] {
        assert_eq!(refused(name), PartNameReason::EmptySegment, "{name}");
    }
    for name in [
        "/a\\b", "/a b", "/[a]", "/a#b", "/a?b", "/%", "/%4", "/%G1", "/%+1",
    ] {
        assert_eq!(refused(name), PartNameReason::InvalidCharacter, "{name}");
    }
    for name in [
        "/\u{80}",
        "/\u{e000}",
        "/\u{fdd0}",
        "/\u{fffe}",
        "/\u{1fffe}",
        "/\u{e0001}",
    ] {
        assert_eq!(refused(name), PartNameReason::InvalidCharacter, "{name}");
    }
    for name in ["/a%2Fb", "/a%2f", "/a%5C", "/a%5c"] {
        assert_eq!(refused(name), PartNameReason::EncodedSlash, "{name}");
    }
    for name in ["/%41", "/%7e", "/%2D", "/%2E"] {
        assert_eq!(refused(name), PartNameReason::EncodedUnreserved, "{name}");
    }
    for name in ["/a.", "/.", "/..", "/a/../b", "/a./b"] {
        assert_eq!(refused(name), PartNameReason::EndsWithDot, "{name}");
    }
}

#[test]
fn non_ascii_percent_encodings_decode_to_the_iri_form() {
    let name = PartName::parse("/a/%D1%86.xml").unwrap();
    assert_eq!(name.as_str(), "/a/ц.xml");
    assert_eq!(name.to_string(), "/a/ц.xml");
    // Lowercase hexadecimal digits decode the same way.
    assert_eq!(PartName::parse("/%c3%a9").unwrap().as_str(), "/é");
    // Bytes that are not UTF-8 stay encoded, in uppercase.
    assert_eq!(
        PartName::parse("/%C3%ff%41b").unwrap_err(),
        PartNameReason::EncodedUnreserved
    );
    assert_eq!(PartName::parse("/%C3%ffb").unwrap().as_str(), "/%C3%FFb");
    // A character an IRI may not hold unencoded stays encoded.
    assert_eq!(
        PartName::parse("/%ee%80%80%C2%80").unwrap().as_str(),
        "/%EE%80%80%C2%80"
    );
    assert_eq!(
        PartName::parse("/%C3%A9%EE%80%80").unwrap().as_str(),
        "/é%EE%80%80"
    );
}

#[test]
fn equivalent_names_share_a_key_that_folds_ascii_case_only() {
    let key = |name: &str| PartName::parse(name).unwrap().key().to_owned();
    assert_eq!(key("/XL/Workbook.XML"), key("/xl/workbook.xml"));
    assert_eq!(key("/%c3%a9"), key("/%C3%A9"));
    assert_eq!(key("/%20"), key("/%20"));
    assert_eq!(key("/a%2b"), key("/a%2B"));
    assert_ne!(key("/É"), key("/é"));
}

#[test]
fn an_entry_name_maps_after_a_slash_and_a_reference_decodes_unreserved_characters() {
    assert_eq!(
        PartName::from_entry("xl/a.xml").unwrap().as_str(),
        "/xl/a.xml"
    );
    assert_eq!(
        PartName::from_entry("/xl").unwrap_err(),
        PartNameReason::EmptySegment
    );
    let reference = PartName::parse(&decode_unreserved("/x%6C/%41%20b%2e%ZZ"));
    assert_eq!(reference.unwrap_err(), PartNameReason::InvalidCharacter);
    let reference = PartName::parse(&decode_unreserved("/x%6C/%41%20b%7E")).unwrap();
    assert_eq!(reference.as_str(), "/xl/A%20b~");
}

#[test]
fn the_extension_is_after_the_last_dot_of_the_last_segment_lowered() {
    let extension = |name: &str| {
        PartName::parse(name)
            .unwrap()
            .extension()
            .map(str::to_owned)
    };
    assert_eq!(extension("/a.b/c.D.XML").as_deref(), Some("xml"));
    assert_eq!(extension("/a.b/c"), None);
    assert_eq!(extension("/a/.rels").as_deref(), Some("rels"));
}

#[test]
fn a_relationships_part_name_tells_its_source() {
    let source = |name: &str| PartName::parse(name).unwrap().relationships_source();
    assert_eq!(source("/_rels/.rels"), Some(Source::Package));
    assert_eq!(source("/_RELS/.RELS"), Some(Source::Package));
    assert_eq!(
        source("/_rels/foo.rels"),
        Some(Source::Part("/foo".to_owned()))
    );
    assert_eq!(
        source("/xl/_Rels/Workbook.xml.Rels"),
        Some(Source::Part("/xl/Workbook.xml".to_owned()))
    );
    assert_eq!(
        source("/a/_rels/.rels"),
        Some(Source::Part("/a/".to_owned()))
    );
    for name in [
        "/rels",
        "/a.rels",
        "/x/a.rels",
        "/_rels/a.xml",
        "/_rels/ц",
        "/_rels/rels",
    ] {
        assert_eq!(source(name), None, "{name}");
        assert!(!PartName::parse(name).unwrap().is_relationships());
    }
    assert!(PartName::parse("/_rels/.rels").unwrap().is_relationships());
}

#[test]
fn an_entry_name_that_is_not_a_part_name_has_the_key_a_lenient_reader_would_give() {
    let key = |name: &[u8]| shadow_key(name);
    assert_eq!(
        key(b"xl\\Worksheets\\Sheet1.xml"),
        "/xl/worksheets/sheet1.xml"
    );
    assert_eq!(key(b"/xl//A b[1].xml/"), "/xl/a%20b%5b1%5d.xml");
    assert_eq!(key("%41%C3%A9中%".as_bytes()), "/a\u{e9}中%25");
    assert_eq!(key(b"\xff"), "/%ef%bf%bd");
    assert_eq!(key(b"./a/../b/./c.xml"), "/b/c.xml");
    assert_eq!(key(b"a.xml/[12].last.PIECE"), "/a.xml");
    for name in [
        &b"a/[x].piece"[..],
        b"a/[].piece",
        b"a/[1]piece",
        b"a/1].piece",
    ] {
        assert!(key(name).ends_with("piece"), "{name:?}");
    }
}
