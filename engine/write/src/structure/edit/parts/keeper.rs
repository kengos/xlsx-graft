//! [`Keeper`]: the references a walk of one part keeps for the report, up to
//! the number of references the report can still hold.

use crate::structure::{KeptReason, KeptReference};
use std::sync::Arc;

/// The references kept while one part is walked.
///
/// A reference counts against `remaining` (the number of references the
/// report can still hold) only when it is kept; once `remaining` references
/// are kept, each further one is counted in `omitted` and nothing of it is
/// held, so a part that names the sheet in millions of places holds no more
/// than `remaining` references.
pub(super) struct Keeper {
    /// The part's name, which every reference kept shares.
    part: Arc<str>,
    kept: Vec<KeptReference>,
    /// How many more references may be kept.
    remaining: usize,
    omitted: u64,
}

impl Keeper {
    /// A keeper for the part `part` that keeps up to `remaining` references.
    pub(super) fn new(part: &str, remaining: usize) -> Self {
        Self {
            part: Arc::from(part),
            kept: Vec::new(),
            remaining,
            omitted: 0,
        }
    }

    /// Keeps a reference at `place` (and `at`), to the `sheet` or the
    /// `table` it names, for `reason`; or counts it when `remaining` references are already kept.
    pub(super) fn keep(
        &mut self,
        place: &'static str,
        at: Option<&Arc<str>>,
        sheet: Option<&Arc<str>>,
        table: Option<&Arc<str>>,
        reason: KeptReason,
    ) {
        if self.kept.len() >= self.remaining {
            self.omitted = self.omitted.saturating_add(1);
            return;
        }
        self.kept.push(KeptReference::new(
            Arc::clone(&self.part),
            place,
            at.cloned(),
            sheet.cloned(),
            table.cloned(),
            reason,
        ));
    }

    /// What was kept, and how many more references were counted.
    pub(super) fn finish(self) -> (Vec<KeptReference>, u64) {
        (self.kept, self.omitted)
    }
}
