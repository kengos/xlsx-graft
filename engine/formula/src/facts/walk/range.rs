//! Ranges: columns (`A:C`) or rows (`1:3`), and the cell that ends a range
//! started by a cell right after a prefix (`Sheet1!A1:B2`), whose location
//! and operand it takes from that cell.

use super::Walker;
use crate::facts::word::{is_column, is_row};
use crate::{Location, ReferenceKind, Token, TokenKind};

impl Walker<'_> {
    /// Columns (`A:C`) or rows (`1:3`), when `first` starts either; the
    /// number of tokens they take. Not when a `(` follows: `A:T(1)` calls
    /// `T`.
    pub(super) fn range(&mut self, first: Token, location: Location) -> Option<usize> {
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

    /// The location of the cell that starts the range the cell being read
    /// ends, when that one came right after its prefix: `Sheet2` for `B2`
    /// in `Sheet2!A1:B2`. Looked up from the end, so only a cell gets it,
    /// and a sheet name is copied twice at most, however long the chain.
    pub(super) fn range_start(&self) -> Option<Location> {
        let colon = self.behind(1).filter(|t| t.kind == TokenKind::Colon)?;
        self.behind(3).filter(|t| t.kind == TokenKind::Bang)?;
        let start = self.facts.references.last()?;
        (start.kind == ReferenceKind::Cell && start.span.end == colon.span.start)
            .then(|| start.location.clone())
    }

    /// The token `back` before the one being read.
    pub(super) fn behind(&self, back: usize) -> Option<Token> {
        let at = self.at.checked_sub(back)?;
        self.tokens.get(at).copied()
    }

    /// The cell being read ends a range that starts with the cell right
    /// after a prefix (`Sheet1!A1:B2`): the operand of that prefix, the last
    /// one read, ends with this cell.
    pub(super) fn extend_range(&mut self, end: usize) {
        let first = self.behind(2).map(|token| token.span.end);
        let index = self.facts.sheet_prefixes.len().saturating_sub(1);
        if let Some(operand) = self
            .facts
            .sheet_prefixes
            .get_mut(index)
            .and_then(|prefix| prefix.operand.as_mut())
            .filter(|operand| Some(operand.end) == first)
        {
            operand.end = end;
            self.ended = Some(index);
        }
    }
}
