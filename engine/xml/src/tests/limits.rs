//! The limits, each at its largest admitted value and one past it: nesting
//! depth, attributes on one element, namespace declarations in scope, and
//! bytes read into memory. Where the streaming reader differs (the depth it
//! reports, and no size of its own), `streaming` covers it.

use super::{read, read_memory_with, read_with};
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
