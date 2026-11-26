//! Reads the facts from a formula's tokens, left to right, whitespace left
//! out. A sheet or book prefix (`Sheet1!`, `[1]`, `'C:\d\[b.xlsx]S'!`) is
//! held for the token after it; nothing is held across a `:`, and the cell
//! that ends a range looks back for the location of the one that starts it.
//! The tokens after a prefix give the operand it qualifies.

mod operand;
mod prefix;
mod range;
mod sheet;

use super::word::{is_cell, is_param, split_prefix, unquote};
use crate::{Call, Facts, Location, Name, Param, Reference, ReferenceKind, Span, Token, TokenKind};

/// Adds to `facts` what `tokens` of `text` state.
pub(super) fn walk(text: &str, tokens: &[Token], facts: &mut Facts) {
    let tokens = tokens
        .iter()
        .copied()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .collect();
    let mut walker = Walker {
        text,
        tokens,
        at: 0,
        pending: Location::default(),
        book_start: None,
        next_prefix: None,
        prefix: None,
        callee: None,
        open: Vec::new(),
        ended: None,
        facts,
    };
    walker.run();
}

/// The walk's state.
struct Walker<'a> {
    text: &'a str,
    tokens: Vec<Token>,
    /// The token being read.
    at: usize,
    /// The book and sheet the token being read is prefixed with.
    pending: Location,
    /// Where the `[1]` that gave `pending` its book starts.
    book_start: Option<usize>,
    /// The prefix just read, whose operand, if it has one, starts at the next
    /// token.
    next_prefix: Option<usize>,
    /// The prefix before the token being read: its operand starts there.
    prefix: Option<usize>,
    /// The prefix a call is the operand of and where the call starts, until
    /// the `(` after the call's name.
    callee: Option<(usize, usize)>,
    /// For each `(` not yet closed, what it opens: the prefix a call is the
    /// operand of and where that call starts.
    open: Vec<Option<(usize, usize)>>,
    /// The prefix whose operand ends with the token just read.
    ended: Option<usize>,
    facts: &'a mut Facts,
}

impl<'a> Walker<'a> {
    /// Reads every token, each handler saying how many it used.
    fn run(&mut self) {
        while let Some(token) = self.peek(0) {
            let location = std::mem::take(&mut self.pending);
            let from = self.book_start.take();
            self.prefix = self.next_prefix.take();
            let ended = self.ended.take();
            let used = match token.kind {
                TokenKind::Word => self.word(token, location, from),
                TokenKind::Number => self.range(token, location).unwrap_or(1),
                TokenKind::SheetName => self.sheet_name(token, location, from),
                TokenKind::Bracket => self.bracket(token, location),
                TokenKind::Pipe => self.dde(None, 0),
                TokenKind::ErrorValue => self.error_value(token),
                TokenKind::OpenParen => self.open_paren(token, ended),
                TokenKind::CloseParen => self.close_paren(token),
                TokenKind::Operator => self.spill(token, ended),
                _ => 1,
            };
            // At least one: no handler uses none today, and this keeps a future
            // one from hanging the walk.
            self.at = self.at.saturating_add(used.max(1));
        }
    }

    /// A word: a DDE application, a sheet (alone or the first of two), a
    /// call, a range of columns or rows, a table, or one word alone.
    fn word(&mut self, token: Token, location: Location, from: Option<usize>) -> usize {
        let text = self.text(token);
        match self.kind(1) {
            Some(TokenKind::Pipe) => return self.dde(Some(token), 1),
            Some(TokenKind::Bang) => {
                let start = from.unwrap_or(token.span.start);
                self.sheet_prefix(start, 1, location.book.clone(), (text, None));
                self.pending = Location {
                    book: location.book,
                    sheet: Some(text.to_owned()),
                };
                return 2;
            }
            Some(TokenKind::OpenParen) => {
                self.call(token, false, location);
                return 1;
            }
            _ => {}
        }
        // A word right after a sheet prefix does not start a 3-D prefix: in
        // `Sheet1!A1:Sheet1!B2`, `A1`, `:` and `Sheet1!` are a cell, the
        // range operator and a sheet prefix. A word with no prefix before it
        // starts a 3-D prefix when `:`, a word and `!` follow it, in the form
        // of a cell or not: `S1:S3!A1` names the sheets `S1` to `S3`.
        let three_d = location.sheet.is_none();
        if let Some(last) = self
            .peek(2)
            .filter(|_| three_d && self.kind(1) == Some(TokenKind::Colon))
            && last.kind == TokenKind::Word
            && self.kind(3) == Some(TokenKind::Bang)
        {
            let (start, to) = (from.unwrap_or(token.span.start), self.text(last));
            self.sheet_prefix(start, 3, location.book.clone(), (text, Some(to)));
            self.pending = Location {
                book: location.book,
                sheet: Some(format!("{text}:{to}")),
            };
            return 4;
        }
        // A prefixed word is a function, brackets after it or not.
        let table = split_prefix(text).0.is_empty();
        if let Some(bracket) = self.peek(1).filter(|next| {
            table && next.kind == TokenKind::Bracket && next.span.start == token.span.end
        }) {
            self.reference(
                ReferenceKind::Table,
                location,
                text,
                token.span.start,
                bracket,
            );
            return 2;
        }
        if let Some(used) = self.range(token, location.clone()) {
            return used;
        }
        self.alone(token, location);
        1
    }

    /// A word that is nothing more: a prefixed name, `TRUE` or `FALSE`, a
    /// cell, or a name.
    fn alone(&mut self, token: Token, location: Location) {
        let text = self.text(token);
        let boolean = text.eq_ignore_ascii_case("TRUE") || text.eq_ignore_ascii_case("FALSE");
        if !split_prefix(text).0.is_empty() {
            self.call(token, true, location);
        } else if is_cell(text) {
            let started = self.range_start();
            if started.is_some() {
                self.extend_range(token.span.end);
            } else {
                self.extend_prefixed_range(token.span.end);
            }
            let location = started.unwrap_or(location);
            self.reference(ReferenceKind::Cell, location, text, token.span.start, token);
        } else if !boolean {
            self.qualify(token.span);
            self.facts.names.push(Name {
                name: text.to_ascii_uppercase(),
                location,
                span: token.span,
            });
        }
    }

    /// A call to the function `token` names, or a parameter. `passed` when
    /// no `(` follows.
    fn call(&mut self, token: Token, passed: bool, location: Location) {
        let (prefix, base) = split_prefix(self.text(token));
        if is_param(prefix) {
            self.facts.params.push(Param {
                name: base.to_ascii_uppercase(),
                span: token.span,
            });
            return;
        }
        if !passed {
            self.callee(token);
        }
        self.facts.calls.push(Call {
            name: base.to_ascii_uppercase(),
            prefix: prefix.to_owned(),
            location,
            passed,
            span: token.span,
        });
    }

    /// A reference of `kind` at `location`, written `target`, from `start`
    /// to the end of `last`.
    fn reference(
        &mut self,
        kind: ReferenceKind,
        location: Location,
        target: &str,
        start: usize,
        last: Token,
    ) {
        let span = Span {
            start,
            end: last.span.end,
        };
        self.qualify(span);
        self.facts.references.push(Reference {
            kind,
            location,
            target: target.to_owned(),
            span,
        });
    }

    /// The token `ahead` of the one being read.
    fn peek(&self, ahead: usize) -> Option<Token> {
        self.tokens.get(self.at.saturating_add(ahead)).copied()
    }

    /// The kind of the token `ahead` of the one being read.
    fn kind(&self, ahead: usize) -> Option<TokenKind> {
        self.peek(ahead).map(|token| token.kind)
    }

    /// The token `ahead`, if it can be a DDE topic or item: a word, a
    /// quoted name or a number with no `(` after it.
    fn operand(&self, ahead: usize) -> Option<Token> {
        let called = self.kind(ahead.saturating_add(1)) == Some(TokenKind::OpenParen);
        self.peek(ahead).filter(|token| {
            !called
                && matches!(
                    token.kind,
                    TokenKind::Word | TokenKind::SheetName | TokenKind::Number
                )
        })
    }

    /// A word or number as written, a sheet name unquoted.
    fn value(&self, token: Token) -> String {
        match token.kind {
            TokenKind::SheetName => unquote(self.text(token), '\''),
            _ => self.text(token).to_owned(),
        }
    }

    /// The text of `token`.
    fn text(&self, token: Token) -> &'a str {
        self.text
            .get(token.span.start..token.span.end)
            .unwrap_or("")
    }
}
