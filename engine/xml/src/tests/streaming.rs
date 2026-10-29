//! What only the streaming reader does: it stops at the first level past
//! the depth limit, does not apply `max_bytes`, reads no further than it
//! must, holds no more of an event too large than one byte past the limit,
//! hands a host failure back apart from the diagnostics, and puts characters split
//! across reads back together. The inputs both readers share are in the
//! other files, run through both by `read_with`.

use super::limits::{event_limits, event_too_large, nested, too_deep};
use super::{Chunked, chunked, read_stream_with, utf16};
use crate::events::Event;
use crate::name::{Namespace, ResolveResult};
use crate::{XmlLimits, XmlStreamError, XmlStreamReader, XmlVersion};
use std::io;
use xlsx_graft_diagnostics::{Diagnostic, Limit};

/// Reads `bytes` a byte at a time to the end with the streaming reader.
fn stream(bytes: &[u8]) -> Result<usize, Diagnostic> {
    read_stream_with(chunked(bytes, 1), &XmlLimits::default())
}

/// A streaming reader over `bytes`, `step` bytes at a time.
fn reader(bytes: &[u8], step: usize) -> XmlStreamReader<Chunked> {
    XmlStreamReader::new(chunked(bytes, step), &XmlLimits::default())
}

#[test]
fn the_depth_reported_is_one_past_the_limit() {
    assert_eq!(stream(&nested(300, "text")), too_deep(257));
    assert_eq!(stream(&nested(257, "<b/>")), too_deep(257));
    let mut malformed_rest = "<a>".repeat(257).into_bytes();
    malformed_rest.extend(b"<b></c>");
    assert_eq!(stream(&malformed_rest), too_deep(257));
}

#[test]
fn reading_stops_at_the_element_past_the_depth_limit() {
    let mut bytes = "<a>".repeat(257).into_bytes();
    bytes.extend("<b/>".repeat(100_000).into_bytes());
    let mut reader = reader(&bytes, 64);
    let mut buf = Vec::new();
    let error = loop {
        match reader.read_event_into(&mut buf) {
            Ok(_) => {}
            Err(error) => break error,
        }
    };
    assert!(matches!(
        error,
        XmlStreamError::Invalid(Diagnostic::XmlTooDeep { .. })
    ));
    let read = reader.get_ref().get_ref().pos;
    assert!(read < 1_024, "read {read} of {} bytes", bytes.len());
}

#[test]
fn an_event_too_large_is_held_to_one_byte_past_the_limit() {
    // A megabyte of text in one event, under a limit of 64 bytes.
    let bytes = format!("<a>{}</a>", "t".repeat(1 << 20)).into_bytes();
    for step in [1, 4_096] {
        let mut reader = XmlStreamReader::new(chunked(&bytes, step), &event_limits());
        let mut buf = Vec::new();
        assert!(matches!(
            reader.read_event_into(&mut buf),
            Ok(Event::Start(_))
        ));
        let Err(XmlStreamError::Invalid(first)) = reader.read_event_into(&mut buf) else {
            panic!("expected xml_event_too_large");
        };
        assert_eq!(Err(first.clone()), event_too_large(3));
        assert_eq!(buf.len(), 65);
        assert!(buf.capacity() <= 130, "held {} bytes", buf.capacity());
        let read = reader.get_ref().get_ref().pos;
        assert!(read < 3 * 4_096, "read {read} of {} bytes", bytes.len());
        let Err(XmlStreamError::Invalid(again)) = reader.read_event_into(&mut buf) else {
            panic!("expected the diagnostic again");
        };
        assert_eq!(again, first);
    }
}

#[test]
fn a_host_failure_after_the_byte_past_the_event_limit_comes_after_the_limit() {
    // The text starts at byte 3, so its 65th byte, one past the limit, is 67.
    let bytes = format!("<a>{}</a>", "t".repeat(100)).into_bytes();
    let read_failing = |fail_at, interrupt_at| {
        let mut source = chunked(&bytes, 1);
        source.get_mut().fail_at = fail_at;
        source.get_mut().interrupt_at = interrupt_at;
        let mut reader = XmlStreamReader::new(source, &event_limits());
        let mut buf = Vec::new();
        assert!(matches!(
            reader.read_event_into(&mut buf),
            Ok(Event::Start(_))
        ));
        match reader.read_event_into(&mut buf) {
            Err(XmlStreamError::Invalid(diagnostic)) => Ok(diagnostic),
            Err(XmlStreamError::Io(error)) => Err(error.to_string()),
            Ok(event) => panic!("unexpected {event:?}"),
        }
    };
    let refused = event_too_large(3).unwrap_err();
    assert_eq!(read_failing(Some(68), None), Ok(refused.clone()));
    assert_eq!(
        read_failing(Some(67), None),
        Err("the host failed".to_owned())
    );
    assert_eq!(read_failing(None, Some(67)), Ok(refused.clone()));
    assert_eq!(read_failing(None, Some(66)), Ok(refused));
}

#[test]
fn max_bytes_does_not_apply() {
    let limits = XmlLimits {
        max_bytes: Limit::default_of(8),
        ..XmlLimits::default()
    };
    let bytes = b"<a></a>  ";
    for step in [1, bytes.len()] {
        assert_eq!(read_stream_with(chunked(bytes, step), &limits), Ok(3));
    }
}

#[test]
fn nothing_is_read_before_the_first_event() {
    let reader = reader(b"\0\0\0<", 1);
    assert_eq!(reader.get_ref().get_ref().reads, 0);
}

#[test]
fn text_split_across_reads_and_decode_chunks_arrives_whole() {
    // 100,000 characters of three bytes: over the 64 KiB decoded at once,
    // which does not end on a character.
    let text = "あ".repeat(100_000);
    let bytes = format!("<a>{text}</a>").into_bytes();
    for step in [bytes.len(), 7] {
        let mut reader = reader(&bytes, step);
        let mut buf = Vec::new();
        assert!(matches!(
            reader.read_event_into(&mut buf),
            Ok(Event::Start(_))
        ));
        let Ok(Event::Text(read)) = reader.read_event_into(&mut buf) else {
            panic!("expected the text");
        };
        assert_eq!(AsRef::<str>::as_ref(&read), text);
    }
}

#[test]
fn a_utf16_attribute_split_a_byte_at_a_time_arrives_as_utf8() {
    for big in [false, true] {
        let bytes = utf16(r#"<?xml version="1.0" encoding="UTF-16"?><a b="é𝄞"/>"#, big);
        let mut reader = reader(&bytes, 1);
        let mut buf = Vec::new();
        assert!(matches!(
            reader.read_event_into(&mut buf),
            Ok(Event::Decl(_))
        ));
        let Ok(Event::Empty(element)) = reader.read_event_into(&mut buf) else {
            panic!("expected the empty element");
        };
        let value = element
            .attributes()
            .next()
            .unwrap()
            .unwrap()
            .normalized_value(XmlVersion::Implicit1_0)
            .unwrap();
        assert_eq!(value, "é𝄞");
        assert_eq!(reader.offset(), 39);
    }
}

#[test]
fn names_resolve_and_offsets_count_as_in_memory() {
    let xml = br#"<p:a xmlns:p="urn:p"><b p:c="1"/>text<q:e/></p:a>"#;
    let mut reader = reader(xml, 1);
    let mut buf = Vec::new();
    assert_eq!(reader.offset(), 0);
    let Ok(Event::Start(a)) = reader.read_event_into(&mut buf) else {
        panic!("expected the root");
    };
    let (namespace, local) = reader.resolve_element(a.name());
    assert_eq!(
        (namespace, local.as_ref()),
        (ResolveResult::Bound(Namespace("urn:p")), "a")
    );
    let Ok(Event::Empty(b)) = reader.read_event_into(&mut buf) else {
        panic!("expected b");
    };
    assert_eq!(reader.offset(), 21);
    assert_eq!(reader.resolve_prefix("p"), Some("urn:p"));
    assert_eq!(reader.resolve_prefix("q"), None);
    assert_eq!(reader.resolve_prefix("p:c"), None);
    let key = b.attributes().next().unwrap().unwrap().key;
    let (namespace, local) = reader.resolve_attribute(key);
    assert_eq!(
        (namespace, local.as_ref()),
        (ResolveResult::Bound(Namespace("urn:p")), "c")
    );
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Ok(Event::Text(_))
    ));
    assert_eq!(reader.offset(), 33);
    let Ok(Event::Empty(e)) = reader.read_event_into(&mut buf) else {
        panic!("expected q:e");
    };
    let (namespace, _) = reader.resolve_element(e.name());
    assert!(matches!(namespace, ResolveResult::Unknown(_)));
}

#[test]
fn a_failed_reader_returns_its_diagnostic_again() {
    let mut reader = reader(b"<a>\0</a>", 1);
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Ok(Event::Start(_))
    ));
    let Err(XmlStreamError::Invalid(first)) = reader.read_event_into(&mut buf) else {
        panic!("expected a diagnostic");
    };
    assert!(matches!(first, Diagnostic::XmlMalformed { offset: 3, .. }));
    let Err(XmlStreamError::Invalid(again)) = reader.read_event_into(&mut buf) else {
        panic!("expected the diagnostic again");
    };
    assert_eq!(again, first);
}

#[test]
fn a_host_failure_comes_back_as_io_and_again() {
    let mut source = chunked(b"<a><b/></a>", 1);
    source.get_mut().fail_at = Some(5);
    let mut reader = XmlStreamReader::new(source, &XmlLimits::default());
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Ok(Event::Start(_))
    ));
    let Err(XmlStreamError::Io(first)) = reader.read_event_into(&mut buf) else {
        panic!("expected an I/O error");
    };
    assert_eq!(first.to_string(), "the host failed");
    let reads = reader.get_ref().get_ref().reads;
    let Err(XmlStreamError::Io(again)) = reader.read_event_into(&mut buf) else {
        panic!("expected the I/O error again");
    };
    assert_eq!(
        (again.kind(), again.to_string()),
        (first.kind(), first.to_string())
    );
    assert_eq!(
        reader.get_ref().get_ref().reads,
        reads,
        "a spent reader reads no more"
    );
}

#[test]
fn an_interrupted_read_is_retried() {
    let mut source = chunked(b"<a/>", 1);
    source.get_mut().interrupt_at = Some(0);
    let mut reader = XmlStreamReader::new(source, &XmlLimits::default());
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Ok(Event::Empty(_))
    ));
    assert!(matches!(reader.read_event_into(&mut buf), Ok(Event::Eof)));
}

#[test]
fn a_read_interrupted_inside_a_character_keeps_its_first_bytes() {
    let utf8 = "<a>あ</a>".as_bytes().to_vec();
    let utf16 = utf16("<a>𝄞</a>", false);
    // Inside あ, and between the two code units of 𝄞.
    for (bytes, at, text) in [(utf8, 4, "あ"), (utf16, 10, "𝄞")] {
        let mut source = chunked(&bytes, 1);
        source.get_mut().interrupt_at = Some(at);
        let mut reader = XmlStreamReader::new(source, &XmlLimits::default());
        let mut buf = Vec::new();
        assert!(matches!(
            reader.read_event_into(&mut buf),
            Ok(Event::Start(_))
        ));
        let Ok(Event::Text(read)) = reader.read_event_into(&mut buf) else {
            panic!("expected the text");
        };
        assert_eq!(AsRef::<str>::as_ref(&read), text);
        assert_eq!(reader.get_ref().get_ref().interrupt_at, None, "interrupted");
    }
}

#[test]
fn the_errors_say_what_stopped_the_reader() {
    let io = XmlStreamError::Io(io::Error::other("gone"));
    assert_eq!(io.to_string(), "reading the XML failed: gone");
    let invalid = XmlStreamError::from(Diagnostic::XmlDoctype { offset: 0 });
    assert_eq!(
        invalid.to_string(),
        "the XML is refused: XmlDoctype { offset: 0 }"
    );
}

#[test]
fn an_io_error_from_a_slice_would_be_malformed_where_it_stopped() {
    let io = XmlStreamError::Io(io::Error::other("gone"));
    let expected = Diagnostic::XmlMalformed {
        offset: 5,
        detail: xlsx_graft_diagnostics::Echo::new("gone"),
    };
    assert_eq!(io.into_diagnostic(5), expected);
    let doctype = Diagnostic::XmlDoctype { offset: 0 };
    assert_eq!(
        XmlStreamError::from(doctype.clone()).into_diagnostic(5),
        doctype
    );
}

#[test]
fn a_failure_after_the_last_byte_comes_before_eof() {
    // How a zip entry whose CRC does not match fails: on the read past its
    // end.
    let mut source = chunked(b"<a/>", 1);
    source.get_mut().fail_at = Some(4);
    let mut reader = XmlStreamReader::new(source, &XmlLimits::default());
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Ok(Event::Empty(_))
    ));
    assert!(matches!(
        reader.read_event_into(&mut buf),
        Err(XmlStreamError::Io(_))
    ));
}
