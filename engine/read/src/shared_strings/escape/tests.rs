//! Tests for `unescape`, on text alone: whole, and in pieces.

use super::unescape;
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
