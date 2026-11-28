//! The merged ranges of a `SheetEdit`: the `mergeCell`s the part has, those
//! `unmerge` removed and those `merge` added, checked as each call is made.
//! `merges/stream.rs` writes them.

mod stream;

use crate::sheet::{CellRange, MergedCells, SheetError};
use std::io::{self, Write};

/// A `mergeCell` of the part.
#[derive(Debug)]
pub(super) struct Entry {
    /// Its `ref`; `None` when it cannot be read as a range, so it is not
    /// compared in the overlap check and not counted toward the 262,144
    /// ranges that `merge` checks (it is counted in the `count` written).
    pub(super) range: Option<CellRange>,
    /// Whether `unmerge` removed it.
    removed: bool,
}

impl Entry {
    pub(super) fn new(range: Option<CellRange>) -> Self {
        Self {
            range,
            removed: false,
        }
    }
}

/// The merged ranges of the sheet and what the calls did to them.
#[derive(Debug)]
pub(super) struct Merges {
    /// The `mergeCell`s of the part, in order. A part with more than one
    /// `mergeCells` cannot be written when a range is merged or unmerged
    /// (`SheetEdit::apply` refuses it), so the position of a `mergeCell` is
    /// its number among all the `mergeCell`s of the part.
    entries: Vec<Entry>,
    /// The ranges `merge` added, in order.
    added: Vec<CellRange>,
    /// The `ref` of each table of the sheet.
    tables: Vec<CellRange>,
    /// Whether a call changed the ranges.
    changed: bool,
}

impl Merges {
    pub(super) fn new(entries: Vec<Entry>, tables: Vec<CellRange>) -> Self {
        Self {
            entries,
            added: Vec::new(),
            tables,
            changed: false,
        }
    }

    pub(super) fn changed(&self) -> bool {
        self.changed
    }

    /// The ranges the sheet holds now: the part's that were not removed,
    /// then those added.
    fn current(&self) -> impl Iterator<Item = CellRange> + '_ {
        let kept = self.entries.iter().filter(|entry| !entry.removed);
        kept.filter_map(|entry| entry.range)
            .chain(self.added.iter().copied())
    }

    /// Merges `range`, as `SheetEdit::merge` says.
    pub(super) fn merge(&mut self, range: CellRange) -> Result<(), SheetError> {
        if range.is_single_cell() {
            return Err(SheetError::MergeSingleCell { range });
        }
        let count = self.current().count().saturating_add(1);
        if count > MergedCells::LIMIT {
            return Err(SheetError::TooManyMerges {
                count,
                limit: MergedCells::LIMIT,
            });
        }
        if let Some(other) = self.current().find(|other| other.overlaps(range)) {
            return Err(SheetError::MergeOverlap { range, other });
        }
        if let Some(&table) = self.tables.iter().find(|table| table.overlaps(range)) {
            return Err(SheetError::MergeTable { range, table });
        }
        self.added.push(range);
        self.changed = true;
        Ok(())
    }

    /// Removes every range equal to `range`, as `SheetEdit::unmerge` says.
    pub(super) fn unmerge(&mut self, range: CellRange) -> Result<(), SheetError> {
        if !self.current().any(|current| current == range) {
            return Err(SheetError::MergeMissing { range });
        }
        for entry in self.entries.iter_mut() {
            entry.removed |= entry.range == Some(range);
        }
        self.added.retain(|added| *added != range);
        self.changed = true;
        Ok(())
    }

    /// Writes a new `mergeCells` of the ranges added, for a part that has
    /// none.
    pub(super) fn write_new(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        // `merge` kept `added` to what `MergedCells::new` checks (no range of one
        // cell, none that share a cell, no more than the sheet holds), so `new`
        // returns `Ok`; an `Err` would write nothing.
        let added = MergedCells::new(self.added.clone());
        added
            .into_iter()
            .try_for_each(|ranges| ranges.write(out, prefix))
    }
}
