//! The diagnostics an [`Unmet`] makes, and the table's applications and
//! fallbacks as the diagnostics name them.

use super::Unmet;
use crate::{App, Fallback};
use xlsx_graft_diagnostics::{Diagnostic, Echo, TargetApp, TargetFallback, TargetList};

/// Builds the variant `$variant` of [`Diagnostic`] from the arguments
/// `$args` holds; the four target codes share their fields.
macro_rules! unmet {
    ($variant:ident, $args:expr) => {{
        let Args {
            name,
            app,
            unsupported,
            unverified,
            requires,
            fallback,
            count,
        } = $args;
        Diagnostic::$variant {
            name,
            app,
            unsupported,
            unverified,
            requires,
            fallback,
            count,
        }
    }};
}

/// The fields every target code has.
struct Args {
    name: Echo,
    app: TargetApp,
    unsupported: Echo,
    unverified: Echo,
    requires: Echo,
    fallback: TargetFallback,
    count: u64,
}

impl Unmet {
    /// The diagnostic of the function this is of, found `count` times:
    /// `target_function_unsupported` for `targets:`,
    /// `best_effort_function_unsupported` for `bestEffort:`.
    #[must_use]
    pub fn function_diagnostic(&self, count: u64) -> Diagnostic {
        match self.list {
            TargetList::Targets => unmet!(TargetFunctionUnsupported, self.args(count)),
            TargetList::BestEffort => unmet!(BestEffortFunctionUnsupported, self.args(count)),
        }
    }

    /// The diagnostic of the feature this is of, held by `count` parts:
    /// `target_feature_unsupported` for `targets:`,
    /// `best_effort_feature_unsupported` for `bestEffort:`.
    #[must_use]
    pub fn feature_diagnostic(&self, count: u64) -> Diagnostic {
        match self.list {
            TargetList::Targets => unmet!(TargetFeatureUnsupported, self.args(count)),
            TargetList::BestEffort => unmet!(BestEffortFeatureUnsupported, self.args(count)),
        }
    }

    /// The fields of either diagnostic, with `count`.
    fn args(&self, count: u64) -> Args {
        let requires: Vec<&str> = self.requires.iter().map(|f| f.as_str()).collect();
        Args {
            name: Echo::new(self.name),
            app: app(self.app),
            unsupported: Echo::new(&self.unsupported.join(", ")),
            unverified: Echo::new(&self.unverified.join(", ")),
            requires: Echo::new(&requires.join(", ")),
            fallback: fallback(self.fallback),
            count,
        }
    }
}

/// The diagnostic's application for the table's.
pub(super) fn app(app: App) -> TargetApp {
    match app {
        App::Excel => TargetApp::Excel,
        App::ExcelMac => TargetApp::ExcelMac,
        App::LibreOffice => TargetApp::LibreOffice,
        App::GoogleSheets => TargetApp::GoogleSheets,
        App::Numbers => TargetApp::Numbers,
        App::OnlyOffice => TargetApp::OnlyOffice,
    }
}

/// The diagnostic's fallback for the table's.
pub(super) fn fallback(fallback: Option<Fallback>) -> TargetFallback {
    match fallback {
        None => TargetFallback::Unknown,
        Some(Fallback::Ignored) => TargetFallback::Ignored,
        Some(Fallback::ValueOnly) => TargetFallback::ValueOnly,
        Some(Fallback::NameError) => TargetFallback::NameError,
        Some(Fallback::ValueError) => TargetFallback::ValueError,
        Some(Fallback::LegacyArray) => TargetFallback::LegacyArray,
        Some(Fallback::AsNote) => TargetFallback::AsNote,
        Some(Fallback::PlaceholderShape) => TargetFallback::PlaceholderShape,
        Some(Fallback::Degraded) => TargetFallback::Degraded,
        Some(Fallback::RepairPrompt) => TargetFallback::RepairPrompt,
        Some(Fallback::Unopenable) => TargetFallback::Unopenable,
    }
}
