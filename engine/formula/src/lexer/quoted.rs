//! The three quoted states: a string, a quoted sheet name and brackets.
//! Each reads to its closing character; one left open makes the rest of
//! the text a malformed token, so whatever follows is never read as code.

use super::{MAX_BRACKET_DEPTH, Scan};
use crate::{MalformedReason, TokenKind};

/// A string, `"…"`, where `""` is one `"` (ECMA-376 Part 1 §18.17.2).
pub(super) fn string(rest: &str) -> Scan {
    match close_doubled(rest, '"') {
        Some(len) => Scan::ok(TokenKind::String, len),
        None => Scan::rest_malformed(rest, MalformedReason::UnclosedString),
    }
}

/// A quoted sheet name, `'…'`, where `''` is one `'` (§18.17.2.3,
/// `sheet-name-special`).
pub(super) fn sheet_name(rest: &str) -> Scan {
    match close_doubled(rest, '\'') {
        Some(len) => Scan::ok(TokenKind::SheetName, len),
        None => Scan::rest_malformed(rest, MalformedReason::UnclosedSheetName),
    }
}

/// The length of the quoted text `rest` starts with, up to and including
/// the `quote` that closes it, where a doubled `quote` is one character.
fn close_doubled(rest: &str, quote: char) -> Option<usize> {
    let mut chars = rest.char_indices().skip(1).peekable();
    while let Some((at, c)) = chars.next() {
        if c != quote {
            continue;
        }
        if chars.next_if(|&(_, next)| next == quote).is_none() {
            return Some(at.saturating_add(c.len_utf8()));
        }
    }
    None
}

/// Brackets, `[…]`, up to the `]` that closes the first `[`. Inside, `'`
/// escapes the next character ([MS-XLSX] §2.2.2, `escape-column-character`:
/// `'`, `[`, `]` and `#` in a column name) and a `"` is just a character.
/// Nested brackets are counted, not recursed into; past
/// [`MAX_BRACKET_DEPTH`] the rest is malformed.
pub(super) fn bracket(rest: &str) -> Scan {
    let mut depth = 0u32;
    let mut chars = rest.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '\'' => {
                chars.next();
            }
            '[' => {
                depth = depth.saturating_add(1);
                if depth > MAX_BRACKET_DEPTH {
                    return Scan::rest_malformed(rest, MalformedReason::BracketsTooDeep);
                }
            }
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Scan::ok(TokenKind::Bracket, at.saturating_add(1));
                }
            }
            _ => {}
        }
    }
    Scan::rest_malformed(rest, MalformedReason::UnclosedBracket)
}
