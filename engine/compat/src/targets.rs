//! [`Targets`]: a template's `targets:` and `bestEffort:` lists, read into
//! the versions of each application a workbook is checked against.
//!
//! `targets:` names the versions the workbook must work in, and a workbook
//! that does not is refused with an error; `bestEffort:` the versions it
//! should work in, and a workbook that does not is written with a warning.
//! An entry that names no versions is an error in either list. Each entry
//! is `name`, `name version` or `name OP version` (`excel >= 2016`,
//! `excel 365`, `google-sheets`); the entries of one list that name the
//! same application add up.

mod check;
mod entry;
mod versions;

pub use check::Unmet;

use crate::{App, Version};
use versions::Versions;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, TargetList, TargetReason};

/// The versions of each application a workbook must work in, and those it
/// should work in.
///
/// [`Targets::default`] holds none, so nothing is checked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Targets {
    required: PerApp,
    best_effort: PerApp,
}

impl Targets {
    /// Reads a template's `targets:` and `bestEffort:` lists, or `None`
    /// when any entry was reported.
    ///
    /// Every entry that names no versions is reported to `diagnostics` as a
    /// `target_invalid` error, and so is every `bestEffort:` entry that
    /// holds a version a `targets:` entry holds as well. The lists are read
    /// to the end either way, so a template with several problems hears of
    /// them all at once, in the order of the lists: `targets:` first.
    pub fn parse<T: AsRef<str>, B: AsRef<str>>(
        targets: &[T],
        best_effort: &[B],
        diagnostics: &mut Diagnostics,
    ) -> Option<Self> {
        let (required, required_read) =
            read_list(TargetList::Targets, targets, diagnostics, |_, _| None);
        let (best_effort, best_effort_read) = read_list(
            TargetList::BestEffort,
            best_effort,
            diagnostics,
            |app, versions| {
                let both = versions.and(required.get(app));
                let names = if app.has_versions() {
                    both.names(app.version_names())
                } else {
                    app.as_str().to_owned()
                };
                (!both.is_empty()).then_some((TargetReason::Overlap, names))
            },
        );
        (required_read && best_effort_read).then_some(Self {
            required,
            best_effort,
        })
    }

    /// Whether neither list holds a version, so nothing is checked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The versions of `V`'s application the workbook must work in, oldest
    /// first.
    pub fn required<V: Version>(&self) -> impl Iterator<Item = V> {
        self.required.get(V::APP).iter()
    }

    /// The versions of `V`'s application the workbook should work in,
    /// oldest first.
    pub fn best_effort<V: Version>(&self) -> impl Iterator<Item = V> {
        self.best_effort.get(V::APP).iter()
    }
}

/// Reads every entry of `list` into one set per application, reporting
/// each that names no versions, or that `check` finds a problem with; the
/// `bool` is whether none was reported.
#[track_caller]
fn read_list<S: AsRef<str>>(
    list: TargetList,
    entries: &[S],
    diagnostics: &mut Diagnostics,
    check: impl Fn(App, Versions) -> Option<entry::Problem>,
) -> (PerApp, bool) {
    let mut sets = PerApp::default();
    let mut read = true;
    for (index, entry) in entries.iter().enumerate() {
        let entry = entry.as_ref();
        let problem = match entry::read(entry) {
            Ok((app, versions)) => match check(app, versions) {
                None => {
                    sets.add(app, versions);
                    continue;
                }
                Some(problem) => problem,
            },
            Err(problem) => problem,
        };
        report(diagnostics, list, index, entry, problem);
        read = false;
    }
    (sets, read)
}

/// Reports the entry at `index` of `list` as naming no versions.
#[track_caller]
fn report(
    diagnostics: &mut Diagnostics,
    list: TargetList,
    index: usize,
    entry: &str,
    (reason, names): entry::Problem,
) {
    let diagnostic = Diagnostic::TargetInvalid {
        list,
        index: u64::try_from(index).unwrap_or(u64::MAX),
        entry: Echo::new(entry),
        reason,
        names: Echo::new(&names),
    };
    diagnostics.push(diagnostic, Location::default());
}

/// One set of versions per application.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PerApp {
    excel: Versions,
    excel_mac: Versions,
    libreoffice: Versions,
    google_sheets: Versions,
    numbers: Versions,
    onlyoffice: Versions,
}

impl PerApp {
    /// `app`'s set.
    fn get(&self, app: App) -> Versions {
        match app {
            App::Excel => self.excel,
            App::ExcelMac => self.excel_mac,
            App::LibreOffice => self.libreoffice,
            App::GoogleSheets => self.google_sheets,
            App::Numbers => self.numbers,
            App::OnlyOffice => self.onlyoffice,
        }
    }

    /// Adds `versions` to `app`'s set.
    fn add(&mut self, app: App, versions: Versions) {
        let set = match app {
            App::Excel => &mut self.excel,
            App::ExcelMac => &mut self.excel_mac,
            App::LibreOffice => &mut self.libreoffice,
            App::GoogleSheets => &mut self.google_sheets,
            App::Numbers => &mut self.numbers,
            App::OnlyOffice => &mut self.onlyoffice,
        };
        *set = set.or(versions);
    }
}

#[cfg(test)]
mod tests;
