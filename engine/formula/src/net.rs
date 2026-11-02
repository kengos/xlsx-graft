//! The safety net: a scan that ignores every quote and finds each word
//! followed by `(`, spaces allowed between. One that is not where the
//! lexer saw a call or a parameter is an [`UnseenCall`]: if the lexer read
//! a quote wrong, a function it hid still shows here.

use crate::facts::word::split_prefix;
use crate::token::{is_space, is_word_char, is_word_start};
use crate::{Facts, Span, UnseenCall};
use std::collections::HashSet;

/// The calls in `text` that are not in `facts`' calls or parameters.
pub(crate) fn unseen(text: &str, facts: &Facts) -> Vec<UnseenCall> {
    let seen: HashSet<usize> = facts
        .calls
        .iter()
        .map(|call| call.span.start)
        .chain(facts.params.iter().map(|param| param.span.start))
        .collect();
    call_like(text)
        .into_iter()
        .filter(|span| !seen.contains(&span.start))
        .map(|span| UnseenCall {
            name: split_prefix(text.get(span.start..span.end).unwrap_or(""))
                .1
                .to_ascii_uppercase(),
            span,
        })
        .collect()
}

/// Where each word followed by `(` is in `text`, quotes ignored. Words
/// start and go on with the characters the lexer's do; a word the lexer
/// starts inside a number (`X` in `1E5X(`) the net starts earlier
/// (`E5X`), and reports as unseen.
pub(crate) fn call_like(text: &str) -> Vec<Span> {
    let mut found = Vec::new();
    let mut word: Option<usize> = None;
    let mut before_paren: Option<Span> = None;
    for (at, c) in text.char_indices() {
        if word.is_some() && is_word_char(c) {
            continue;
        }
        if let Some(start) = word.take() {
            before_paren = Some(Span { start, end: at });
        }
        if c == '(' {
            found.extend(before_paren.take());
        } else if !is_space(c) {
            before_paren = None;
            word = is_word_start(c).then_some(at);
        }
    }
    found
}
