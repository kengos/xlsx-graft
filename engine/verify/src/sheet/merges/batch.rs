//! [`Batch`]: one block of merged ranges checked against the block before
//! it, in one sweep down the rows, as [`Merges`](super::Merges) says.
//!
//! The ranges of the block before that are taken, none overlapping
//! another, are open from their top row to their bottom one. A range
//! checked overlaps one of them either open on its own top row, which is
//! one of its two neighbours by first column, as those open there share
//! the row and so do not share a column; or opening on a row it spans,
//! which marks its columns with when it opened (`marks.rs`), found as the
//! range checked closes.

use super::Range;
use super::marks::Marks;
use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Unbounded};

/// What happens on a row, in the order it happens there: a range taken
/// closes (on the row after its last), one opens, and then a range checked
/// opens, and closes (on its last row).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    Close,
    Open,
    CheckOpen,
    CheckClose,
}

/// The checks of every block, and what they found.
#[derive(Debug)]
pub(super) struct Batch<'a> {
    ranges: &'a [Range],
    /// For each range, the range taken before it that it overlaps, once it
    /// is found dropped.
    dropped: Vec<Option<usize>>,
    /// For each range checked, when it opened.
    opened: Vec<u64>,
    marks: Marks,
    /// How many steps were taken, in every sweep: when each happened.
    time: u64,
    /// The steps of one sweep: the row, what happens, and to which range.
    steps: Vec<(u32, Step, usize, Range)>,
}

impl<'a> Batch<'a> {
    /// No block checked yet.
    pub(super) fn new(ranges: &'a [Range]) -> Self {
        Self {
            ranges,
            dropped: vec![None; ranges.len()],
            opened: vec![0; ranges.len()],
            marks: Marks::new(ranges.iter().map(|range| range.right).max().unwrap_or(1)),
            time: 0,
            steps: Vec::new(),
        }
    }

    /// Checks the ranges of `checked` not yet dropped against those of
    /// `taken`, every range before them settled, not dropped.
    pub(super) fn check(&mut self, taken: std::ops::Range<usize>, checked: std::ops::Range<usize>) {
        self.steps.clear();
        let ranges = self.ranges;
        let settled = |index: &usize| self.dropped.get(*index).is_some_and(Option::is_none);
        let block = |indices: std::ops::Range<usize>| {
            let indices = indices.filter(settled);
            indices.filter_map(|index| ranges.get(index).map(|range| (index, *range)))
        };
        for (index, range) in block(taken) {
            self.steps.push((range.top, Step::Open, index, range));
            self.steps
                .push((range.bottom.saturating_add(1), Step::Close, index, range));
        }
        for (index, range) in block(checked) {
            self.steps.push((range.top, Step::CheckOpen, index, range));
            self.steps
                .push((range.bottom, Step::CheckClose, index, range));
        }
        self.steps.sort_unstable();
        let mut open: BTreeMap<u16, usize> = BTreeMap::new();
        for &(_, step, index, range) in &self.steps {
            self.time = self.time.saturating_add(1);
            match step {
                Step::Close => {
                    open.remove(&range.left);
                }
                Step::Open => {
                    open.insert(range.left, index);
                    self.marks.set(range.left, range.right, self.time, index);
                }
                Step::CheckOpen => {
                    let found = neighbour(ranges, &open, &range);
                    if let Some(slot) = self.opened.get_mut(index) {
                        *slot = self.time;
                    }
                    if let Some(slot) = self.dropped.get_mut(index) {
                        *slot = found;
                    }
                }
                Step::CheckClose => {
                    let opened = self.opened.get(index).copied().unwrap_or(u64::MAX);
                    if let Some(slot) = self.dropped.get_mut(index)
                        && slot.is_none()
                    {
                        *slot = self.marks.since(range.left, range.right, opened);
                    }
                }
            }
        }
    }

    /// What every check found.
    pub(super) fn into_dropped(self) -> Vec<Option<usize>> {
        self.dropped
    }
}

/// The range of `open` that `range` overlaps, if any: of those open, none
/// overlapping another, only its neighbours by first column can.
fn neighbour(ranges: &[Range], open: &BTreeMap<u16, usize>, range: &Range) -> Option<usize> {
    let right = |index: &usize| ranges.get(*index).map_or(0, |open| open.right);
    let before = open.range(..=range.left).next_back();
    let before = before.filter(|(_, index)| right(index) >= range.left);
    let after = open.range((Excluded(range.left), Unbounded)).next();
    let after = after.filter(|&(&left, _)| left <= range.right);
    before.or(after).map(|(_, &index)| index)
}
