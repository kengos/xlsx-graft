//! Words, numbers and what starts with `#`.

use super::{Scan, run};
use crate::TokenKind;
use crate::token::is_word_char;

/// The error constants (ECMA-376-1:2016 §18.17.2, `error-constant`), and
/// the ones newer Excel shows. Compared without regard to letter case.
const ERRORS: [&str; 15] = [
    "#DIV/0!",
    "#N/A",
    "#NAME?",
    "#NULL!",
    "#NUM!",
    "#REF!",
    "#VALUE!",
    "#GETTING_DATA",
    "#SPILL!",
    "#CALC!",
    "#FIELD!",
    "#BLOCKED!",
    "#CONNECT!",
    "#UNKNOWN!",
    "#BUSY!",
];

/// A word: a run of word characters.
pub(super) fn word(rest: &str) -> Scan {
    Scan::ok(TokenKind::Word, run(rest, is_word_char))
}

/// An error constant, or else `#` alone: the spill operator after a
/// reference (`A1#`).
pub(super) fn hash(rest: &str) -> Scan {
    let error = ERRORS.iter().find(|error| {
        rest.get(..error.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(error))
    });
    match error {
        Some(error) => Scan::ok(TokenKind::ErrorValue, error.len()),
        None => Scan::ok(TokenKind::Operator, 1),
    }
}

/// Whether `rest` starts with a number: a digit, or `.` and a digit.
pub(super) fn starts_number(rest: &str) -> bool {
    let bytes = rest.as_bytes();
    let digit = |at: usize| bytes.get(at).is_some_and(u8::is_ascii_digit);
    digit(0) || (bytes.first() == Some(&b'.') && digit(1))
}

/// A number: digits, then `.` and digits, then `E`, a sign and digits,
/// each part there only if it is whole (§18.17.2, `numerical-constant`).
pub(super) fn number(rest: &str) -> Scan {
    let bytes = rest.as_bytes();
    let digits = |from: usize| {
        from.saturating_add(bytes.get(from..).map_or(0, |tail| {
            tail.iter().take_while(|b| b.is_ascii_digit()).count()
        }))
    };
    let mut len = digits(0);
    if bytes.get(len) == Some(&b'.') {
        len = digits(len.saturating_add(1));
    }
    if matches!(bytes.get(len), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(
            bytes.get(len.saturating_add(1)),
            Some(b'+' | b'-')
        ));
        let from = len.saturating_add(1).saturating_add(sign);
        let end = digits(from);
        if end > from {
            len = end;
        }
    }
    Scan::ok(TokenKind::Number, len)
}
