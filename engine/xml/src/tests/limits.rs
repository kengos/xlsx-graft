//! The limits, each at its largest admitted value and one past it: nesting
//! depth, attributes on one element, namespace declarations in scope, the
//! length of one, bytes read into memory, and bytes in one event. Where the
//! streaming reader differs (the depth it reports, and no `max_bytes`),
//! `streaming` covers it, and how little it holds of an event too large.

use super::{read, read_memory_with, read_with, utf16};
use crate::XmlLimits;
use xlsx_graft_diagnostics::{Diagnostic, Limit};

/// `depth` nested `<a>` elements, the innermost holding `inner`.
pub(super) fn nested(depth: usize, inner: &str) -> Vec<u8> {
    format!("{}{inner}{}", "<a>".repeat(depth), "</a>".repeat(depth)).into_bytes()
}

/// One element with `count` distinct attributes.
fn attributes(count: usize) -> Vec<u8> {
    let attributes: String = (0..count).map(|i| format!(" a{i}=\"\"")).collect();
    format!("<e{attributes}/>").into_bytes()
}

/// [`read_memory_with`] under the default limits: the in-memory reader
/// alone, which reads on for the deepest nesting.
fn read_memory(bytes: &[u8]) -> Result<usize, Diagnostic> {
    read_memory_with(bytes, &XmlLimits::default())
}

/// `xml_too_deep` at byte 768, where the 257th level opens in every case.
pub(super) fn too_deep(depth: u64) -> Result<usize, Diagnostic> {
    Err(Diagnostic::XmlTooDeep {
        offset: 768,
        depth,
        limit: Limit::default_of(256),
    })
}

#[test]
fn nesting_at_the_depth_limit_reads() {
    assert_eq!(read(&nested(256, "")), Ok(512));
    assert_eq!(read(&nested(255, "<b/>")), Ok(511));
}

#[test]
fn nesting_one_past_the_depth_limit_is_refused() {
    assert_eq!(read(&nested(257, "")), too_deep(257));
    assert_eq!(read(&nested(256, "<b/>")), too_deep(257));
}

#[test]
fn the_depth_reported_is_the_deepest_in_the_part() {
    assert_eq!(read_memory(&nested(300, "text")), too_deep(300));
    assert_eq!(read_memory(&nested(257, "<b/>")), too_deep(258));
    let mut back_down = b"<r>".to_vec();
    back_down.extend(nested(257, ""));
    back_down.extend(b"<a><a></a></a></r>");
    assert_eq!(read_memory(&back_down), too_deep(258));
}

#[test]
fn a_malformed_rest_reports_the_deepest_before_it() {
    let mut bytes = "<a>".repeat(257).into_bytes();
    bytes.extend(b"<b></c>");
    assert_eq!(read_memory(&bytes), too_deep(258));
}

#[test]
fn attributes_at_the_limit_read() {
    assert_eq!(read(&attributes(1_024)), Ok(1));
}

#[test]
fn attributes_past_the_limit_are_refused_with_their_count() {
    let refused = |count| {
        Err(Diagnostic::XmlTooManyAttributes {
            offset: 0,
            count,
            limit: Limit::default_of(1_024),
        })
    };
    assert_eq!(read(&attributes(1_025)), refused(1_025));
    assert_eq!(read(&attributes(5_000)), refused(5_000));
}

#[test]
fn bytes_at_the_size_limit_read_and_one_more_are_refused() {
    let limits = XmlLimits {
        max_bytes: Limit::default_of(8),
        ..XmlLimits::default()
    };
    assert_eq!(read_with(b"<a></a> ", &limits), Ok(3));
    let refused = Err(Diagnostic::XmlTooLarge {
        size: 9,
        limit: Limit::default_of(8),
    });
    assert_eq!(read_memory_with(b"<a></a>  ", &limits), refused);
}

/// `count` namespace declarations of the prefixes `p{from}` onwards.
fn declarations(from: usize, count: usize) -> String {
    (from..from + count)
        .map(|i| format!(" xmlns:p{i}=\"urn:{i}\""))
        .collect()
}

fn too_many_namespaces(offset: u64, count: u64) -> Result<usize, Diagnostic> {
    Err(Diagnostic::XmlTooManyNamespaces {
        offset,
        count,
        limit: Limit::default_of(256),
    })
}

#[test]
fn a_namespace_as_long_as_the_limit_reads_and_one_byte_longer_is_refused() {
    let at = format!(
        r#"<r xmlns="{}"><p:a xmlns:p="{}"/></r>"#,
        "u".repeat(256),
        "&amp;".repeat(51)
    );
    assert_eq!(read(at.as_bytes()), Ok(3));
    let past = format!(
        r#"<r xmlns="urn:d"><p:a xmlns:p="{}"/></r>"#,
        "u".repeat(257)
    );
    assert_eq!(
        read(past.as_bytes()),
        Err(Diagnostic::XmlNamespaceTooLong {
            offset: 17,
            length: 257,
            limit: Limit::default_of(256),
        })
    );
}

#[test]
fn namespaces_at_the_limit_read_and_one_more_is_refused() {
    let at = format!(r#"<r xmlns="urn:d"{}/>"#, declarations(0, 255));
    assert_eq!(read(at.as_bytes()), Ok(1));
    let past = format!(r#"<r xmlns="urn:d"{}/>"#, declarations(0, 256));
    assert_eq!(read(past.as_bytes()), too_many_namespaces(0, 257));
}

#[test]
fn namespaces_count_while_in_scope_and_free_when_closed() {
    let child = |from| format!("<c{}/>", declarations(from, 56));
    let xml = format!(
        "<r{}>{}<s{}></s>{}</r>",
        declarations(0, 200),
        child(200),
        declarations(200, 56),
        child(200)
    );
    assert_eq!(read(xml.as_bytes()), Ok(6));
    let inner = format!(
        "<r{}><s{}><t xmlns:z=\"urn:z\"/></s></r>",
        declarations(0, 200),
        declarations(200, 56)
    );
    let offset = inner.find("<t").unwrap();
    assert_eq!(
        read(inner.as_bytes()),
        too_many_namespaces(u64::try_from(offset).unwrap(), 257)
    );
}

#[test]
fn depth_is_checked_before_the_attributes_of_the_same_element() {
    let mut bytes = "<a>".repeat(256).into_bytes();
    bytes.extend(attributes(1_025));
    assert_eq!(read(&bytes), too_deep(257));
}

#[test]
fn an_offset_after_a_byte_order_mark_counts_from_the_text() {
    let mut bytes = b"\xef\xbb\xbf<r>".to_vec();
    bytes.extend(attributes(1_025));
    bytes.extend(b"</r>");
    let refused = Err(Diagnostic::XmlTooManyAttributes {
        offset: 3,
        count: 1_025,
        limit: Limit::default_of(1_024),
    });
    assert_eq!(read(&bytes), refused);
}

/// `max_event_bytes` of 64 bytes, with every other limit at its default.
pub(super) fn event_limits() -> XmlLimits {
    XmlLimits {
        max_event_bytes: Limit::default_of(64),
        ..XmlLimits::default()
    }
}

/// `xml_event_too_large` for the event at `offset`, read to one byte past
/// the limit of [`event_limits`].
pub(super) fn event_too_large(offset: usize) -> Result<usize, Diagnostic> {
    Err(Diagnostic::XmlEventTooLarge {
        offset: u64::try_from(offset).unwrap(),
        size: 65,
        limit: Limit::default_of(64),
    })
}

/// One event of each kind, as what comes before it, how it opens, how it
/// closes, what comes after it, the text that pads it to a size, and how
/// many events the part holds.
const EVENTS: [(&str, &str, &str, &str, &str, usize); 11] = [
    ("", r#"<p:r xmlns:p="urn:p" a=""#, r#"">"#, "</p:r>", "x", 2),
    ("", r#"<r a=""#, r#""/>"#, "", "x", 1),
    ("<r>", "</r", ">", "", " ", 2),
    ("<r>", "", "", "</r>", "t", 3),
    ("<r>", "", "", "&amp;</r>", "t", 4),
    ("<r/>", "", "", "", " ", 2),
    ("<r>", "<![CDATA[", "]]>", "</r>", "x", 3),
    ("<r>", "<!--", "-->", "</r>", "x", 3),
    ("<r>", "<?p ", "?>", "</r>", "x", 3),
    ("", r#"<?xml version="1.0""#, "?>", "<r/>", " ", 2),
    ("<r>", "&#x", "41;", "</r>", "0", 3),
];

/// The part of `event`, one of [`EVENTS`], with the event `size` bytes long.
fn part_of(event: (&str, &str, &str, &str, &str, usize), size: usize) -> Vec<u8> {
    let (before, open, close, after, pad, _) = event;
    let padding = pad.repeat(size - open.len() - close.len());
    format!("{before}{open}{padding}{close}{after}").into_bytes()
}

#[test]
fn an_event_of_every_kind_at_the_event_limit_reads() {
    for event in EVENTS {
        assert_eq!(read_with(&part_of(event, 64), &event_limits()), Ok(event.5));
    }
}

#[test]
fn an_event_of_every_kind_one_byte_past_the_event_limit_is_refused() {
    for event in EVENTS {
        let refused = event_too_large(event.0.len());
        assert_eq!(read_with(&part_of(event, 65), &event_limits()), refused);
        assert_eq!(read_with(&part_of(event, 5_000), &event_limits()), refused);
    }
}

#[test]
fn an_event_counts_its_bytes_as_utf8_whatever_the_encoding() {
    let at = format!("<r>t{}</r>", "あ".repeat(21));
    assert_eq!(read_with(at.as_bytes(), &event_limits()), Ok(3));
    assert_eq!(read_with(&utf16(&at, false), &event_limits()), Ok(3));
    // The limit falls inside a character: the size is the bytes read.
    let past = format!("<r>t{}</r>", "あ".repeat(22));
    assert_eq!(
        read_with(past.as_bytes(), &event_limits()),
        event_too_large(3)
    );
    assert_eq!(
        read_with(&utf16(&past, true), &event_limits()),
        event_too_large(3)
    );
}

#[test]
fn the_event_size_is_checked_before_the_guard() {
    let limits = XmlLimits {
        max_attributes: Limit::default_of(2),
        ..event_limits()
    };
    assert_eq!(
        read_with(&attributes(3), &limits),
        Err(Diagnostic::XmlTooManyAttributes {
            offset: 0,
            count: 3,
            limit: Limit::default_of(2),
        })
    );
    assert_eq!(read_with(&attributes(20), &limits), event_too_large(0));
}

#[test]
fn a_tag_quick_xml_refuses_past_the_event_limit_is_refused_for_its_size() {
    // A mismatched end tag and a comment holding `--`, each ended by the
    // byte past the limit.
    let end = format!("<r></{}>", "r".repeat(62));
    let comment = format!("<r><!--{}--></r>", "-".repeat(58));
    for bytes in [end, comment] {
        assert_eq!(
            read_with(bytes.as_bytes(), &event_limits()),
            event_too_large(3)
        );
    }
}

#[test]
fn a_character_xml_does_not_allow_comes_first_only_within_the_event_limit() {
    // A control character as the event's 64th byte, its 65th, one past the
    // limit, which is never handed out, and its 66th.
    let at = |size: usize| {
        let mut bytes = format!("<r>{}", "t".repeat(size - 1)).into_bytes();
        bytes.push(0x01);
        bytes.extend("t".repeat(1 << 16).into_bytes());
        bytes
    };
    assert!(matches!(
        read_with(&at(64), &event_limits()),
        Err(Diagnostic::XmlMalformed { offset: 66, .. })
    ));
    assert!(matches!(
        read_with(&at(65), &event_limits()),
        Err(Diagnostic::XmlMalformed { offset: 67, .. })
    ));
    assert_eq!(read_with(&at(66), &event_limits()), event_too_large(3));
}

#[test]
fn a_doctype_past_the_event_limit_is_refused_for_its_size() {
    let doctype = |size: usize| format!("<!DOCTYPE r [{}]><r/>", " ".repeat(size - 15));
    let refused = Err(Diagnostic::XmlDoctype { offset: 0 });
    assert_eq!(read_with(doctype(64).as_bytes(), &event_limits()), refused);
    assert_eq!(
        read_with(doctype(65).as_bytes(), &event_limits()),
        event_too_large(0)
    );
}

#[test]
fn the_deepest_nesting_is_counted_by_each_event_and_up_to_one_too_large() {
    let limits = event_limits();
    assert_eq!(
        read_memory_with(&nested(300, "text"), &limits),
        too_deep(300)
    );
    let mut bytes = "<a>".repeat(257).into_bytes();
    bytes.extend(format!("<b>{}<c/></b>", "t".repeat(65)).into_bytes());
    bytes.extend("</a>".repeat(257).into_bytes());
    assert_eq!(read_memory_with(&bytes, &limits), too_deep(258));
    // 65 bytes, ended by the byte past the limit: read whole, then refused.
    let tag = format!(r#"<c a="{}"/>"#, "x".repeat(56));
    let mut bytes = "<a>".repeat(257).into_bytes();
    bytes.extend(format!("<b>{tag}<d><e/></d></b>").into_bytes());
    assert_eq!(read_memory_with(&bytes, &limits), too_deep(258));
}
