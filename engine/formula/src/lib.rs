//! xlsx-graft formula: the facts a formula states, read by a lexer that
//! does not trust it.
//!
//! [`analyze`] takes the text of a formula as a workbook stores it (the
//! content of an `<f>`, a `definedName` and the like, with no leading `=`)
//! and returns a [`Formula`]: the text with its `_xHHHH_` escapes decoded,
//! the [`Token`]s it lexes to, and the [`Facts`] read from them. It reads no
//! zip and no XML, and it judges nothing: which function is allowed is for
//! its caller, with the workbook's defined names at hand.
//!
//! # Why a lexer and not a pattern
//!
//! A pattern that looks for `NAME(` is told where a string or a sheet name
//! starts by a quote, and a formula has three kinds of quote that each
//! escape differently, so a pattern is easy to lead astray:
//!
//! - in a structured reference `'` escapes the next character, so
//!   `T[a']]+WEBSERVICE("x")` closes the reference after `]]`, where a
//!   pattern reading `'` as a sheet name's quote sees no call at all;
//! - a column name may hold `"` (`T[a"b]+CALL(…)`), which is no string;
//! - `_xHHHH_` hides any character, `_x0057_EBSERVICE(` among them.
//!
//! # What it reads
//!
//! 1. **Escapes**: `_xHHHH_` is decoded once, left to right, so the `_`
//!    an `_x005F_` gives back starts nothing (`_x005F_x0041_` is
//!    `_x0041_`). One naming a surrogate, and one written `_XHHHH_`, stay
//!    as written and are [`Malformed`]. Every [`Span`] is a byte range of
//!    the decoded text.
//! 2. **Tokens**, in four states: the default one, a string (`"…"`, `""`
//!    escapes), a quoted sheet name (`'…'`, `''` escapes) and brackets
//!    (`[…]`, where `'` escapes the next character and nested brackets are
//!    counted, not recursed into, up to [`MAX_BRACKET_DEPTH`]). The tokens'
//!    text joined is the decoded text, byte for byte. A string, sheet name
//!    or bracket left open, or brackets nested too deep, make the rest of
//!    the text one malformed token; a character no token starts with is one
//!    of its own, and lexing goes on after it.
//! 3. **Facts**: the [`Call`]s (with the functions named as values, such
//!    as `_xleta.SUM`), the LAMBDA [`Param`]s (`_xlpm.x`), the
//!    [`Reference`]s, the [`Name`]s, the [`DdeLink`]s, the
//!    [`ExternalBook`]s and the [`Malformed`] places.
//! 4. **A safety net**: a second scan that ignores every quote finds each
//!    `NAME(` in the text; one the lexer did not see as a call is an
//!    [`UnseenCall`]. A string holding `"SUM("` is one too: the net is for a
//!    lexer that reads a formula wrong, and it errs towards reporting.
//!
//! # Failing closed
//!
//! A misread must make a formula look more dangerous, never less.
//!
//! - A caller refuses a formula with any [`Malformed`] place: past one,
//!   what the tokens state is a guess.
//! - A name with no `(` after it is still a [`Name`], which a caller
//!   treats as a possible function (a defined name can hold a LAMBDA,
//!   called by its name).
//! - A prefixed word (`_xlfn.`, `_xlws.`, `_xludf.`, `_xleta.`) is a
//!   [`Call`] with or without `(`, and before brackets.
//! - A name followed by `(` after spaces is a call too.
//!
//! # Sources
//!
//! - ECMA-376-1:2016 §18.17.2 (the formula grammar: strings, sheet names, cell
//!   references, functions, names, error constants) and §22.9.2.19
//!   (`ST_Xstring`, the `_xHHHH_` escape).
//! - [MS-XLSX] §2.2.2 Formulas: structured references (`'` escapes `[`,
//!   `]`, `#` and `'` in a column name), the `_xlfn.` and `_xlfn._xlws.`
//!   prefixes and the LAMBDA parameter names.
//! - Excel's own files, for what [MS-XLSX] does not write down: the
//!   `_xlpm.` prefix a parameter is saved with, `_xleta.` on a function
//!   passed as a value, `_xludf.`, and the DDE form `app|topic!item`.
//!
//! # Bounds
//!
//! Work is linear in the text and nothing recurses. The text's length is
//! bounded by the caller (an XML event is at most `max_event_bytes`); there
//! are no more tokens than bytes, and no more facts than tokens.

mod facts;
mod lexer;
mod net;
mod token;
mod unescape;

pub use facts::{
    Call, DdeLink, ExternalBook, Facts, Location, Malformed, MalformedReason, Name, Param,
    Reference, ReferenceKind, UnseenCall,
};
pub use lexer::MAX_BRACKET_DEPTH;
pub use token::{Span, Token, TokenKind};

/// A formula, read: its decoded text, its tokens and what they state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    /// The text with its `_xHHHH_` escapes decoded once; every span is a
    /// byte range of it.
    pub text: String,
    /// The tokens, in order; their text joined is [`Formula::text`].
    pub tokens: Vec<Token>,
    /// What the tokens state.
    pub facts: Facts,
}

impl Formula {
    /// The text `span` covers, or `""` for a span that is not one of this
    /// formula's.
    #[must_use]
    pub fn slice(&self, span: Span) -> &str {
        self.text.get(span.start..span.end).unwrap_or("")
    }
}

/// Reads `formula`, the text of a formula with no leading `=`.
#[must_use]
pub fn analyze(formula: &str) -> Formula {
    let (text, mut malformed) = unescape::unescape(formula);
    let (tokens, lexed) = lexer::lex(&text);
    malformed.extend(lexed);
    let mut facts = facts::read(&text, &tokens, malformed);
    facts.unseen_calls = net::unseen(&text, &facts);
    Formula {
        text,
        tokens,
        facts,
    }
}

#[cfg(test)]
mod tests;
