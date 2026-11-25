//! Properties of any string: it lexes without a panic, its tokens join
//! back to its decoded text, and every call the quote-blind scan finds is
//! a call, a parameter or an unseen call, and the operand of each sheet
//! prefix lies after the prefix, inside the text, and is separated from it
//! by whitespace only, and the ranges from each prefix to the end of its
//! operand are nested or disjoint. The generator has pieces that put a call
//! right after a prefix (`Sheet1!MyFn(`, `Sheet1!SUM(`) and a `)`, so that
//! nested ranges do come up.

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
        1 => proptest::sample::select(&["_x0041_", "_x005F_", "_xD800_", "_xlfn.", "_xlpm.", "_xleta.", "SUM(", "#REF!", "Sheet1!", "S1:S3!", "A1:B2", "MyFn(", "[1]"][..]).prop_map(String::from),
        1 => proptest::sample::select(&["Sheet1!MyFn(", "Sheet1!SUM(", "Sheet1!A1", ")"][..]).prop_map(String::from),
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

    #[test]
    fn the_operand_of_a_prefix_follows_it_after_whitespace(text in formula()) {
        let read = analyze(&text);
        for prefix in &read.facts.sheet_prefixes {
            let Some(operand) = prefix.operand else { continue };
            prop_assert!(prefix.span.end <= operand.start, "{:?} in {:?}", operand, read.text);
            prop_assert!(operand.start <= operand.end);
            prop_assert!(operand.end <= read.text.len());
            prop_assert!(read.text.is_char_boundary(operand.start));
            prop_assert!(read.text.is_char_boundary(operand.end));
            let between = read.text.get(prefix.span.end..operand.start);
            prop_assert!(
                between.is_some_and(|gap| gap.chars().all(char::is_whitespace)),
                "{:?} in {:?}", operand, read.text
            );
        }
    }

    #[test]
    fn the_ranges_of_prefixes_and_operands_nest_or_do_not_overlap(text in formula()) {
        let read = analyze(&text);
        let ranges: Vec<(usize, usize)> = read.facts.sheet_prefixes.iter()
            .filter_map(|prefix| prefix.operand.map(|operand| (prefix.span.start, operand.end)))
            .collect();
        for (index, &(a_start, a_end)) in ranges.iter().enumerate() {
            for &(b_start, b_end) in &ranges[index + 1..] {
                let disjoint = a_end <= b_start || b_end <= a_start;
                let nested = (a_start <= b_start && b_end <= a_end)
                    || (b_start <= a_start && a_end <= b_end);
                prop_assert!(disjoint || nested, "{:?} {:?} in {:?}", (a_start, a_end), (b_start, b_end), read.text);
            }
        }
    }
}
