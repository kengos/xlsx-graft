//! [`Graph`]: the defined names, followed from one to the next, each
//! bucket scanned and each name read once.

use super::Context;
use super::body::{self, Finding};
use super::names::{Bucket, Names, Scope};
use std::collections::VecDeque;

/// The buckets scanned and the defined names read so far.
pub(super) struct Graph {
    /// Whether each bucket was scanned, by its index.
    scanned: Vec<bool>,
    /// Whether each defined name was read, or is on the way to be, by its
    /// index.
    read: Vec<bool>,
}

impl Graph {
    /// A graph over `names`, nothing scanned or read yet.
    pub(super) fn new(names: &Names) -> Self {
        Self {
            scanned: vec![false; names.bucket_count()],
            read: vec![false; names.len()],
        }
    }

    /// What a formula naming the buckets `roots` reaches that no formula
    /// reached before: the problems the formulas of the defined names in
    /// them hold, and those of the names those name, and so on, breadth
    /// first, each with the defined name whose formula holds it.
    pub(super) fn reach(
        &mut self,
        roots: &[Bucket],
        context: Context<'_>,
    ) -> Vec<(usize, Finding)> {
        let mut reached = Vec::new();
        let mut queue = VecDeque::new();
        self.scan(roots, context, &mut queue);
        while let Some(index) = queue.pop_front() {
            let body = context
                .names
                .get(index)
                .map_or_else(Default::default, |defined| {
                    body::read(&defined.formula, Scope::Any, context)
                });
            reached.extend(body.findings.into_iter().map(|finding| (index, finding)));
            self.scan(&body.names, context, &mut queue);
        }
        reached
    }

    /// Scans each of `buckets` not scanned before, queueing each defined
    /// name in it that is not read yet.
    fn scan(&mut self, buckets: &[Bucket], context: Context<'_>, queue: &mut VecDeque<usize>) {
        for &bucket in buckets {
            match self.scanned.get_mut(bucket.index()) {
                Some(scanned) if !*scanned => *scanned = true,
                _ => continue,
            }
            for &index in context.names.members(bucket) {
                if let Some(read) = self.read.get_mut(index)
                    && !*read
                {
                    *read = true;
                    queue.push_back(index);
                }
            }
        }
    }
}
