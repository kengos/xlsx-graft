//! [`Found`]: what the walks of all the parts came to.

use crate::structure::KeptReference;
use crate::structure::report::MAX_KEPT;

/// What the formulas of the other parts came to.
#[derive(Default)]
pub(in crate::structure::edit) struct Found {
    /// Whether a `#REF!` was written.
    pub(in crate::structure::edit) ref_written: bool,
    /// What was left as it was, up to `MAX_KEPT`.
    pub(in crate::structure::edit) kept: Vec<KeptReference>,
    /// How many more were left as they were.
    pub(in crate::structure::edit) omitted: u64,
}

impl Found {
    /// Adds `kept`, which holds no more references than `MAX_KEPT` had left,
    /// and the `omitted` that did not fit.
    pub(super) fn add(&mut self, kept: Vec<KeptReference>, omitted: u64) {
        self.kept.extend(kept);
        self.omitted = self.omitted.saturating_add(omitted);
    }

    /// The number of references the report can still hold.
    pub(super) fn remaining(&self) -> usize {
        MAX_KEPT.saturating_sub(self.kept.len())
    }
}
