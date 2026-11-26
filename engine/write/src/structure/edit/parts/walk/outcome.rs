//! [`Rules`], what a walk looks at and may change in a part, and [`Outcome`], what walking
//! a part found.

use crate::structure::KeptReference;
use crate::structure::edit::tabs::Tabs;
use std::rc::Rc;

/// What a walk looks at in a part.
#[derive(Debug, Clone, Copy)]
pub(in crate::structure::edit::parts) enum Mode {
    /// The formula places (held and judged, reported when kept as they
    /// were), the pivot source and the sheet views: the edit renamed or
    /// deleted a sheet.
    Formulas {
        /// Whether the text of a `Rewrite` element is rewritten.
        rewrite: bool,
        /// Whether a `worksheetSource`'s `sheet` is rewritten.
        pivot: bool,
    },
    /// The sheet views only: the edit renamed and deleted no sheet, so no
    /// formula is looked at or reported, and every other event is written as
    /// it came.
    TabsOnly,
}

/// What a walk may look at and change in a part, by the part's kind and the
/// edit.
#[derive(Debug, Clone)]
pub(in crate::structure::edit::parts) struct Rules {
    /// What the walk looks at.
    pub(in crate::structure::edit::parts) mode: Mode,
    /// The `tabSelected` changes of the part's `sheetView`s, when the part
    /// is a sheet whose tabs change.
    pub(in crate::structure::edit::parts) tabs: Option<Rc<Tabs>>,
}

impl Rules {
    /// Whether the walk looks at formula places and a pivot source.
    pub(in crate::structure::edit::parts) fn looks(&self) -> bool {
        matches!(self.mode, Mode::Formulas { .. })
    }

    /// Whether the text of a `Rewrite` element is rewritten.
    pub(in crate::structure::edit::parts) fn rewrites(&self) -> bool {
        matches!(self.mode, Mode::Formulas { rewrite: true, .. })
    }

    /// Whether a `worksheetSource`'s `sheet` is rewritten.
    pub(in crate::structure::edit::parts) fn pivot(&self) -> bool {
        matches!(self.mode, Mode::Formulas { pivot: true, .. })
    }
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
