//! [`Merges`]: the merged cells of a worksheet, kept up to
//! `maxMergedCells`, and those that overlap one kept before them.
//!
//! The ranges are taken in the order of the sheet: one that overlaps a
//! range taken before it is dropped, as Excel for Mac 16.114.3 drops the
//! second of two that overlap, and one dropped is no reason to drop
//! another. What Excel does with three or more that overlap in a chain was
//! not seen: taking them in order is the engine's own choice, which
//! drops one of every two that overlap, so a sheet with an overlap has at
//! least one dropped, and without the ranges dropped it has none.
//!
//! Each range is checked against those before it in `n log² n`, without
//! looking at every pair: the ranges are split into blocks of 1, 2, 4, …,
//! and each block's ranges, once every range before them is settled, are
//! checked against the block just before it in one sweep down the rows
//! (`batch.rs`). Every two ranges meet in exactly one such sweep: the one
//! at the middle of the smallest block that holds them both.

mod batch;
mod marks;

use super::place::name;
use batch::Batch;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Limit, Location};
use xlsx_graft_read::CellRef;

/// One merged range, its corners within the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Range {
    pub(super) top: u32,
    pub(super) bottom: u32,
    pub(super) left: u16,
    pub(super) right: u16,
}

impl Range {
    /// The range `text` names, `A1:B2` or `A1`, its corners in either
    /// order; `None` for anything else.
    fn parse(text: &str) -> Option<Self> {
        let (first, last) = text.split_once(':').unwrap_or((text, text));
        let (first, last) = (CellRef::parse(first).ok()?, CellRef::parse(last).ok()?);
        Some(Self {
            top: first.row().min(last.row()),
            bottom: first.row().max(last.row()),
            left: first.column().min(last.column()),
            right: first.column().max(last.column()),
        })
    }

    /// Its A1 reference, as it reads.
    fn reference(&self) -> String {
        format!(
            "{}:{}",
            name(self.left, self.top),
            name(self.right, self.bottom)
        )
    }
}

/// The merged cells of a worksheet.
#[derive(Debug)]
pub(super) struct Merges {
    /// The ranges kept, in the order they come.
    kept: Vec<Range>,
    /// How many `mergeCell` elements came.
    count: u64,
    /// The most kept.
    limit: Limit,
}

impl Merges {
    /// No merged cells yet, `limit` of them to keep.
    pub(super) fn new(limit: Limit) -> Self {
        Self {
            kept: Vec::new(),
            count: 0,
            limit,
        }
    }

    /// Adds the merged cell whose `ref` is `text`: kept while fewer than
    /// the limit came, if it names a range.
    pub(super) fn add(&mut self, text: Option<&str>) {
        self.count = self.count.saturating_add(1);
        if self.count > self.limit.value {
            return;
        }
        self.kept.extend(text.and_then(Range::parse));
    }

    /// What is wrong with the merged cells: more than the limit, and each
    /// range dropped, as the module says, with a range taken before it
    /// that it overlaps, in the order of the sheet, pushed into
    /// `diagnostics` at `location`.
    pub(super) fn finish(self, diagnostics: &mut Diagnostics, location: &Location) {
        if self.count > self.limit.value {
            let too_many = Diagnostic::MergedCellsTooMany {
                count: self.count,
                limit: self.limit,
            };
            diagnostics.push(too_many, location.clone());
        }
        let dropped = dropped(&self.kept);
        for (range, other) in self.kept.iter().zip(dropped) {
            if let Some(other) = other.and_then(|other| self.kept.get(other)) {
                let overlap = Diagnostic::MergedCellsOverlap {
                    reference: Echo::new(&range.reference()),
                    other: Echo::new(&other.reference()),
                };
                diagnostics.push(overlap, location.clone());
            }
        }
    }
}

/// For each of `ranges`, in the order of the sheet, the index of a range
/// taken before it that it overlaps, when it is dropped.
pub(super) fn dropped(ranges: &[Range]) -> Vec<Option<usize>> {
    let mut batch = Batch::new(ranges);
    let n = ranges.len();
    for middle in 1..n {
        // The block that ends at `middle`, and the one as large after it.
        let size = middle.isolate_lowest_one();
        let after = middle.saturating_add(size).min(n);
        batch.check(middle.saturating_sub(size)..middle, middle..after);
    }
    batch.into_dropped()
}
