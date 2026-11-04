//! [`Marks`]: the columns of the sheet, each marked with the last range
//! that opened over it and when, in a segment tree over the columns up to
//! the last a range reaches: 16,384 at the most, whatever the sheet holds.
//!
//! A mark set over columns is put on the few nodes that cover them
//! exactly, and each node keeps the latest mark below it. The marks are
//! set in the order of time, so the latest is the largest, and one is
//! never taken back: a mark of an earlier sweep is older than any range a
//! later sweep checks.

/// A mark: when, and which range, `(0, 0)` for none.
type Mark = (u64, usize);

/// The marks of the columns.
#[derive(Debug)]
pub(super) struct Marks {
    /// The leaves, one for each column up to the last, a power of two.
    leaves: usize,
    /// For each node, the mark set over all its columns last.
    whole: Vec<Mark>,
    /// For each node, the latest mark set over any of its columns.
    latest: Vec<Mark>,
}

impl Marks {
    /// No column up to `last` marked.
    pub(super) fn new(last: u16) -> Self {
        let leaves = usize::from(last).next_power_of_two();
        Self {
            leaves,
            whole: vec![(0, 0); 2 * leaves],
            latest: vec![(0, 0); 2 * leaves],
        }
    }

    /// Marks the columns `left` to `right`, from 1, with `range` at `time`,
    /// later than every mark set before.
    pub(super) fn set(&mut self, left: u16, right: u16, time: u64, range: usize) {
        let (first, last) = self.leaves(left, right);
        let (mut low, mut high) = (first, last.saturating_add(1));
        while low < high {
            if low & 1 == 1 {
                self.put(low, (time, range));
                low += 1;
            }
            if high & 1 == 1 {
                high -= 1;
                self.put(high, (time, range));
            }
            (low, high) = (low >> 1, high >> 1);
        }
        // The nodes above those marked are on the paths up from the first
        // and the last column: each holds one of them.
        for leaf in [first, last] {
            let mut node = leaf >> 1;
            while node >= 1 {
                let below = self.latest_of(2 * node).max(self.latest_of(2 * node + 1));
                let latest = self.whole_of(node).max(below);
                if let Some(slot) = self.latest.get_mut(node) {
                    *slot = latest;
                }
                node >>= 1;
            }
        }
    }

    /// The range of the latest mark over any of the columns `left` to
    /// `right`, when it was set after `time`.
    pub(super) fn since(&self, left: u16, right: u16, time: u64) -> Option<usize> {
        let (first, last) = self.leaves(left, right);
        let mut found = (0, 0);
        for leaf in [first, last] {
            let mut node = leaf >> 1;
            while node >= 1 {
                found = found.max(self.whole_of(node));
                node >>= 1;
            }
        }
        let (mut low, mut high) = (first, last.saturating_add(1));
        while low < high {
            if low & 1 == 1 {
                found = found.max(self.latest_of(low));
                low += 1;
            }
            if high & 1 == 1 {
                high -= 1;
                found = found.max(self.latest_of(high));
            }
            (low, high) = (low >> 1, high >> 1);
        }
        (found.0 > time).then_some(found.1)
    }

    /// The leaves of the columns `left` and `right`, from 1.
    fn leaves(&self, left: u16, right: u16) -> (usize, usize) {
        let leaf = |column: u16| self.leaves + usize::from(column.saturating_sub(1));
        (leaf(left), leaf(right))
    }

    /// Sets `mark` over every column of `node`.
    fn put(&mut self, node: usize, mark: Mark) {
        for marks in [&mut self.whole, &mut self.latest] {
            if let Some(slot) = marks.get_mut(node) {
                *slot = mark;
            }
        }
    }

    /// The mark set over every column of `node` last.
    fn whole_of(&self, node: usize) -> Mark {
        self.whole.get(node).copied().unwrap_or_default()
    }

    /// The latest mark over any column of `node`.
    fn latest_of(&self, node: usize) -> Mark {
        self.latest.get(node).copied().unwrap_or_default()
    }
}
