//! [`Used`]: the features of the compatibility table a workbook was found
//! to use as its parts were read (`parts/features.rs` finds them), each
//! checked against the [`Targets`].
//!
//! A feature is a fact of the workbook, not of one part: each gets one
//! diagnostic per application and list, at the first part that holds it,
//! counting the parts that do. A function that needs a feature the
//! workbook was found to use is not blamed for it again
//! ([`Targets::check_besides`]): the version that lacks the feature is
//! reported once, for the feature.

use xlsx_graft_compat::{Feature, Targets};
use xlsx_graft_diagnostics::{Diagnostics, Location};

/// The features found, in the order first found: by part, then in the
/// table's order.
#[derive(Debug, Default)]
pub(crate) struct Used {
    /// Each feature, the first part that holds it and how many do.
    found: Vec<Found>,
}

/// One feature found.
#[derive(Debug)]
struct Found {
    feature: Feature,
    location: Location,
    count: u64,
}

impl Used {
    /// Notes that the part `part` holds `feature`.
    pub(crate) fn add(&mut self, feature: Feature, part: &str) {
        if let Some(found) = self.found.iter_mut().find(|f| f.feature == feature) {
            found.count = found.count.saturating_add(1);
            return;
        }
        self.found.push(Found {
            feature,
            location: Location::default().with_part(part),
            count: 1,
        });
    }

    /// The features found.
    pub(crate) fn features(&self) -> Vec<Feature> {
        self.found.iter().map(|found| found.feature).collect()
    }

    /// Pushes a diagnostic into `diagnostics` for each feature found and
    /// each list and application of `targets` with a version that lacks
    /// it, or is not confirmed to have it; a feature it needs that was
    /// found too is left to its own.
    pub(crate) fn judge(self, targets: &Targets, diagnostics: &mut Diagnostics) {
        let features = self.features();
        for entry in self.found {
            for unmet in targets.check_besides(entry.feature.row(), &features) {
                let diagnostic = unmet.feature_diagnostic(entry.count);
                diagnostics.push(diagnostic, entry.location.clone());
            }
        }
    }
}
