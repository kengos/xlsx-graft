//! Gives each sheet prefix the operand it qualifies, as the walker reads it:
//! the handlers of the tokens after a prefix call [`Walker::qualify`], and a
//! call or a `#` after the operand moves the end the prefix has.

use super::Walker;
use crate::{Span, Token, TokenKind};

impl Walker<'_> {
    /// `span` is the operand of `prefix`, the prefix read just before the
    /// token being read, when there is one.
    pub(super) fn qualify(&mut self, span: Span) {
        if let Some(index) = self.prefix {
            self.set_operand(index, span);
        }
    }

    /// `span` is the operand of the prefix `index`, which a `#` right after
    /// it may extend.
    fn set_operand(&mut self, index: usize, span: Span) {
        let prefix = self.facts.sheet_prefixes.get_mut(index);
        prefix
            .into_iter()
            .for_each(|prefix| prefix.operand = Some(span));
        self.ended = Some(index);
    }

    /// The call `token` starts, with a `(` after it, is the operand of the
    /// prefix before it, up to the `)` that closes that `(`.
    pub(super) fn callee(&mut self, token: Token) {
        self.callee = self.prefix.map(|index| (index, token.span.start));
    }

    /// A `(` opens an operand in two cases: it follows the name of a call
    /// that is the operand of a prefix (`callee`), or it follows, with no
    /// whitespace between them, the `)` that ended the operand of a prefix
    /// (`ended`), as in `Sheet1!MyFn(1)(2)`, where `(2)` calls the value
    /// `MyFn(1)` returns. The operand then ends with the `)` that closes the
    /// `(`. Any other `(` opens no operand.
    pub(super) fn open_paren(&mut self, token: Token, ended: Option<usize>) -> usize {
        let callee = self.callee.take().or_else(|| {
            let closed = self.behind(1).map(|before| before.kind) == Some(TokenKind::CloseParen);
            let index = ended.filter(|_| closed)?;
            let operand = self.facts.sheet_prefixes.get(index)?.operand?;
            (operand.end == token.span.start).then_some((index, operand.start))
        });
        self.open.push(callee);
        1
    }

    /// A `)` closes the `(` opened last that is still open, and ends the
    /// operand that `(` opened, if it did. One with no `(` closes nothing.
    pub(super) fn close_paren(&mut self, token: Token) -> usize {
        if let Some(Some((index, start))) = self.open.pop() {
            let span = Span {
                start,
                end: token.span.end,
            };
            self.set_operand(index, span);
        }
        1
    }

    /// An error value is an operand, if a prefix is before it.
    pub(super) fn error_value(&mut self, token: Token) -> usize {
        self.qualify(token.span);
        1
    }

    /// An operator: a `#` right after an operand that ended with the token
    /// before this one (`ended`) is the spill operator, and is part of that
    /// operand, and of the operand of the previous prefix when that operand
    /// ends at the same token (`Sheet1!A1:Sheet1!B2#`, whose first operand
    /// holds the second prefix). `run` takes `ended` before each token, so a
    /// `#` extends only an operand that the token right before it ended: in
    /// `Sheet1!A2##` the operand is `A2#`, and the second `#` is not part of
    /// it (`spill` moves the end but does not set `ended`).
    pub(super) fn spill(&mut self, token: Token, ended: Option<usize>) -> usize {
        if self.text(token) != "#" {
            return 1;
        }
        let previous = ended.and_then(|index| index.checked_sub(1));
        for index in ended.into_iter().chain(previous) {
            let operand = (self.facts.sheet_prefixes.get_mut(index))
                .and_then(|prefix| prefix.operand.as_mut())
                .filter(|operand| operand.end == token.span.start);
            if let Some(operand) = operand {
                operand.end = token.span.end;
            }
        }
        1
    }
}
