//! What a walk keeps and reports: the attributes that hold a formula the
//! edit only reports, a pivot cache's `worksheetSource`, and the judgement
//! of one formula (the tables removed it names, the sheet a malformed one
//! holds, and what the changes make of it).

use super::super::pivot::{Source, source};
use super::super::text::{MAX_FORMULA_BYTES, Rewritten, attribute};
use super::Walk;
use crate::structure::KeptReason;
use std::io::{BufRead, Write};
use std::sync::Arc;
use xlsx_graft_formula::places::{FORMULA_ATTRIBUTES, OnSheetChange};
use xlsx_graft_xml::XmlRewriter;
use xlsx_graft_xml::events::BytesStart;

impl Walk<'_> {
    /// Reads the attributes of `start` that hold a formula the edit reports,
    /// the element being `local`.
    pub(super) fn attributes(&mut self, local: &str, start: &BytesStart<'_>) {
        let reported = (FORMULA_ATTRIBUTES.iter())
            .filter(|row| row.element == local && row.on_sheet_change == OnSheetChange::Report);
        for row in reported {
            let Some(value) = attribute(start, row.attribute) else {
                continue;
            };
            let at = self.range.clone();
            if value.len() > MAX_FORMULA_BYTES {
                let reason = KeptReason::TooLong;
                self.keeper.keep(row.place, at.as_ref(), None, None, reason);
            } else if let Some(done) = self.judge(row.place, &at, &value, false) {
                let reason = KeptReason::Outside;
                (self.keeper).keep(row.place, at.as_ref(), Some(&done.sheet), None, reason);
            }
        }
    }

    /// The start tag to write for the `worksheetSource` `start`, when that
    /// is what `local` names and its sheet is renamed. When the sheet is
    /// deleted the source is kept and reported.
    pub(super) fn pivot<R: BufRead, W: Write>(
        &mut self,
        rw: &XmlRewriter<R, W>,
        local: &str,
        start: &BytesStart<'_>,
    ) -> Option<BytesStart<'static>> {
        if local != "worksheetSource" || !self.rules.pivot {
            return None;
        }
        match source(rw, start, self.context) {
            Source::Keep => None,
            Source::Renamed(renamed) => {
                self.changed = true;
                Some(renamed)
            }
            Source::Deleted(sheet) => {
                let reason = KeptReason::DeletedSource;
                let place = "worksheetSource@sheet";
                self.keeper.keep(place, None, Some(&sheet), None, reason);
                None
            }
        }
    }

    /// Reads the formula `value` at `place`, `at` where it is, and keeps the
    /// tables removed it names, when it is malformed the sheet it holds, and
    /// when the place is rewritten (`rewrite`) the sheet of a prefix left as
    /// written because it has no operand. At a place that is not rewritten,
    /// the caller reports a formula that a change rewrote as
    /// [`KeptReason::Outside`], and this method reports a formula that no
    /// change rewrote but that keeps a prefix with no operand as
    /// [`KeptReason::Outside`].
    /// Returns the formula with the changes applied; `None` when no change
    /// touches it.
    pub(super) fn judge(
        &mut self,
        place: &'static str,
        at: &Option<Arc<str>>,
        value: &str,
        rewrite: bool,
    ) -> Option<Rewritten> {
        let judged = self.context.judge(value);
        for table in &judged.tables {
            let reason = KeptReason::DeletedTable;
            self.keeper
                .keep(place, at.as_ref(), None, Some(table), reason);
        }
        if let Some(sheet) = &judged.malformed {
            let reason = KeptReason::Malformed;
            (self.keeper).keep(place, at.as_ref(), Some(sheet), None, reason);
        }
        if let Some(sheet) = &judged.unresolved
            && (rewrite || judged.rewritten.is_none())
        {
            let reason = if rewrite {
                KeptReason::UnknownOperand
            } else {
                KeptReason::Outside
            };
            (self.keeper).keep(place, at.as_ref(), Some(sheet), None, reason);
        }
        judged.rewritten
    }
}
