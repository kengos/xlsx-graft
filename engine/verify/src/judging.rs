//! [`Judging`]: the [`FormulaSink`] [`verify`](crate::verify) reads the
//! parts into, which hands each formula to the [`FormulaJudge`] at its
//! [`Place`].
//!
//! A formula's sheet is the sheet whose part holds it ([`Book::sheet`]); a
//! defined name's own formula is judged at its `localSheetId`, wherever it
//! is. A `macro` attribute, which names the macro a control or a drawing
//! runs, is a formula like any other, as SpreadsheetML types it: a name
//! that is no function the table knows is denied, and one in another
//! workbook (`[1]!Run`) is an external link. So is one after `[0]`, which
//! may stand for this workbook: whether Excel writes it so is not yet
//! confirmed, and the judge reads every workbook number as another
//! workbook, erring towards refusing.
//!
//! A formula whose text was not kept is refused unread: one longer than
//! the limit (`formula_too_long`), and one with an element inside it
//! (`formula_malformed`, `markup`). Each, like the judge's findings, is
//! one diagnostic per part, at the first place found, counting every
//! time; one too long gives the size of the longest, which the limit has
//! to reach for the part to be read.

use crate::book::{Book, local_sheet_id};
use crate::collect::{FormulaSink, TableTag};
use crate::{FormulaJudge, FunctionTable, Place, Rules, Targets};
use std::collections::BTreeMap;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, FormulaMalformedReason, Limit, Location};

/// Judges the formulas a workbook's parts are read into.
pub(crate) struct Judging<'a> {
    judge: FormulaJudge<'a>,
    book: &'a Book,
    /// The formula limit, which a formula too long was held to.
    limit: Limit,
    /// The formulas refused unread, each part and way once, in the order
    /// first found.
    unread: Vec<Unread>,
    /// The index into `unread` of each part and way, `true` for too long.
    seen: BTreeMap<(String, bool), usize>,
}

/// The formulas of one part refused unread one way.
struct Unread {
    /// Where the first was.
    location: Location,
    /// The size of the longest, for one too long; `None` for markup.
    size: Option<u64>,
    /// How many there were.
    count: u64,
}

impl<'a> Judging<'a> {
    /// A sink judging by `rules` and `functions`, checking the functions
    /// against `targets`, with the names and sheets of `book`, of formulas
    /// held to `limit`.
    pub(crate) fn new(
        rules: &'a Rules,
        functions: &'a dyn FunctionTable,
        targets: &'a Targets,
        book: &'a Book,
        limit: Limit,
    ) -> Self {
        Self {
            judge: FormulaJudge::new(rules, functions, targets, &book.names),
            book,
            limit,
            unread: Vec::new(),
            seen: BTreeMap::new(),
        }
    }

    /// Pushes a diagnostic for each finding into `diagnostics`: the
    /// judge's, then the formulas refused unread.
    pub(crate) fn finish(self, diagnostics: &mut Diagnostics) {
        self.judge.finish(diagnostics);
        for unread in self.unread {
            let count = unread.count;
            let diagnostic = match unread.size {
                Some(size) => Diagnostic::FormulaTooLong {
                    size,
                    limit: self.limit,
                    count,
                },
                None => Diagnostic::FormulaMalformed {
                    reason: FormulaMalformedReason::Markup,
                    count,
                },
            };
            diagnostics.push(diagnostic, unread.location);
        }
    }

    /// Notes a formula at `at` in `part` refused unread: too long, of
    /// `size` bytes, or with markup when `size` is `None`.
    fn refuse(&mut self, part: &str, at: &str, size: Option<u64>) {
        let key = (part.to_owned(), size.is_some());
        if let Some(unread) = (self.seen.get(&key)).and_then(|&index| self.unread.get_mut(index)) {
            unread.count = unread.count.saturating_add(1);
            unread.size = unread.size.max(size);
            return;
        }
        self.seen.insert(key, self.unread.len());
        self.unread.push(Unread {
            location: Location::default().with_part(part).with_place(at),
            size,
            count: 1,
        });
    }
}

impl FormulaSink for Judging<'_> {
    fn formula(&mut self, part: &str, at: &str, text: &str) {
        let sheet = self.book.sheet(part);
        self.judge.judge(&Place { part, at, sheet }, text);
    }

    fn too_long(&mut self, part: &str, at: &str, size: u64) {
        self.refuse(part, at, Some(size));
    }

    fn markup(&mut self, part: &str, at: &str) {
        self.refuse(part, at, None);
    }

    fn defined_name(&mut self, part: &str, at: &str, _: &str, sheet: Option<&str>, text: &str) {
        let sheet = sheet.and_then(local_sheet_id);
        self.judge.judge(&Place { part, at, sheet }, text);
    }

    fn table(&mut self, _part: &str, _table: &TableTag) {}
}
