//! Tests for `unescape`, on text alone: whole, and in pieces, and for
//! `escape`, its inverse.

mod properties;

use super::{Xstring, escape, unescape};
use std::convert::Infallible;

/// The value of `text` handed in whole.
fn whole(text: &str) -> String {
    let mut value = String::new();
    let read = unescape::<Infallible>(text, true, |piece| {
        value.push_str(piece);
        Ok(())
    });
    assert_eq!(read, Ok(text.len()));
    value
}

/// The value of `text` handed in as the pieces `text` splits into at
/// `cuts`, each with what the one before held back.
fn in_pieces(text: &str, cuts: &[usize]) -> String {
    let mut value = String::new();
    let mut held = String::new();
    let mut start = 0;
    for (index, &end) in cuts.iter().chain([&text.len()]).enumerate() {
        held.push_str(&text[start..end]);
        start = end;
        let last = index == cuts.len();
        let read = unescape::<Infallible>(&held, last, |piece| {
            value.push_str(piece);
            Ok(())
        })
        .unwrap();
        held.drain(..read);
    }
    assert_eq!(held, "");
    value
}

#[test]
fn an_escape_stands_for_its_code_unit_in_either_case() {
    assert_eq!(whole("a_x0008_b"), "a\u{8}b");
    assert_eq!(whole("_x000d__x000A_"), "\r\n");
    assert_eq!(whole("_x3042_"), "あ");
}

#[test]
fn an_escaped_underscore_keeps_what_follows_from_being_read_as_an_escape() {
    assert_eq!(whole("_x005F_x0041_"), "_x0041_");
    assert_eq!(whole("_x005F__x0041_"), "_A");
}

#[test]
fn a_surrogate_pair_makes_one_character() {
    assert_eq!(whole("_xD83D__xDE00_"), "😀");
}

#[test]
fn what_is_not_an_escape_is_kept_as_written() {
    for text in [
        "", "a_b x", "_x", "_x004", "_x0041", "_x004G_", "_x+041_", "_X0041_", "_x00041_",
        "_xD83D_", "_xD83D_x", "_xDE00_", "__x", "_",
    ] {
        assert_eq!(whole(text), text);
    }
    // An unpaired surrogate is kept, and the escape after it read.
    assert_eq!(whole("_xD83D__x0041_"), "_xD83D_A");
}

#[test]
fn an_escape_split_across_pieces_reads_as_one() {
    let text = "ab_x0041_c_xD83D__xDE00_d_x005F_x0042_e_";
    for cut in 0..=text.len() {
        assert_eq!(in_pieces(text, &[cut]), whole(text), "cut at {cut}");
    }
    let every: Vec<usize> = (1..text.len()).collect();
    assert_eq!(in_pieces(text, &every), "abAc😀d_x0042_e_");
}

#[test]
fn what_is_held_back_is_at_most_a_pair_of_escapes() {
    let text = "abc_x00";
    let read = unescape::<Infallible>(text, false, |_| Ok(())).unwrap();
    assert_eq!(read, 3);
    let text = "_x0041_ and some more text";
    let read = unescape::<Infallible>(text, false, |_| Ok(())).unwrap();
    assert_eq!(read, text.len());
}

#[test]
fn a_failing_output_stops_the_reading() {
    assert_eq!(unescape("a", true, |_| Err(())), Err(()));
    let fail_at_a = |piece: &str| if piece == "A" { Err(()) } else { Ok(()) };
    assert_eq!(unescape("_x0041_", true, fail_at_a), Err(()));
}

/// `Xstring::begin` discards the text a previous, unfinished value held
/// back, so the next value does not join it.
#[test]
fn begin_discards_the_text_held_back_from_the_previous_value() {
    let mut xstring = Xstring::default();
    let mut first = String::new();
    xstring
        .push::<Infallible>("ab_x00", false, |piece| {
            first.push_str(piece);
            Ok(())
        })
        .unwrap();
    assert_eq!(first, "ab");
    xstring.begin();
    let mut second = String::new();
    xstring
        .push::<Infallible>("41_", true, |piece| {
            second.push_str(piece);
            Ok(())
        })
        .unwrap();
    assert_eq!(second, "41_");
}

/// A surrogate pair that ends the text is decoded in full, and all of the
/// text is reported as read, although more text may follow.
#[test]
fn a_surrogate_pair_that_ends_the_text_is_decoded_even_when_more_text_may_follow() {
    let mut value = String::new();
    let read = unescape::<Infallible>("a_xD83D__xDE00_", false, |piece| {
        value.push_str(piece);
        Ok(())
    });
    assert_eq!((read, value.as_str()), (Ok(15), "a\u{1F600}"));
}

/// The value of `text` written by `escape`, joined.
fn escaped(text: &str) -> String {
    let mut value = String::new();
    escape::<Infallible>(text, |piece| {
        value.push_str(piece);
        Ok(())
    })
    .unwrap();
    value
}

#[test]
fn a_character_that_is_not_an_xml_char_is_written_as_an_escape_in_upper_case() {
    assert_eq!(
        escaped("\u{0}\u{8}\u{b}\u{c}\u{e}\u{1f}"),
        "_x0000__x0008__x000B__x000C__x000E__x001F_"
    );
    assert_eq!(escaped("\u{fffe}\u{ffff}"), "_xFFFE__xFFFF_");
    // Tab, line feed, carriage return and the characters at the edges of
    // the ranges XML admits are kept.
    let kept = "\t\n\r \u{7f}\u{9f}\u{d7ff}\u{e000}\u{fffd}\u{10000}\u{10ffff}a😀あ";
    assert_eq!(escaped(kept), kept);
    assert_eq!(escaped(""), "");
}

#[test]
fn an_underscore_that_would_start_an_escape_is_written_as_one() {
    assert_eq!(escaped("_x0041_"), "_x005F_x0041_");
    assert_eq!(escaped("_xabcd_"), "_x005F_xabcd_");
    assert_eq!(escaped("_x0041__x0042_"), "_x005F_x0041__x005F_x0042_");
    // Any other underscore is kept.
    for kept in [
        "_", "a_b", "_x", "_x004", "_x0041", "_x004G_", "_X0041_", "_x00041_", "__x",
    ] {
        assert_eq!(escaped(kept), kept);
    }
}

/// U+0001 is written as `_x0001_`, so the `_` of `_xABCD` is followed by
/// `_` after the four digits and `_xABCD_` would be read as the escape of
/// U+ABCD.
#[test]
fn an_underscore_before_a_character_written_as_an_escape_is_written_as_one() {
    let text = "_xABCD\u{1}";
    assert_eq!(escaped(text), "_x005F_xABCD_x0001_");
    assert_eq!(whole(&escaped(text)), text);
}

#[test]
fn a_failing_output_stops_the_writing() {
    // `out` fails at its first call, and is not called again.
    let mut calls = 0;
    let result = escape("a\u{1}b", |_| {
        calls += 1;
        Err(())
    });
    assert_eq!((result, calls), (Err(()), 1));
    // `out` fails at the escape of U+0001, its second call, and is not
    // called for the `b`.
    let mut calls = 0;
    let result = escape("a\u{1}b", |piece| {
        calls += 1;
        if piece.starts_with("_x") {
            Err(())
        } else {
            Ok(())
        }
    });
    assert_eq!((result, calls), (Err(()), 2));
    // `out` fails at the last piece.
    assert_eq!(escape("a", |_| Err(())), Err(()));
}
