//! Properties of `escape`: `unescape` of what it writes is the text.

use super::{escape, unescape};
use proptest::prelude::*;
use std::convert::Infallible;

/// The characters of the cases the escapes turn on, over-weighted: the
/// underscore, `x`, hexadecimal digits, a control character, the two
/// non-characters, and some past the BMP.
const ALPHABET: &[char] = &[
    '_', '_', '_', 'x', 'x', 'X', '0', '0', '5', 'F', 'f', 'A', 'a', 'D', '8', '3', 'E', 'g', ' ',
    '\u{0}', '\u{1}', '\u{8}', '\u{1f}', '\t', '\n', '\r', '\u{fffe}', '\u{ffff}', '\u{fffd}',
    '😀', 'あ', '&', '<',
];

/// A string of [`ALPHABET`], and sometimes any `char`.
fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            8 => prop::sample::select(ALPHABET),
            1 => any::<char>(),
        ],
        0..40,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

/// A string made of the pieces of escapes, so that most strings hold some.
fn pieces() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(
            &[
                "_x", "_", "x", "ABCD", "0041", "d83d", "_x005F_", "\u{1}", "_xABCD",
            ][..],
        ),
        0..12,
    )
    .prop_map(|pieces| pieces.concat())
}

/// `text` written by `escape`, then read by `unescape`.
fn round_trip(text: &str) -> (String, String) {
    let mut written = String::new();
    escape::<Infallible>(text, |piece| {
        written.push_str(piece);
        Ok(())
    })
    .unwrap();
    let mut read = String::new();
    let used = unescape::<Infallible>(&written, true, |piece| {
        read.push_str(piece);
        Ok(())
    })
    .unwrap();
    assert_eq!(used, written.len());
    (written, read)
}

proptest! {
    #[test]
    fn what_escape_writes_unescape_reads_back(text in text()) {
        let (written, read) = round_trip(&text);
        prop_assert_eq!(read, text);
        // What is written holds no character that is not an XML 1.0 `Char`.
        prop_assert!(written.chars().all(|c| !crate::escape::is_not_xml_char(c)));
    }

    #[test]
    fn what_escape_writes_of_pieces_of_escapes_unescape_reads_back(text in pieces()) {
        prop_assert_eq!(round_trip(&text).1, text);
    }

    #[test]
    fn any_string_is_read_back(text in any::<String>()) {
        prop_assert_eq!(round_trip(&text).1, text);
    }
}
