//! The sheets a defined name's formula names, rewritten when a sheet is
//! renamed or deleted.
//!
//! A formula is read with `xlsx_graft_formula::analyze`, which gives the
//! span of each sheet prefix (`Sheet1!`, `'S 1'!`, `Sheet1:Sheet3!`). A prefix
//! with no external book is replaced, and a formula the reader cannot read
//! whole, or one with no prefix to replace, is left as it is.
//!
//! - A rename writes the new name in place of the old one at either end of
//!   a prefix, quoted as [`sheet_prefix`] says.
//! - A delete writes `#REF!` in place of a prefix that names only the
//!   deleted sheet, and moves the end of a 3-D prefix that names it one
//!   sheet inward, in the order the sheets have before the delete: the end
//!   at the lower position moves to the next sheet, and the end at the
//!   higher position to the previous one. Which end is the lower is by
//!   position, so a prefix written the other way round (`Sheet3:Sheet1`)
//!   keeps its direction. A 3-D prefix left with one sheet is that sheet's
//!   prefix, and one left with none is `#REF!`.

use super::state::{NameEntry, State, Text, escaped};
use crate::structure::sheet_prefix;
use xlsx_graft_formula::{Formula, SheetPrefix, Span, analyze};

/// What happened to a sheet.
pub(super) enum Change<'a> {
    /// The sheet `old` is now named `new`.
    Rename {
        /// The old name.
        old: &'a str,
        /// The new name.
        new: &'a str,
    },
    /// The sheet at `position` of `order` (the names of the sheets in the
    /// order they had before the delete) is deleted.
    Delete {
        /// The deleted sheet's position in `order`.
        position: usize,
        /// The sheets' names, before the delete.
        order: &'a [String],
    },
}

impl State<'_> {
    /// Rewrites each defined name's formula for `change`, setting
    /// `ref_written` when a `#REF!` is written.
    pub(super) fn rewrite_formulas(&mut self, change: &Change<'_>) {
        let mut ref_written = false;
        for entry in &mut self.names {
            ref_written |= rewrite(entry, change);
        }
        self.ref_written |= ref_written;
    }
}

/// Rewrites `entry`'s formula for `change`. Whether it wrote a `#REF!`.
fn rewrite(entry: &mut NameEntry, change: &Change<'_>) -> bool {
    let formula = read(&entry.text);
    if !formula.facts.malformed.is_empty() {
        return false;
    }
    let mut replacements: Vec<(Span, String)> = (formula.facts.sheet_prefixes.iter())
        .filter(|prefix| prefix.book.is_none())
        .filter(|prefix| {
            prefix
                .last
                .as_deref()
                .is_none_or(|last| !last.contains(':'))
        })
        .filter_map(|prefix| Some((prefix.span, replacement(prefix, change)?)))
        .collect();
    if replacements.is_empty() {
        return false;
    }
    replacements.sort_by_key(|(span, _)| span.start);
    let mut text = String::new();
    let mut at = 0;
    let mut ref_written = false;
    for (span, new) in &replacements {
        text.push_str(formula.text.get(at..span.start).unwrap_or_default());
        text.push_str(new);
        ref_written |= new == "#REF!";
        at = span.end;
    }
    text.push_str(formula.text.get(at..).unwrap_or_default());
    entry.text = Text::Changed(text);
    ref_written
}

/// `text` read, its escapes undone once: what is changed is already undone,
/// so it is escaped again to be read the way the workbook's own text is.
fn read(text: &Text) -> Formula {
    match text {
        Text::Original(text) => analyze(text),
        Text::Changed(text) => analyze(&escaped(text)),
    }
}

/// What replaces `prefix` (with its `!`), `None` to leave it.
fn replacement(prefix: &SheetPrefix, change: &Change<'_>) -> Option<String> {
    match change {
        Change::Rename { old, new } => renamed(prefix, old, new),
        Change::Delete { position, order } => deleted(prefix, *position, order),
    }
}

/// The prefix for the sheet `old` renamed to `new`.
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

/// The prefix for the sheet at `position` of `order` deleted.
fn deleted(prefix: &SheetPrefix, position: usize, order: &[String]) -> Option<String> {
    let at = |name: &str| {
        let folded = name.to_lowercase();
        order
            .iter()
            .position(|sheet| sheet.to_lowercase() == folded)
    };
    let first = at(&prefix.first)?;
    let Some(last) = prefix.last.as_deref() else {
        return (first == position).then(|| "#REF!".to_owned());
    };
    let last = at(last)?;
    let (low, high) = (first.min(last), first.max(last));
    let (new_low, new_high) = if low == high && low == position {
        return Some("#REF!".to_owned());
    } else if low == position {
        (low.checked_add(1)?, high)
    } else if high == position {
        (low, high.checked_sub(1)?)
    } else {
        return None;
    };
    let (new_first, new_last) = if first <= last {
        (new_low, new_high)
    } else {
        (new_high, new_low)
    };
    let name = |index: usize| order.get(index).map(String::as_str);
    let last = if new_low == new_high {
        None
    } else {
        Some(name(new_last)?)
    };
    Some(format!("{}!", sheet_prefix(name(new_first)?, last)))
}
