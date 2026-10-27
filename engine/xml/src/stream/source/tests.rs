//! Tests for `Source` read as a plain `Read`, which quick-xml never does,
//! over both readers the crate builds it for: the decoded text comes out
//! whole, and a problem ends it as an error.

use super::Source;
use crate::tests::chunked;
use std::io::Read;

#[test]
fn decoded_text_reads_whole_through_read() {
    let mut bytes = vec![0xfe, 0xff];
    bytes.extend("<a>𝄞</a>".encode_utf16().flat_map(u16::to_be_bytes));
    let mut text = String::new();
    Source::new(chunked(&bytes, 1))
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "<a>𝄞</a>");
    let mut text = String::new();
    Source::new(&bytes[..]).read_to_string(&mut text).unwrap();
    assert_eq!(text, "<a>𝄞</a>");
}

#[test]
fn a_problem_ends_the_text_as_an_error() {
    let bytes = b"<a>\x01</a>";
    let mut source = Source::new(chunked(bytes, 2));
    let mut text = Vec::new();
    assert!(source.read_to_end(&mut text).is_err());
    assert_eq!(text, b"<a>");
    assert!(source.decoder().failed().is_some());
    let mut source = Source::new(&bytes[..]);
    let mut text = Vec::new();
    assert!(source.read_to_end(&mut text).is_err());
    assert_eq!(text, b"<a>");
    assert!(source.decoder().failed().is_some());
}
