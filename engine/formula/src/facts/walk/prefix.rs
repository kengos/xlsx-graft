//! What a quoted name, brackets or `|` start: a sheet, an external book
//! (`[1]`, or the path in `'C:\d\[b.xlsx]S'`) or a DDE link
//! (`app|topic!item`).

use super::Walker;
use crate::facts::word::unquote;
use crate::{
    DdeLink, ExternalBook, Location, Malformed, MalformedReason, ReferenceKind, Span, Token,
    TokenKind,
};

impl Walker<'_> {
    /// A quoted sheet name: before `!` a sheet, or a book and a sheet;
    /// before `|` a DDE application; otherwise malformed.
    pub(super) fn sheet_name(&mut self, token: Token, location: Location) -> usize {
        match self.kind(1) {
            Some(TokenKind::Pipe) => self.dde(Some(token), 1),
            Some(TokenKind::Bang) => {
                let name = unquote(self.text(token), '\'');
                self.pending = match name.rsplit_once(']') {
                    Some((path, sheet)) => {
                        let book = path
                            .rsplit_once('[')
                            .map_or_else(|| path.to_owned(), |(dir, file)| format!("{dir}{file}"));
                        self.external(&book, token.span);
                        Location {
                            book: Some(book),
                            sheet: Some(sheet.to_owned()),
                        }
                    }
                    // No sheet name holds these: a path to a book.
                    None if name.contains(['\\', '/', '?', '*', '[']) => {
                        self.external(&name, token.span);
                        Location {
                            book: Some(name),
                            sheet: None,
                        }
                    }
                    None => Location {
                        book: location.book,
                        sheet: Some(name),
                    },
                };
                2
            }
            _ => {
                self.facts.malformed.push(Malformed {
                    reason: MalformedReason::StraySheetName,
                    span: token.span,
                });
                1
            }
        }
    }

    /// Brackets: an external book when `!`, or a sheet (or two, joined by
    /// `:`) and `!`, follow them; otherwise a structured reference inside
    /// its own table.
    pub(super) fn bracket(&mut self, token: Token, location: Location) -> usize {
        let text = self.text(token);
        let book = text
            .strip_prefix('[')
            .and_then(|t| t.strip_suffix(']'))
            .unwrap_or(text);
        let next = self
            .peek(1)
            .filter(|next| next.span.start == token.span.end)
            .map(|next| next.kind);
        let three_d =
            self.kind(2) == Some(TokenKind::Colon) && self.kind(4) == Some(TokenKind::Bang);
        let sheet_next = matches!(next, Some(TokenKind::Word | TokenKind::SheetName))
            && (self.kind(2) == Some(TokenKind::Bang) || three_d);
        if next != Some(TokenKind::Bang) && !sheet_next {
            self.reference(ReferenceKind::Table, location, "", token.span.start, token);
            return 1;
        }
        self.external(book, token.span);
        self.pending = Location {
            book: Some(book.to_owned()),
            sheet: None,
        };
        if self.kind(1) == Some(TokenKind::Bang) {
            2
        } else {
            1
        }
    }

    /// A DDE link: `app`, the `|` `pipe` tokens ahead, then a topic, and
    /// `!` and an item, if they are there.
    pub(super) fn dde(&mut self, app: Option<Token>, pipe: usize) -> usize {
        let start = app.or(self.peek(pipe)).map_or(0, |token| token.span.start);
        let mut end = self.peek(pipe).map_or(start, |token| token.span.end);
        let mut used = pipe.saturating_add(1);
        let mut topic = String::new();
        if let Some(token) = self.operand(used) {
            topic = self.value(token);
            end = token.span.end;
            used = used.saturating_add(1);
        }
        let mut item = None;
        if let Some(token) = self
            .operand(used.saturating_add(1))
            .filter(|_| self.kind(used) == Some(TokenKind::Bang))
        {
            item = Some(self.value(token));
            end = token.span.end;
            used = used.saturating_add(2);
        }
        self.facts.dde_links.push(DdeLink {
            app: app.map_or_else(String::new, |token| self.value(token)),
            topic,
            item,
            span: Span { start, end },
        });
        used
    }

    /// The external book `book`, named at `span`.
    fn external(&mut self, book: &str, span: Span) {
        self.facts.external_books.push(ExternalBook {
            book: book.to_owned(),
            span,
        });
    }
}
