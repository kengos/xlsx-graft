//! The message of a target code: what a workbook uses that some version a
//! template's `targets:` or `bestEffort:` lists lacks, or is not
//! confirmed to have.

use crate::{Echo, TargetApp, TargetFallback};

/// What a target code's message says.
pub(crate) struct Unmet<'a> {
    /// What the workbook does, as the message opens: `a formula calls
    /// XLOOKUP`.
    pub(crate) what: String,
    /// The list, as the template spells it: `targets` or `bestEffort`.
    pub(crate) list: &'static str,
    /// The application.
    pub(crate) app: TargetApp,
    /// The versions that lack it, comma-separated; may be empty, though
    /// not with `unverified`.
    pub(crate) unsupported: &'a Echo,
    /// The versions not confirmed to have it, comma-separated; may be
    /// empty.
    pub(crate) unverified: &'a Echo,
    /// The features it needs as well that some versions lack where it is
    /// there itself, comma-separated; may be empty.
    pub(crate) requires: &'a Echo,
    /// What a version that lacks it shows.
    pub(crate) fallback: TargetFallback,
    /// How many times it was found, or, with `parts`, how many parts hold
    /// it.
    pub(crate) count: u64,
    /// Whether `count` counts parts rather than times.
    pub(crate) parts: bool,
}

impl Unmet<'_> {
    /// The English message.
    pub(crate) fn message(&self) -> String {
        let mut facts = Vec::new();
        if !self.unsupported.as_str().is_empty() {
            facts.push(format!("that lack it ({})", self.unsupported));
        }
        if !self.unverified.as_str().is_empty() {
            facts.push(format!(
                "that are not confirmed to have it ({})",
                self.unverified
            ));
        }
        if facts.is_empty() {
            facts.push("that may lack it".to_owned());
        }
        let mut message = format!(
            "{}; {} lists {} versions {}",
            self.what,
            self.list,
            spelled(self.app),
            facts.join(" and ")
        );
        if !self.requires.as_str().is_empty() {
            message += &format!(
                "; in some of them, it needs {} as well, which is missing or not confirmed there",
                self.requires
            );
        }
        if let Some(shows) = shows(self.fallback) {
            message += &format!("; where it is missing, if it is, {shows}");
        }
        let count = self.count;
        message
            + &if self.parts {
                format!("; in {count} part(s), the first here")
            } else {
                format!("; {count} time(s), the first here")
            }
    }
}

/// `app` as the template spells it.
pub(crate) fn spelled(app: TargetApp) -> &'static str {
    match app {
        TargetApp::Excel => "excel",
        TargetApp::ExcelMac => "excel-mac",
        TargetApp::LibreOffice => "libreoffice",
        TargetApp::GoogleSheets => "google-sheets",
        TargetApp::Numbers => "numbers",
        TargetApp::OnlyOffice => "onlyoffice",
    }
}

/// What a version that lacks it shows, in words; `None` when no source
/// says.
fn shows(fallback: TargetFallback) -> Option<&'static str> {
    Some(match fallback {
        TargetFallback::Unknown => return None,
        TargetFallback::Ignored => "it is skipped and not shown",
        TargetFallback::ValueOnly => "only the value saved with the cell is shown",
        TargetFallback::NameError => "the formula evaluates to #NAME?",
        TargetFallback::ValueError => "the cell shows #VALUE!",
        TargetFallback::LegacyArray => "it is a legacy array formula of a fixed size",
        TargetFallback::AsNote => "it shows as a legacy note",
        TargetFallback::PlaceholderShape => "a shape stands in for it",
        TargetFallback::Degraded => "only part of it works",
        TargetFallback::RepairPrompt => "the application offers to repair the workbook",
        TargetFallback::Unopenable => "the workbook does not open",
    })
}

#[cfg(test)]
mod tests;
