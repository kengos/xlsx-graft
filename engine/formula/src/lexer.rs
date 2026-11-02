//! Cuts a formula's decoded text into tokens, in four states: the default
//! one here, and a string, a quoted sheet name and brackets in `quoted`.
//! Each token is as long as its scanner says, so the tokens' text joined is
//! the text.

mod quoted;
mod word;

use crate::token::{is_space, is_word_start};
use crate::{Malformed, MalformedReason, Span, Token, TokenKind};

/// How deep brackets may nest: a structured reference nests two deep
/// (`T[[#This Row],[a]]`). Deeper, the rest of the formula is one
/// malformed token.
pub const MAX_BRACKET_DEPTH: u32 = 64;

/// What a scanner read at the start of the rest of the text.
pub(crate) struct Scan {
    pub(crate) kind: TokenKind,
    /// Its length in bytes, never 0, ending on a character boundary.
    pub(crate) len: usize,
    /// Why it is malformed, if it is.
    pub(crate) problem: Option<MalformedReason>,
}

impl Scan {
    /// A well-formed token of `kind`, `len` bytes long.
    pub(crate) fn ok(kind: TokenKind, len: usize) -> Self {
        Scan {
            kind,
            len,
            problem: None,
        }
    }

    /// The whole of `rest` as one malformed token, for `reason`.
    pub(crate) fn rest_malformed(rest: &str, reason: MalformedReason) -> Self {
        Scan {
            kind: TokenKind::Malformed,
            len: rest.len(),
            problem: Some(reason),
        }
    }
}

/// The tokens of `text`, and where it is malformed.
pub(crate) fn lex(text: &str) -> (Vec<Token>, Vec<Malformed>) {
    let mut tokens = Vec::new();
    let mut malformed = Vec::new();
    let mut rest = text;
    let mut start = 0usize;
    while let Some(c) = rest.chars().next() {
        let scan = next(c, rest);
        let (_, after) = rest.split_at_checked(scan.len).unwrap_or((rest, ""));
        let end = start.saturating_add(rest.len().saturating_sub(after.len()));
        let span = Span { start, end };
        tokens.push(Token {
            kind: scan.kind,
            span,
        });
        if let Some(reason) = scan.problem {
            malformed.push(Malformed { reason, span });
        }
        rest = after;
        start = end;
    }
    (tokens, malformed)
}

/// The token `rest` starts with; `c` is its first character.
fn next(c: char, rest: &str) -> Scan {
    match c {
        '"' => quoted::string(rest),
        '\'' => quoted::sheet_name(rest),
        '[' => quoted::bracket(rest),
        '#' => word::hash(rest),
        _ if is_space(c) => Scan::ok(TokenKind::Whitespace, run(rest, is_space)),
        _ if word::starts_number(rest) => word::number(rest),
        _ if is_word_start(c) => word::word(rest),
        _ => punctuation(c, rest),
    }
}

/// An operator or a punctuator, or one malformed character.
fn punctuation(c: char, rest: &str) -> Scan {
    if ["<>", "<=", ">="].iter().any(|op| rest.starts_with(op)) {
        return Scan::ok(TokenKind::Operator, 2);
    }
    let kind = match c {
        '+' | '-' | '*' | '/' | '^' | '&' | '=' | '<' | '>' | '%' | '@' => TokenKind::Operator,
        ':' => TokenKind::Colon,
        '!' => TokenKind::Bang,
        '|' => TokenKind::Pipe,
        ',' => TokenKind::Comma,
        ';' => TokenKind::Semicolon,
        '(' => TokenKind::OpenParen,
        ')' => TokenKind::CloseParen,
        '{' => TokenKind::OpenBrace,
        '}' => TokenKind::CloseBrace,
        _ => {
            return Scan {
                kind: TokenKind::Malformed,
                len: c.len_utf8(),
                problem: Some(MalformedReason::UnknownCharacter),
            };
        }
    };
    Scan::ok(kind, 1)
}

/// The length of the run of characters `rest` starts with that `keep`
/// keeps.
pub(crate) fn run(rest: &str, keep: impl Fn(char) -> bool) -> usize {
    rest.char_indices()
        .find(|&(_, c)| !keep(c))
        .map_or(rest.len(), |(at, _)| at)
}
