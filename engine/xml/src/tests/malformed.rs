//! Well-formedness: what quick-xml refuses and what the reader adds to it,
//! each as `xml_malformed` at the byte offset where reading stopped.

use super::{malformed_at, read, utf16};
use crate::XmlLimits;
use crate::XmlReader;
use xlsx_graft_diagnostics::Diagnostic;

fn detail(bytes: &[u8]) -> String {
    malformed_at(bytes).1
}

#[test]
fn a_nul_character_is_refused_at_its_offset() {
    let nul = "a character XML does not allow (U+0000)";
    assert_eq!(malformed_at(b"<a>\0</a>"), (3, nul.to_owned()));
    assert_eq!(malformed_at(b"\xef\xbb\xbf<a>\0</a>"), (3, nul.to_owned()));
    let expected = Err(Diagnostic::XmlMalformed {
        offset: 5,
        detail: xlsx_graft_diagnostics::Echo::new(nul),
    });
    assert_eq!(read(&utf16("<a>é\0</a>", false)), expected);
}

#[test]
fn opening_a_part_reads_none_of_it() {
    let mut reader = XmlReader::open(b"<a>\0</a>", &XmlLimits::default()).unwrap();
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Ok(crate::events::Event::Start(_))
    ));
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Err(Diagnostic::XmlMalformed { offset: 3, .. })
    ));
}

#[test]
fn a_problem_is_the_first_that_stops_reading() {
    let doctype_first = b"<!DOCTYPE a><a>\xff</a>";
    assert_eq!(
        read(doctype_first),
        Err(Diagnostic::XmlDoctype { offset: 0 })
    );
    let late_byte = b"<a></b>\xff";
    assert!(matches!(
        read(late_byte),
        Err(Diagnostic::XmlMalformed { .. })
    ));
}

#[test]
fn other_characters_xml_does_not_allow_are_refused() {
    assert_eq!(
        detail(b"<a>\x01</a>"),
        "a character XML does not allow (U+0001)"
    );
    assert_eq!(
        detail("<a>\u{fffe}</a>".as_bytes()),
        "a character XML does not allow (U+FFFE)"
    );
}

#[test]
fn a_mismatched_end_tag_is_refused_where_the_parser_stopped() {
    let (offset, detail) = malformed_at(b"<a></b>");
    assert!(offset >= 3, "{offset}");
    assert!(detail.contains('b'), "{detail}");
}

#[test]
fn an_unclosed_element_is_refused_at_the_end() {
    assert_eq!(
        malformed_at(b"<a><b></b>"),
        (10, "an element is not closed".to_owned())
    );
}

#[test]
fn a_part_with_no_root_element_is_refused() {
    assert_eq!(detail(b""), "the XML has no root element");
    assert_eq!(detail(b"<!-- only -->\n"), "the XML has no root element");
}

#[test]
fn a_second_root_element_is_refused() {
    assert_eq!(
        malformed_at(b"<a/><b/>"),
        (4, "the XML has a second root element".to_owned())
    );
}

#[test]
fn content_outside_the_root_element_is_refused() {
    let outside = "the XML has content outside the root element";
    assert_eq!(detail(b"x<a/>"), outside);
    assert_eq!(detail(b"<a/>x"), outside);
    assert_eq!(detail(b"<![CDATA[x]]><a/>"), outside);
    assert_eq!(detail(b"&amp;<a/>"), outside);
    assert_eq!(read(b" \t\r\n<a/>\n"), Ok(3));
}

#[test]
fn a_declaration_that_is_not_first_is_refused() {
    let late = "the XML declaration is not at the start";
    assert_eq!(detail(b" <?xml version=\"1.0\"?><a/>"), late);
    assert_eq!(detail(b"<a/><?xml version=\"1.0\"?>"), late);
    assert_eq!(detail(b"<a><?xml version=\"1.0\"?></a>"), late);
}

#[test]
fn a_duplicate_attribute_is_refused() {
    malformed_at(br#"<a b="1" b="2"/>"#);
}

#[test]
fn a_double_hyphen_in_a_comment_is_refused() {
    malformed_at(b"<a><!-- a -- b --></a>");
}

#[test]
fn a_character_xml_does_not_allow_in_the_declaration_is_found_before_it() {
    let refused = "a character XML does not allow (U+0001)";
    let in_value = b"<?xml version=\"1.0\" encoding=\"UTF-8\x01\"?><a/>";
    assert_eq!(malformed_at(in_value), (35, refused.to_owned()));
    let between = b"<?xml version=\"1.0\"\x01 encoding=\"UTF-8\"?><a/>";
    assert_eq!(malformed_at(between), (19, refused.to_owned()));
}
