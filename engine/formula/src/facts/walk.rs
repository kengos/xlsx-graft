//! Reads the facts from a formula's tokens, left to right, whitespace left
//! out. A sheet or book prefix (`Sheet1!`, `[1]`, `'C:\d\[b.xlsx]S'!`) is
//! held for the token after it; nothing is held across a `:`, and the cell
//! that ends a range looks back for the location of the one that starts it.

mod prefix;

use super::word::{is_cell, is_column, is_param, is_row, split_prefix, unquote};
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
    facts: &'a mut Facts,
}

impl<'a> Walker<'a> {
    /// Reads every token, each handler saying how many it used.
    fn run(&mut self) {
        while let Some(token) = self.peek(0) {
            let location = std::mem::take(&mut self.pending);
            let used = match token.kind {
                TokenKind::Word => self.word(token, location),
                TokenKind::Number => self.range(token, location).unwrap_or(1),
                TokenKind::SheetName => self.sheet_name(token, location),
                TokenKind::Bracket => self.bracket(token, location),
                TokenKind::Pipe => self.dde(None, 0),
                _ => 1,
            };
            self.at = self.at.saturating_add(used);
        }
    }

    /// A word: a DDE application, a sheet (alone or the first of two), a
    /// call, a range of columns or rows, a table, or one word alone.
    fn word(&mut self, token: Token, location: Location) -> usize {
        let text = self.text(token);
        match self.kind(1) {
            Some(TokenKind::Pipe) => return self.dde(Some(token), 1),
            Some(TokenKind::Bang) => {
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
        if let Some(last) = self
            .peek(2)
            .filter(|_| self.kind(1) == Some(TokenKind::Colon))
            && last.kind == TokenKind::Word
            && self.kind(3) == Some(TokenKind::Bang)
        {
            self.pending = Location {
                book: location.book,
                sheet: Some(format!("{text}:{}", self.text(last))),
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
            let location = self.range_start().unwrap_or(location);
            self.reference(ReferenceKind::Cell, location, text, token.span.start, token);
        } else if !boolean {
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
        self.facts.calls.push(Call {
            name: base.to_ascii_uppercase(),
            prefix: prefix.to_owned(),
            location,
            passed,
            span: token.span,
        });
    }

    /// Columns (`A:C`) or rows (`1:3`), when `first` starts either; the
    /// number of tokens they take. Not when a `(` follows: `A:T(1)` calls
    /// `T`.
    fn range(&mut self, first: Token, location: Location) -> Option<usize> {
        let last = self.peek(2).filter(|_| {
            self.kind(1) == Some(TokenKind::Colon) && self.kind(3) != Some(TokenKind::OpenParen)
        })?;
        let (from, to) = (self.text(first), self.text(last));
        let kind = if is_column(from) && is_column(to) {
            ReferenceKind::Columns
        } else if is_row(from) && is_row(to) {
            ReferenceKind::Rows
        } else {
            return None;
        };
        self.reference(
            kind,
            location,
            &format!("{from}:{to}"),
            first.span.start,
            last,
        );
        Some(3)
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
        self.facts.references.push(Reference {
            kind,
            location,
            target: target.to_owned(),
            span: Span {
                start,
                end: last.span.end,
            },
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

    /// The location of the cell that starts the range the cell being read
    /// ends, when that one came right after its prefix: `Sheet2` for `B2`
    /// in `Sheet2!A1:B2`. Looked up from the end, so only a cell gets it,
    /// and a sheet name is copied twice at most, however long the chain.
    fn range_start(&self) -> Option<Location> {
        let colon = self.behind(1).filter(|t| t.kind == TokenKind::Colon)?;
        self.behind(3).filter(|t| t.kind == TokenKind::Bang)?;
        let start = self.facts.references.last()?;
        (start.kind == ReferenceKind::Cell && start.span.end == colon.span.start)
            .then(|| start.location.clone())
    }

    /// The token `back` before the one being read.
    fn behind(&self, back: usize) -> Option<Token> {
        let at = self.at.checked_sub(back)?;
        self.tokens.get(at).copied()
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
