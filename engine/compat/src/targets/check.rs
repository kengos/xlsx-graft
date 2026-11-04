//! [`Targets::check`]: which versions the targets hold lack a function,
//! or are not confirmed to have it, the features it needs included.

use super::{PerApp, Targets};
use crate::{
    App, Entry, Excel, ExcelMac, Fallback, Feature, GoogleSheets, LibreOffice, Numbers, OnlyOffice,
    Row, Status, Support, Version,
};
use xlsx_graft_diagnostics::{Diagnostic, Echo, TargetApp, TargetFallback, TargetList};

/// The versions of one application, in one of the template's lists, that
/// lack a function or are not confirmed to have it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Unmet {
    /// The function's name, as its row writes it.
    pub name: &'static str,
    /// The list that holds the versions.
    pub list: TargetList,
    /// The application.
    pub app: App,
    /// The versions that lack it, or a feature it needs, oldest first, as
    /// written.
    pub unsupported: Vec<&'static str>,
    /// The versions no source the table cites says have it, or a feature
    /// it needs, and none says lack them, oldest first. This or
    /// `unsupported` is never empty.
    pub unverified: Vec<&'static str>,
    /// The features it needs that make some of the versions worse than
    /// it is there itself: missing where it is there or not confirmed, or
    /// not confirmed where it is there; in the order first found.
    pub requires: Vec<Feature>,
    /// What a version that lacks it shows: its own cell's whenever it is
    /// not there itself, or not confirmed, in some of the versions, `None`
    /// when no source gives one; the first a source gives for the features
    /// in `requires` only when it is there itself in all of them. One value
    /// stands for every version, so where both kinds fail, the function's
    /// own is the one a reader most needs.
    pub fallback: Option<Fallback>,
}

impl Targets {
    /// What the versions the targets hold say of `row`: one [`Unmet`] per
    /// list and application with a version that lacks it, or is not
    /// confirmed to have it, `targets:` first, then the applications in
    /// the table's order. A version lacks it when it lacks the row's
    /// function or a feature the row requires, or one such a feature
    /// requires, followed without recursing.
    ///
    /// A function the table has no row for is not checked: the table says
    /// nothing about it.
    #[must_use]
    pub fn check(&self, row: &'static Row) -> Vec<Unmet> {
        let needs = needs(row);
        let mut unmet = Vec::new();
        for (list, versions) in [
            (TargetList::Targets, &self.required),
            (TargetList::BestEffort, &self.best_effort),
        ] {
            let mut column = Column {
                row,
                needs: &needs,
                versions,
                list,
                unmet: &mut unmet,
            };
            column.check::<Excel>(|support| &support.excel);
            column.check::<ExcelMac>(|support| &support.excel_mac);
            column.check::<LibreOffice>(|support| &support.libreoffice);
            column.check::<GoogleSheets>(|support| &support.google_sheets);
            column.check::<Numbers>(|support| &support.numbers);
            column.check::<OnlyOffice>(|support| &support.onlyoffice);
        }
        unmet
    }
}

impl Unmet {
    /// The diagnostic of the function this is of, found `count` times:
    /// `target_function_unsupported` for `targets:`,
    /// `best_effort_function_unsupported` for `bestEffort:`.
    #[must_use]
    pub fn function_diagnostic(&self, count: u64) -> Diagnostic {
        let requires: Vec<&str> = self.requires.iter().map(|f| f.as_str()).collect();
        let (name, app) = (Echo::new(self.name), app(self.app));
        let unsupported = Echo::new(&self.unsupported.join(", "));
        let unverified = Echo::new(&self.unverified.join(", "));
        let requires = Echo::new(&requires.join(", "));
        let fallback = fallback(self.fallback);
        match self.list {
            TargetList::Targets => Diagnostic::TargetFunctionUnsupported {
                name,
                app,
                unsupported,
                unverified,
                requires,
                fallback,
                count,
            },
            TargetList::BestEffort => Diagnostic::BestEffortFunctionUnsupported {
                name,
                app,
                unsupported,
                unverified,
                requires,
                fallback,
                count,
            },
        }
    }
}

/// One list's versions, checked one application at a time.
struct Column<'a> {
    row: &'static Row,
    needs: &'a [Feature],
    versions: &'a PerApp,
    list: TargetList,
    unmet: &'a mut Vec<Unmet>,
}

impl Column<'_> {
    /// Checks the versions of `V` the list holds against the cells `cell`
    /// picks.
    fn check<V: Version>(&mut self, cell: fn(&Support) -> &Entry<V>) {
        let own = cell(&self.row.support);
        let mut found = Unmet {
            name: self.row.name,
            list: self.list,
            app: V::APP,
            unsupported: Vec::new(),
            unverified: Vec::new(),
            requires: Vec::new(),
            fallback: None,
        };
        let mut missing_itself = false;
        for version in self.versions.get(V::APP).iter::<V>() {
            let itself = own.at(version);
            let mut status = itself;
            if itself != Status::Unsupported {
                for &feature in self.needs {
                    let entry = cell(&feature.row().support);
                    let needed = entry.at(version);
                    if worse(itself, needed) != itself && !found.requires.contains(&feature) {
                        found.requires.push(feature);
                        found.fallback = found.fallback.or(entry.fallback);
                    }
                    status = worse(status, needed);
                }
            }
            missing_itself |= itself != Status::Supported;
            match status {
                Status::Supported => {}
                Status::Unsupported => found.unsupported.push(version.as_str()),
                Status::Unverified => found.unverified.push(version.as_str()),
            }
        }
        if missing_itself {
            found.fallback = own.fallback;
        }
        if !(found.unsupported.is_empty() && found.unverified.is_empty()) {
            self.unmet.push(found);
        }
    }
}

/// The features `row` needs: those it requires, those they require, and
/// so on, each once, in the order first found. No feature requires
/// another today (the table's tests say so), so the walk goes one step;
/// it is a loop, not a recursion, for the day one does.
fn needs(row: &Row) -> Vec<Feature> {
    let mut needs: Vec<Feature> = Vec::new();
    add(&mut needs, row.requires);
    let mut next = 0;
    while let Some(&feature) = needs.get(next) {
        next = next.saturating_add(1);
        add(&mut needs, feature.row().requires);
    }
    needs
}

/// Adds each of `features` that `needs` does not hold yet.
fn add(needs: &mut Vec<Feature>, features: &[Feature]) {
    for &feature in features {
        if !needs.contains(&feature) {
            needs.push(feature);
        }
    }
}

/// The worse of two statuses: unsupported, then unverified.
fn worse(a: Status, b: Status) -> Status {
    match (a, b) {
        (Status::Unsupported, _) | (_, Status::Unsupported) => Status::Unsupported,
        (Status::Unverified, _) | (_, Status::Unverified) => Status::Unverified,
        (Status::Supported, Status::Supported) => Status::Supported,
    }
}

/// The diagnostic's application for the table's.
fn app(app: App) -> TargetApp {
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
fn fallback(fallback: Option<Fallback>) -> TargetFallback {
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

#[cfg(test)]
mod tests;
