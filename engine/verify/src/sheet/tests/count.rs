//! How the checks read a number and count a string's length, as the text
//! arrives in pieces.

use crate::sheet::count::{Unsigned, Utf16};

/// The length of the value that arrives as `pieces`.
fn units(pieces: &[&str]) -> u64 {
    let mut length = Utf16::default();
    for piece in pieces {
        length.push(piece);
    }
    length.end();
    length.units()
}

#[test]
fn a_number_is_digits_with_white_space_around_them() {
    assert_eq!(Unsigned::parse("42"), Some(42));
    assert_eq!(Unsigned::parse(" \t\r\n42\n "), Some(42));
    assert_eq!(Unsigned::parse("0042"), Some(42));
    // XML Schema takes a `+` before the digits, as `SheetRows` does.
    assert_eq!(Unsigned::parse(" +42 "), Some(42));
    for text in [
        "", " ", "4 2", "+", "+ 42", "++42", "4+2", "42+", "-1", "4.0", "x", "４２",
    ] {
        assert_eq!(Unsigned::parse(text), None, "{text:?}");
    }
    assert_eq!(Unsigned::parse(&"9".repeat(40)), Some(u64::MAX));
    let mut number = Unsigned::default();
    for piece in [" 1", "2", "3 "] {
        number.push(piece);
    }
    assert_eq!(number.value(), Some(123));
}

#[test]
fn a_string_counts_utf16_code_units() {
    assert_eq!(units(&["abc"]), 3);
    assert_eq!(units(&["é😀"]), 3);
    assert_eq!(units(&[]), 0);
}

#[test]
fn an_escape_counts_as_the_one_unit_it_stands_for() {
    assert_eq!(units(&["_x0041_"]), 1);
    assert_eq!(units(&["_x00e9__xD83D__xde00_"]), 3);
    // Read once from the left: `_x005F_` is `_`, and what follows it is
    // no escape.
    assert_eq!(units(&["_x005F_x0041_"]), 7);
    assert_eq!(units(&["__x0041_"]), 2);
    assert_eq!(units(&["___x0041_"]), 3);
}

#[test]
fn what_is_no_escape_counts_as_written() {
    for text in [
        "_", "_x", "_x004", "_x0041", "_y0041_", "_x00G1_", "_x0041x", "x_",
    ] {
        let written = u64::try_from(text.len()).unwrap();
        assert_eq!(units(&[text]), written, "{text:?}");
    }
    // A character that breaks an escape may start the next one.
    assert_eq!(units(&["_x00_x0041_"]), 5);
    assert_eq!(units(&["_x0😀"]), 5);
}

#[test]
fn an_escape_counts_across_the_pieces_it_arrives_in() {
    assert_eq!(units(&["_", "x", "00", "41", "_"]), 1);
    assert_eq!(units(&["a_x00", "41_b"]), 3);
    // Held back, it still counts before the value ends.
    let mut length = Utf16::default();
    length.push("ab_x00");
    assert_eq!(length.units(), 6);
}
