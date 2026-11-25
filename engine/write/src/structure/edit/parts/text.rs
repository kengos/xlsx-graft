//! [`Context`]: what a rename or a delete does to the text of one formula
//! of a part other than the workbook part, and the functions the walk uses
//! to read an attribute, undo `_xHHHH_` escapes and resolve a reference.
//!
//! The changes are those the edit's calls made, applied in their order to
//! the formula as `formula.rs` applies them to a defined name's: a formula
//! for which `xlsx_graft_formula::analyze` reports a malformed place is not
//! rewritten, and is searched for the names of the sheets changed, as text,
//! to be reported.
//!
//! A formula is not tried against every change. A change is about one sheet
//! name, so `Context` lists, for each name in lower case, the changes about
//! it, and a formula is tried only against the changes about the names its
//! sheet prefixes have: the time for a formula is the number of its prefixes
//! (and of the changes that name them), not the number of changes.

use super::super::change::{Change, SheetIds};
use super::super::formula::rewrite_text;
use super::super::state::escaped;
use crate::structure::report::MAX_AT_BYTES;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use xlsx_graft_cell::unescape;
use xlsx_graft_formula::{Formula, ReferenceKind, analyze};
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::{BytesRef, BytesStart};

/// At most this many bytes of a formula are read: the bytes of its text, of
/// the comments and of the processing instructions inside it, added up. A
/// longer one is reported and not rewritten. It has the value of
/// `max_formula_bytes` of `xlsx-graft-verify`'s default limits, which this
/// crate cannot depend on; verify counts only the text.
pub(super) const MAX_FORMULA_BYTES: usize = 64 * 1024;

/// What the `at` of a defined name starts with.
const DEFINED_NAME: &str = "definedName ";

/// The changes and the removed tables a walk applies.
pub(super) struct Context {
    /// The renames and deletes, in order.
    changes: Vec<Change>,
    /// The numbers of the names of the sheets, which a delete is read with.
    ids: SheetIds,
    /// The sheet of each change in lower case, with the indexes in `changes`
    /// of the changes about it, in ascending order.
    concerned: BTreeMap<String, Vec<usize>>,
    /// The sheets of the changes, one for each name in lower case, in the
    /// order a change first has it, with the name as the change gives it.
    names: Vec<(String, Arc<str>)>,
    /// The names and display names of the tables whose parts are removed, in
    /// lower case, with the name as the table's part writes it.
    tables: BTreeMap<String, Arc<str>>,
}

/// A formula with the changes applied.
pub(super) struct Rewritten {
    /// The text, its `_xHHHH_` escapes undone.
    pub(super) text: String,
    /// The sheet of the first change that touched it.
    pub(super) sheet: Arc<str>,
    /// Whether a `#REF!` was written.
    pub(super) ref_written: bool,
}

/// What is found of one formula.
pub(super) struct Judged {
    /// The formula after the changes, `None` when none touched it.
    pub(super) rewritten: Option<Rewritten>,
    /// The sheet whose name a malformed formula holds.
    pub(super) malformed: Option<Arc<str>>,
    /// The sheet of the first change that left a prefix of its sheet as
    /// written, because it has no operand (`SheetPrefix::operand` is `None`).
    pub(super) unresolved: Option<Arc<str>>,
    /// The tables removed that it names, as their parts write them.
    pub(super) tables: Vec<Arc<str>>,
}

impl Context {
    /// A context of `changes`, whose delete changes `ids` numbers the names
    /// of, and of the `tables` removed (their names in lower case, each
    /// with the name as its part writes it).
    pub(super) fn new(
        changes: Vec<Change>,
        ids: SheetIds,
        tables: BTreeMap<String, Arc<str>>,
    ) -> Self {
        let mut concerned: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut names = Vec::new();
        for (index, change) in changes.iter().enumerate() {
            let sheet = change.sheet();
            let folded = sheet.to_lowercase();
            let indexes = concerned.entry(folded.clone()).or_default();
            // An empty name would be found in every text.
            if indexes.is_empty() && !sheet.is_empty() {
                names.push((folded, Arc::clone(sheet)));
            }
            indexes.push(index);
        }
        Self {
            changes,
            ids,
            concerned,
            names,
            tables,
        }
    }

    /// How many changes there are.
    pub(super) fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// The first change from the index `from` on that is about the sheet
    /// `name`, letter case aside, with its index.
    pub(super) fn change_about(&self, name: &str, from: usize) -> Option<(usize, &Change)> {
        let indexes = self.concerned.get(&name.to_lowercase())?;
        let index = *indexes.get(indexes.partition_point(|&index| index < from))?;
        self.changes.get(index).map(|change| (index, change))
    }

    /// Reads `raw`, the text of a formula with its `_xHHHH_` escapes still
    /// in it, and applies the changes.
    pub(super) fn judge(&self, raw: &str) -> Judged {
        self.judge_formula(analyze(raw))
    }

    /// As [`judge`](Self::judge), for a formula already read.
    pub(super) fn judge_formula(&self, formula: Formula) -> Judged {
        let (malformed, tables) = self.inspect(&formula);
        let mut current = formula;
        let mut rewritten: Option<Rewritten> = None;
        let mut unresolved: Option<Arc<str>> = None;
        let mut cursor = 0;
        // Each change is applied at most once, so the loop ends in as many
        // rounds as there are changes.
        for _ in 0..self.changes.len() {
            let Some((index, change)) = self.next_change(&current, cursor) else {
                break;
            };
            cursor = index.saturating_add(1);
            let done = rewrite_text(&current, change, &self.ids);
            if done.unresolved && unresolved.is_none() {
                unresolved = Some(Arc::clone(change.sheet()));
            }
            let Some((text, ref_written)) = done.written else {
                continue;
            };
            let before = rewritten.as_ref();
            let ref_written = before.is_some_and(|done| done.ref_written) || ref_written;
            let sheet = before.map_or_else(|| Arc::clone(change.sheet()), |d| Arc::clone(&d.sheet));
            current = analyze(&escaped(&text));
            rewritten = Some(Rewritten {
                text,
                sheet,
                ref_written,
            });
        }
        Judged {
            rewritten,
            malformed,
            unresolved,
            tables,
        }
    }

    /// The first change from the index `from` on that is about a sheet a
    /// prefix of `formula` names (none for a formula that is malformed),
    /// with its index.
    fn next_change(&self, formula: &Formula, from: usize) -> Option<(usize, &Change)> {
        if !formula.facts.malformed.is_empty() {
            return None;
        }
        (formula.facts.sheet_prefixes.iter())
            .filter(|prefix| prefix.book.is_none())
            .flat_map(|prefix| std::iter::once(prefix.first.as_str()).chain(prefix.last.as_deref()))
            .filter_map(|name| self.change_about(name, from))
            .min_by_key(|(index, _)| *index)
    }

    /// The sheet of the first change, in the order of the changes, whose
    /// name `text` holds, letter case aside. It looks for each name as a
    /// substring, so a short name (`A`) is found in any text that holds the
    /// letter, and the reports are more than the references to the sheet.
    /// The time is the length of the text times the number of names, and it
    /// runs only for a formula that is malformed or has an element inside
    /// it.
    pub(super) fn mentioned(&self, text: &str) -> Option<&Arc<str>> {
        let folded = text.to_lowercase();
        (self.names.iter())
            .find(|(name, _)| folded.contains(name.as_str()))
            .map(|(_, name)| name)
    }

    /// The sheet a malformed `formula` mentions (`None` when `analyze`
    /// reports no malformed place in it), and the tables removed that it
    /// names, each once.
    pub(super) fn inspect(&self, formula: &Formula) -> (Option<Arc<str>>, Vec<Arc<str>>) {
        let malformed = (!formula.facts.malformed.is_empty())
            .then(|| self.mentioned(&formula.text).cloned())
            .flatten();
        let mut named: BTreeMap<String, &Arc<str>> = BTreeMap::new();
        let tables = (formula.facts.references.iter()).filter(|r| r.kind == ReferenceKind::Table);
        for reference in tables {
            let folded = reference.target.to_lowercase();
            if let Some(table) = self.tables.get(&folded) {
                named.entry(folded).or_insert(table);
            }
        }
        (malformed, named.into_values().cloned().collect())
    }
}

/// The value of the unprefixed attribute `name` of `start`, XML
/// normalized, when it has one that reads.
pub(super) fn attribute(start: &BytesStart<'_>, name: &str) -> Option<String> {
    (start.attributes().flatten())
        .find(|attribute| attribute.key.as_ref() == name)
        .and_then(|attribute| {
            let value = attribute.normalized_value(XmlVersion::Implicit1_0);
            value.ok().map(|value| value.into_owned())
        })
}

/// `text` with its `_xHHHH_` escapes undone.
pub(super) fn unescaped(text: &str) -> String {
    let mut out = String::new();
    unescape::<Infallible>(text, true, |piece| {
        out.push_str(piece);
        Ok(())
    })
    .unwrap_or_default();
    out
}

/// What the reference `reference` stands for: one of the five predefined
/// entities or a character reference; nothing for another, which
/// `XmlRewriter` refuses.
pub(super) fn resolve(reference: &BytesRef<'_>) -> String {
    const ENTITIES: [(&str, char); 5] = [
        ("lt", '<'),
        ("gt", '>'),
        ("amp", '&'),
        ("apos", '\''),
        ("quot", '"'),
    ];
    let named = ENTITIES.iter().find(|(name, _)| *name == &**reference);
    let c = reference.resolve_char_ref().ok().flatten();
    c.or(named.map(|&(_, c)| c))
        .map(String::from)
        .unwrap_or_default()
}

/// `text` cut at `most` bytes, at a character boundary.
fn cut(text: &str, most: usize) -> &str {
    text.get(..text.floor_char_boundary(most)).unwrap_or(text)
}

/// `text` as an `at` of a report: cut at [`MAX_AT_BYTES`].
pub(super) fn clipped(text: &str) -> Arc<str> {
    Arc::from(cut(text, MAX_AT_BYTES))
}

/// The `at` of the defined name `name`: `definedName <name>`, cut at
/// [`MAX_AT_BYTES`] with the prefix counted.
pub(super) fn defined_at(name: &str) -> Arc<str> {
    let name = cut(name, MAX_AT_BYTES.saturating_sub(DEFINED_NAME.len()));
    Arc::from(format!("{DEFINED_NAME}{name}"))
}

#[cfg(test)]
mod tests;
