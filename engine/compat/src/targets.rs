//! [`Targets`]: a template's `targets:` and `bestEffort:` lists, read into
//! the versions of each application a workbook is checked against.
//!
//! `targets:` names the versions the workbook must work in, and a workbook
//! that does not is refused with an error; `bestEffort:` the versions it
//! should work in, and a workbook that does not is written with a warning.
//! An entry that names no versions is an error in either list. Each entry
//! is `name`, `name version` or `name OP version` (`excel >= 2016`,
//! `excel 365`, `google-sheets`); the entries of one list that name the
//! same application add up. A template without `targets:`, or with an
//! empty one, is taken to say [`DEFAULT_TARGETS`], whatever its
//! `bestEffort:` says; a `bestEffort:` version the default holds adds
//! nothing and is not reported.

mod check;
mod entry;
mod versions;

pub use check::Unmet;

use crate::{App, Version};
use versions::Versions;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, TargetList, TargetReason};

/// The `targets:` a template without one is taken to say.
pub const DEFAULT_TARGETS: [&str; 2] = ["excel >= 2024", "excel-mac >= 2024"];

/// The versions of each application a workbook must work in, and those it
/// should work in.
///
/// [`Targets::default`] is a template's with neither list: it holds
/// [`DEFAULT_TARGETS`] and no `bestEffort:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targets {
    required: PerApp,
    best_effort: PerApp,
}

impl Default for Targets {
    fn default() -> Self {
        Self {
            required: default_targets(),
            best_effort: PerApp::default(),
        }
    }
}

impl Targets {
    /// Reads a template's `targets:` and `bestEffort:` lists, or `None`
    /// when any entry was reported.
    ///
    /// A template without `targets:`, an empty one included, is read as
    /// saying [`DEFAULT_TARGETS`]. Every entry that names no versions is
    /// reported to `diagnostics` as a `target_invalid` error, and so is
    /// every `bestEffort:` entry that holds a version a written `targets:`
    /// entry holds as well; a version the default holds is left out of
    /// `bestEffort:` instead, unreported. The lists are read
    /// to the end either way, so a template with several problems hears of
    /// them all at once, in the order of the lists: `targets:` first.
    pub fn parse<T: AsRef<str>, B: AsRef<str>>(
        targets: &[T],
        best_effort: &[B],
        diagnostics: &mut Diagnostics,
    ) -> Option<Self> {
        let written = !targets.is_empty();
        let (required, required_read) = if written {
            read_list(TargetList::Targets, targets, diagnostics, |_, versions| {
                Ok(versions)
            })
        } else {
            (default_targets(), true)
        };
        let (best_effort, best_effort_read) = read_list(
            TargetList::BestEffort,
            best_effort,
            diagnostics,
            |app, versions| {
                let both = versions.and(required.get(app));
                if !written || both.is_empty() {
                    return Ok(versions.without(both));
                }
                let names = if app.has_versions() {
                    both.names(app.version_names())
                } else {
                    app.as_str().to_owned()
                };
                Err((TargetReason::Overlap, names))
            },
        );
        (required_read && best_effort_read).then_some(Self {
            required,
            best_effort,
        })
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

/// The versions [`DEFAULT_TARGETS`] holds. Its entries all name versions
/// (the tests say so), so nothing is reported.
fn default_targets() -> PerApp {
    let (sets, _) = read_list(
        TargetList::Targets,
        &DEFAULT_TARGETS,
        &mut Diagnostics::new(),
        |_, versions| Ok(versions),
    );
    sets
}

/// Reads every entry of `list` into one set per application, reporting
/// each that names no versions, or that `check` finds a problem with;
/// `check` gives the versions to add otherwise. The `bool` is whether none
/// was reported.
#[track_caller]
fn read_list<S: AsRef<str>>(
    list: TargetList,
    entries: &[S],
    diagnostics: &mut Diagnostics,
    check: impl Fn(App, Versions) -> Result<Versions, entry::Problem>,
) -> (PerApp, bool) {
    let mut sets = PerApp::default();
    let mut read = true;
    for (index, entry) in entries.iter().enumerate() {
        let entry = entry.as_ref();
        let read_entry = entry::read(entry)
            .and_then(|(app, versions)| check(app, versions).map(|versions| (app, versions)));
        match read_entry {
            Ok((app, versions)) => sets.add(app, versions),
            Err(problem) => {
                report(diagnostics, list, index, entry, problem);
                read = false;
            }
        }
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
