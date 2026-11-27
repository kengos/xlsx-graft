//! The base64 encoder: the examples of RFC 4648 §10 and a round trip
//! through a decoder written here.

use crate::sheet::base64::encode;
use proptest::prelude::*;

/// The standard alphabet's decoder, with `=` padding.
pub(super) fn decode(text: &str) -> Vec<u8> {
    let value = |c: char| match c {
        'A'..='Z' => c as u32 - 'A' as u32,
        'a'..='z' => c as u32 - 'a' as u32 + 26,
        '0'..='9' => c as u32 - '0' as u32 + 52,
        '+' => 62,
        '/' => 63,
        _ => panic!("not base64: {c:?}"),
    };
    let body = text.trim_end_matches('=');
    let mut out = Vec::new();
    let (mut bits, mut count) = (0_u32, 0);
    for c in body.chars() {
        bits = bits << 6 | value(c);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count & 0xff) as u8);
        }
    }
    out
}

#[test]
fn the_examples_of_rfc_4648_section_10_are_written_with_padding() {
    for (plain, text) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(encode(plain.as_bytes()), text);
    }
}

#[test]
fn the_alphabet_ends_with_plus_and_slash() {
    assert_eq!(encode(&[0xfb, 0xff, 0xbf]), "+/+/");
    assert_eq!(encode(&[0x00, 0x10, 0x83]), "ABCD");
    assert_eq!(encode(&[0xff; 16]), "/////////////////////w==");
}

proptest! {
    #[test]
    fn what_is_written_decodes_to_the_bytes(bytes in prop::collection::vec(any::<u8>(), 0..100)) {
        let text = encode(&bytes);
        prop_assert_eq!(text.len(), bytes.len().div_ceil(3) * 4);
        prop_assert_eq!(decode(&text), bytes);
    }
}
