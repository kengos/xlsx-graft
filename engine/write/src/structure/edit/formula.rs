//! The sheets a defined name's formula names, rewritten when a sheet is
//! renamed or deleted.
//!
//! A formula is read with `xlsx_graft_formula::analyze`, which gives the
//! span of each sheet prefix (`Sheet1!`, `'S 1'!`, `Sheet1:Sheet3!`). A prefix
//! with no external book is replaced, and a formula for which
//! `xlsx_graft_formula::analyze` reports a malformed place, or one with no
//! prefix to replace, is left as it is.
//!
//! - A rename writes the new name in place of the old one at either end of
//!   a prefix, quoted as [`sheet_prefix`] says.
//! - A delete writes one `#REF!` in place of both a prefix that names only
//!   the deleted sheet and the operand that prefix qualifies
//!   (`xlsx_graft_formula::SheetPrefix::operand`: a cell, a range of cells,
//!   columns or rows, a table reference, a name, an error value, or a call up
//!   to its closing `)`, and a spill operator `#` right after it).
//!   `Sheet1!A1:B2` becomes `#REF!`, not `#REF!A1:B2`, and `Sheet1!A2#`
//!   becomes `#REF!`, not `#REF!#`, which Excel repairs. Excel saves
//!   `Sheet1!A2#` as `_xlfn.ANCHORARRAY(Sheet1!A2)`: the operand of
//!   `Sheet1!` is `A2`, the call has no prefix, and a delete writes
//!   `_xlfn.ANCHORARRAY(#REF!)`. The `#` form is written by other
//!   applications. A prefix with no operand (`Sheet1!A1:Sheet1!B2`, which
//!   `analyze` reads as a 3-D prefix `A1:Sheet1!`, a `(`, a call that never
//!   closes, a number) is left as written and reported as
//!   `KeptReason::UnknownOperand`, because it still names the deleted sheet,
//!   and a sheet added later with that name would be the sheet it names.
//! - A delete moves the end of a 3-D prefix that names the sheet one sheet
//!   inward, in the order the sheets have before the delete: the end at the
//!   lower position moves to the next sheet, and the end at the higher
//!   position to the previous one. Which end is the lower is by position, so
//!   a prefix written the other way round (`Sheet3:Sheet1`) keeps its
//!   direction. Each end that is written is spelled as the latest sheet that
//!   had that name spells it (`SheetIds::spelling`). A 3-D prefix left with
//!   one sheet is that sheet's prefix, and one left with none is `#REF!` in
//!   place of the prefix and its operand.

use super::change::{Change, Deleted, SheetIds, Side};
use super::state::{NameEntry, State, Text, escaped};
use crate::structure::sheet_prefix;
use std::sync::Arc;
use xlsx_graft_formula::{Formula, SheetPrefix, Span, analyze};

impl State<'_> {
    /// Rewrites each defined name's formula for `change`, setting
    /// `ref_written` when a `#REF!` is written, and keeps `change` in
    /// [`State::changes`].
    pub(super) fn rewrite_formulas(&mut self, change: Change) {
        let mut ref_written = false;
        for entry in &mut self.names {
            ref_written |= rewrite(entry, &change, &self.sheet_ids);
        }
        self.ref_written |= ref_written;
        self.changes.push(change);
    }
}

/// Rewrites `entry`'s formula for `change`. Whether it wrote a `#REF!`. A
/// prefix left as written because it has no operand sets `entry.unresolved`,
/// to the sheet of the first change that left one.
fn rewrite(entry: &mut NameEntry, change: &Change, ids: &SheetIds) -> bool {
    let done = rewrite_text(&read(&entry.text), change, ids);
    if done.unresolved && entry.unresolved.is_none() {
        entry.unresolved = Some(Arc::clone(change.sheet()));
    }
    let Some((text, ref_written)) = done.written else {
        return false;
    };
    entry.text = Text::Changed(text);
    ref_written
}

/// What [`rewrite_text`] makes of a formula.
pub(super) struct Rewrite {
    /// The text, with its `_xHHHH_` escapes undone as [`Formula::text`] has
    /// it, and whether a `#REF!` was written. `None` when `formula` is
    /// malformed or has no prefix to replace.
    pub(super) written: Option<(String, bool)>,
    /// Whether a prefix of the deleted sheet was left as written, because
    /// it has no operand (`SheetPrefix::operand` is `None`).
    pub(super) unresolved: bool,
}

/// `formula` with the sheet prefixes `change` touches replaced, and the
/// operands of the prefixes a delete replaces with `#REF!`. `ids` numbers
/// the names of the sheets that a delete `change` is about.
pub(super) fn rewrite_text(formula: &Formula, change: &Change, ids: &SheetIds) -> Rewrite {
    let none = Rewrite {
        written: None,
        unresolved: false,
    };
    if !formula.facts.malformed.is_empty() {
        return none;
    }
    let mut found: Vec<(Span, Option<String>)> = (formula.facts.sheet_prefixes.iter())
        .filter(|prefix| prefix.book.is_none())
        .filter(|prefix| {
            prefix
                .last
                .as_deref()
                .is_none_or(|last| !last.contains(':'))
        })
        .filter_map(|prefix| match replacement(prefix, change, ids)? {
            Replacement::Prefix(new) => Some((prefix.span, Some(new))),
            Replacement::Ref => Some(match prefix.operand {
                Some(operand) => {
                    let span = Span {
                        start: prefix.span.start,
                        end: operand.end,
                    };
                    (span, Some("#REF!".to_owned()))
                }
                None => (prefix.span, None),
            }),
        })
        .collect();
    if found.is_empty() {
        return none;
    }
    found.sort_by_key(|(span, _)| span.start);
    let mut text = String::new();
    let mut at = 0;
    let mut ref_written = false;
    let mut replaced = false;
    let mut unresolved = false;
    for (span, new) in &found {
        // A prefix inside an operand already replaced is gone with it.
        if span.start < at {
            continue;
        }
        let Some(new) = new else {
            unresolved = true;
            continue;
        };
        text.push_str(formula.text.get(at..span.start).unwrap_or_default());
        text.push_str(new);
        ref_written |= new == "#REF!";
        replaced = true;
        at = span.end;
    }
    text.push_str(formula.text.get(at..).unwrap_or_default());
    Rewrite {
        written: replaced.then_some((text, ref_written)),
        unresolved,
    }
}

/// `text` read, its escapes undone once: what is changed is already undone,
/// so it is escaped again to be read the way the workbook's own text is.
pub(super) fn read(text: &Text) -> Formula {
    match text {
        Text::Original(text) => analyze(text),
        Text::Changed(text) => analyze(&escaped(text)),
    }
}

/// What replaces a sheet prefix.
enum Replacement {
    /// A prefix, with its `!`, in place of the prefix.
    Prefix(String),
    /// `#REF!` in place of the prefix and the operand it qualifies.
    Ref,
}

/// What replaces `prefix` (with its `!`) for `change`, `None` to leave it.
fn replacement(prefix: &SheetPrefix, change: &Change, ids: &SheetIds) -> Option<Replacement> {
    match change {
        Change::Rename { old, new } => renamed(prefix, old, new).map(Replacement::Prefix),
        Change::Delete(deleted) => self::deleted(prefix, deleted, ids),
    }
}

/// The prefix, with its `!`, for the sheet `old` renamed to `new`; `None` when
/// `prefix` names no sheet that is `old`.
fn renamed(prefix: &SheetPrefix, old: &str, new: &str) -> Option<String> {
    let folded = old.to_lowercase();
    let is_old = |sheet: &str| sheet.to_lowercase() == folded;
    let first_hit = is_old(&prefix.first);
    let last_hit = prefix.last.as_deref().is_some_and(is_old);
    if !first_hit && !last_hit {
        return None;
    }
    let first = if first_hit { new } else { &prefix.first };
    let last = (prefix.last.as_deref()).map(|last| if is_old(last) { new } else { last });
    Some(format!("{}!", sheet_prefix(first, last)))
}

/// What replaces the prefix for the sheet `deleted` deleted.
fn deleted(prefix: &SheetPrefix, deleted: &Deleted, ids: &SheetIds) -> Option<Replacement> {
    let first = deleted.side(ids, &prefix.first);
    let Some(last) = prefix.last.as_deref() else {
        return (first == Side::Deleted).then_some(Replacement::Ref);
    };
    match (first, deleted.side(ids, last)) {
        (Side::Deleted, Side::Deleted) => Some(Replacement::Ref),
        (Side::Deleted, other @ (Side::Before | Side::After)) => {
            moved(deleted, ids, (other, last), true).map(Replacement::Prefix)
        }
        (other @ (Side::Before | Side::After), Side::Deleted) => {
            moved(deleted, ids, (other, &prefix.first), false).map(Replacement::Prefix)
        }
        _ => None,
    }
}

/// The prefix of a 3-D reference with one end at the sheet deleted and the
/// other end `kept` (its side and its name in the formula), the deleted end
/// being the first when `deleted_first`: the deleted end moves to the sheet
/// next to it on the side of the end that stays.
fn moved(
    deleted: &Deleted,
    ids: &SheetIds,
    kept: (Side, &str),
    deleted_first: bool,
) -> Option<String> {
    let (side, name) = kept;
    let beside = match side {
        Side::After => deleted.next,
        _ => deleted.prev,
    };
    let beside_id = beside?;
    let kept_id = ids.get(name)?;
    let beside_name = ids.spelling(beside_id);
    if beside_id == kept_id {
        return Some(format!("{}!", sheet_prefix(beside_name, None)));
    }
    let kept_name = ids.spelling(kept_id);
    let (first, last) = if deleted_first {
        (beside_name, kept_name)
    } else {
        (kept_name, beside_name)
    };
    Some(format!("{}!", sheet_prefix(first, Some(last))))
}
