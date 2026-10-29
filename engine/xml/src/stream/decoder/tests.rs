//! Tests for `Decoder` driven by hand, a chunk at a time: what it reports
//! before and after the byte order mark is read, and that it hands out the
//! text before a problem and only then the error.

use super::Decoder;
use crate::encoding::Found;

#[test]
fn the_encoding_is_utf8_until_a_byte_order_mark_says_otherwise() {
    let mut decoder = Decoder::default();
    assert_eq!(decoder.found(), Found::Utf8);
    decoder.push(&[0xff, 0xfe, b'<', 0]);
    assert_eq!(decoder.found(), Found::Utf16Le);
    assert_eq!(decoder.pending().unwrap(), b"<");
}

#[test]
fn the_text_before_a_problem_comes_out_before_the_error() {
    let mut decoder = Decoder::default();
    decoder.push(b"<a>\x01");
    assert!(!decoder.wants_more());
    assert_eq!(decoder.pending().unwrap(), b"<a>");
    decoder.consume(3);
    assert!(decoder.pending().is_err());
    assert!(decoder.failed().is_some());
}
