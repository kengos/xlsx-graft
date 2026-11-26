//! [`Rules`], what a walk may change in a part, and [`Outcome`], what walking
//! a part found.

use crate::structure::KeptReference;

/// What a walk may change in a part, by the part's kind.
#[derive(Debug, Clone, Copy)]
pub(in crate::structure::edit::parts) struct Rules {
    /// Whether the text of a `Rewrite` element is rewritten.
    pub(in crate::structure::edit::parts) formulas: bool,
    /// Whether a `worksheetSource`'s `sheet` is rewritten.
    pub(in crate::structure::edit::parts) pivot: bool,
}

/// What walking a part found.
#[derive(Default)]
pub(in crate::structure::edit::parts) struct Outcome {
    /// Whether the part's text is not what was read.
    pub(in crate::structure::edit::parts) changed: bool,
    /// Whether a `#REF!` was written.
    pub(in crate::structure::edit::parts) ref_written: bool,
    /// What was left as it was, up to `MAX_KEPT`.
    pub(in crate::structure::edit::parts) kept: Vec<KeptReference>,
    /// How many more were left as they were.
    pub(in crate::structure::edit::parts) omitted: u64,
}
