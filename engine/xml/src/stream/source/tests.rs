//! Tests for `Source` read as a plain `Read`, which quick-xml never does,
//! over both readers the crate builds it for: the decoded text comes out
//! whole, and a problem or an event past the limit ends it as an error.

use super::Source;
use crate::XmlLimits;
use crate::tests::chunked;
use std::io::Read;
use xlsx_graft_diagnostics::{Diagnostic, Limit};

/// The default `max_event_bytes`.
fn limit() -> Limit {
    Limit::default_of(XmlLimits::DEFAULT_MAX_EVENT_BYTES)
}

#[test]
fn decoded_text_reads_whole_through_read() {
    let mut bytes = vec![0xfe, 0xff];
    bytes.extend("<a>𝄞</a>".encode_utf16().flat_map(u16::to_be_bytes));
    let mut text = String::new();
    Source::new(chunked(&bytes, 1), limit())
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "<a>𝄞</a>");
    let mut text = String::new();
    Source::new(&bytes[..], limit())
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "<a>𝄞</a>");
}

#[test]
fn a_problem_ends_the_text_as_an_error() {
    let bytes = b"<a>\x01</a>";
    let mut source = Source::new(chunked(bytes, 2), limit());
    let mut text = Vec::new();
    assert!(source.read_to_end(&mut text).is_err());
    assert_eq!(text, b"<a>");
    assert!(source.failed().is_some());
    let mut source = Source::new(&bytes[..], limit());
    let mut text = Vec::new();
    assert!(source.read_to_end(&mut text).is_err());
    assert_eq!(text, b"<a>");
    assert!(source.failed().is_some());
}

#[test]
fn the_text_stops_one_byte_past_the_event_limit() {
    let refused = Diagnostic::XmlEventTooLarge {
        offset: 0,
        size: 5,
        limit: Limit::default_of(4),
    };
    let mut source = Source::new(chunked(b"<a></a>", 3), Limit::default_of(4));
    let mut text = Vec::new();
    assert!(source.read_to_end(&mut text).is_err());
    assert_eq!(text, b"<a></");
    assert_eq!(source.failed(), Some(&refused));
    let mut source = Source::new(&b"<a></a>"[..], Limit::default_of(4));
    let mut text = Vec::new();
    assert!(source.read_to_end(&mut text).is_err());
    assert_eq!(source.failed(), Some(&refused));
}
