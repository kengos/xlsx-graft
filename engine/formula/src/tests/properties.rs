//! Properties of any string: it lexes without a panic, its tokens join
//! back to its decoded text, and every call the quote-blind scan finds is
//! a call, a parameter or an unseen call.

use crate::analyze;
use crate::net::call_like;
use proptest::prelude::*;

/// The characters formulas are made of, quotes and escapes over-weighted,
/// and a few past ASCII.
const ALPHABET: &[char] = &[
    'A',
    'B',
    'x',
    '_',
    '0',
    '1',
    '5',
    'F',
    'D',
    '.',
    '$',
    '\\',
    '?',
    '"',
    '"',
    '\'',
    '\'',
    '[',
    '[',
    ']',
    ']',
    '#',
    '!',
    '|',
    ':',
    ',',
    ';',
    '(',
    '(',
    ')',
    '{',
    '}',
    '+',
    '-',
    '<',
    '>',
    '=',
    '@',
    '%',
    '&',
    ' ',
    '\n',
    'é',
    '表',
    '\u{0}',
    '\u{1F600}',
];

fn formula() -> impl Strategy<Value = String> {
    let pieces = prop_oneof![
        4 => proptest::sample::select(ALPHABET).prop_map(String::from),
        1 => proptest::sample::select(&["_x0041_", "_x005F_", "_xD800_", "_xlfn.", "_xlpm.", "_xleta.", "SUM(", "#REF!"][..]).prop_map(String::from),
        1 => any::<char>().prop_map(String::from),
    ];
    proptest::collection::vec(pieces, 0..48).prop_map(|pieces| pieces.concat())
}

proptest! {
    #[test]
    fn any_string_lexes_back_to_its_text(text in formula()) {
        let read = analyze(&text);
        let joined: String = read.tokens.iter().map(|t| read.slice(t.span)).collect();
        prop_assert_eq!(joined, read.text.clone());
        let mut end = 0;
        for token in &read.tokens {
            prop_assert_eq!(token.span.start, end);
            prop_assert!(token.span.end > token.span.start);
            end = token.span.end;
        }
    }

    #[test]
    fn every_call_like_word_is_reported(text in formula()) {
        let read = analyze(&text);
        let facts = &read.facts;
        for span in call_like(&read.text) {
            let reported = facts.calls.iter().map(|c| c.span)
                .chain(facts.params.iter().map(|p| p.span))
                .chain(facts.unseen_calls.iter().map(|u| u.span))
                .any(|s| s.start == span.start);
            prop_assert!(reported, "{:?} in {:?}", span, read.text);
        }
    }
}
