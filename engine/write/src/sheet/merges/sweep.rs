//! [`first_overlap`]: two ranges that share a cell, found by sweeping the
//! rows from the top in O(n log n).

use super::super::CellRange;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::ops::Bound::{Excluded, Unbounded};

/// A range whose top row is at or above the top row of the range being
/// checked and whose bottom row is at or below it.
#[derive(Debug, Clone, Copy)]
struct Live {
    /// The range's last column.
    right: u16,
    /// Its position among the ranges given.
    index: usize,
}

/// A pair of ranges among `existing` followed by `added` that share a cell,
/// as `(later, earlier)` by their order in that list: the pair found first
/// when the ranges are taken by top row, then left column; `None` when no
/// two do. It also reports two ranges of `existing` that overlap each other.
///
/// The ranges are taken by their top row, down the sheet. The ranges whose
/// rows include the top row of the range being checked are kept by their
/// left column, and no two of them share a column (two that did would
/// already have been returned), so a new range can only overlap the live
/// range with the nearest left column at or before its own, or the nearest
/// after it.
pub(crate) fn first_overlap(
    existing: &[CellRange],
    added: &[CellRange],
) -> Option<(CellRange, CellRange)> {
    let all: Vec<CellRange> = existing.iter().chain(added).copied().collect();
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by_key(|&i| (all[i].first().row(), all[i].first().column(), i));
    // The live ranges by left column, and by bottom row to remove the ones
    // whose bottom row is above the top row of the range being checked.
    let mut live: BTreeMap<u16, Live> = BTreeMap::new();
    let mut by_bottom: BinaryHeap<Reverse<(u32, u16)>> = BinaryHeap::new();
    for index in order {
        let range = all[index];
        let (left, right) = (range.first().column(), range.last().column());
        while let Some(&Reverse((bottom, gone))) = by_bottom.peek() {
            if bottom >= range.first().row() {
                break;
            }
            by_bottom.pop();
            live.remove(&gone);
        }
        let at_left = live.range(..=left).next_back();
        let at_right = live.range((Excluded(left), Unbounded)).next();
        let hit = at_left
            .filter(|(_, other)| other.right >= left)
            .or_else(|| at_right.filter(|(start, _)| **start <= right));
        if let Some((_, other)) = hit {
            return Some((all[index.max(other.index)], all[index.min(other.index)]));
        }
        live.insert(left, Live { right, index });
        by_bottom.push(Reverse((range.last().row(), left)));
    }
    None
}
