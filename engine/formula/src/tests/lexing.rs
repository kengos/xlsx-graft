//! The tokens a formula lexes to, where it is malformed, and the bound on
//! nested brackets.

use super::texts;
use crate::{MAX_BRACKET_DEPTH, Malformed, MalformedReason, Span, TokenKind, analyze};

#[test]
fn every_kind_of_token_is_cut_where_it_ends() {
    use TokenKind::{
        Bang, Bracket, CloseBrace, CloseParen, Colon, Comma, ErrorValue, Number, OpenBrace,
        OpenParen, Operator, Pipe, Semicolon, SheetName, String, Whitespace, Word,
    };
    let read = analyze("{1,.5;2E+3,1e}<>'a'!A1# |\"s\"<=#n/a>=(@[x])^#REF!:%");
    let kinds: Vec<_> = read.tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        texts(&read),
        [
            "{", "1", ",", ".5", ";", "2E+3", ",", "1", "e", "}", "<>", "'a'", "!", "A1", "#", " ",
            "|", "\"s\"", "<=", "#n/a", ">=", "(", "@", "[x]", ")", "^", "#REF!", ":", "%",
        ]
    );
    assert_eq!(
        kinds,
        [
            OpenBrace, Number, Comma, Number, Semicolon, Number, Comma, Number, Word, CloseBrace,
            Operator, SheetName, Bang, Word, Operator, Whitespace, Pipe, String, Operator,
            ErrorValue, Operator, OpenParen, Operator, Bracket, CloseParen, Operator, ErrorValue,
            Colon, Operator,
        ]
    );
    assert!(read.facts.malformed.is_empty());
}

#[test]
fn a_character_no_token_starts_with_is_malformed_alone() {
    let read = analyze("1?]+.x");
    assert_eq!(texts(&read), ["1", "?", "]", "+", ".", "x"]);
    let malformed: Vec<_> = read
        .facts
        .malformed
        .iter()
        .map(|m| (m.reason, m.span.start))
        .collect();
    let unknown = MalformedReason::UnknownCharacter;
    assert_eq!(malformed, [(unknown, 1), (unknown, 2), (unknown, 4)]);
}

#[test]
fn a_quoted_sheet_name_with_nothing_after_it_is_malformed() {
    let read = analyze("'x'+1");
    assert_eq!(
        read.facts.malformed,
        [Malformed {
            reason: MalformedReason::StraySheetName,
            span: Span { start: 0, end: 3 },
        }]
    );
}

#[test]
fn an_escape_naming_a_surrogate_stays_as_written() {
    let read = analyze("_xD800_+_x0041_+_xZZZZ_+_x00");
    assert_eq!(read.text, "_xD800_+A+_xZZZZ_+_x00");
    assert_eq!(
        read.facts.malformed,
        [Malformed {
            reason: MalformedReason::UndecodableEscape,
            span: Span { start: 0, end: 7 },
        }]
    );
}

#[test]
fn brackets_nest_up_to_the_bound_and_no_deeper() {
    let depth = usize::try_from(MAX_BRACKET_DEPTH).unwrap();
    let at_bound = format!("{}{}+F(1)", "[".repeat(depth), "]".repeat(depth));
    let read = analyze(&at_bound);
    assert!(read.facts.malformed.is_empty());
    assert_eq!(
        read.tokens.first().map(|t| t.kind),
        Some(TokenKind::Bracket)
    );
    assert_eq!(super::calls(&at_bound), ["F"]);

    let past = format!("{}{}+F(1)", "[".repeat(depth + 1), "]".repeat(depth + 1));
    let read = analyze(&past);
    assert_eq!(texts(&read), [past.as_str()]);
    let reasons: Vec<_> = read.facts.malformed.iter().map(|m| m.reason).collect();
    assert_eq!(reasons, [MalformedReason::BracketsTooDeep]);
    assert_eq!(super::unseen(&past), ["F"]);
}

#[test]
fn a_span_that_is_not_the_formulas_slices_to_nothing() {
    let read = analyze("A1");
    assert_eq!(read.slice(Span { start: 1, end: 9 }), "");
}
