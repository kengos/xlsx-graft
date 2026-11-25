//! Notes each prefix that names a sheet, as the walker reads it.

use super::Walker;

impl Walker<'_> {
    /// Notes the prefix of `first` and `last` (two sheets of a 3-D
    /// reference), in `book` if any, that starts at `start` and ends with
    /// the `!` `bang` tokens ahead.
    pub(super) fn sheet_prefix(
        &mut self,
        start: usize,
        bang: usize,
        book: Option<String>,
        sheets: (&str, Option<&str>),
    ) {
        let end = self.peek(bang).map_or(start, |token| token.span.end);
        // The token after the `!` gives the operand, if it has one.
        self.next_prefix = Some(self.facts.sheet_prefixes.len());
        self.facts.sheet_prefixes.push(crate::SheetPrefix {
            span: crate::Span { start, end },
            book,
            first: sheets.0.to_owned(),
            last: sheets.1.map(str::to_owned),
            operand: None,
        });
    }
}
