//! xlsx-graft compat: which version of each application supports each
//! function and feature a workbook may use, and what an older one shows
//! instead.
//!
//! The table has one [`Row`] per function or feature. A row's [`Support`]
//! has one [`Entry`] per [`App`]: when the application [`Added`] it (a
//! version, never, or not known), whether it differs from Excel's, what a
//! version that lacks it shows ([`Fallback`]) and notes. It also lists the
//! [`Feature`]s it needs as well, and the URLs of the sources every cell
//! was read from. [`Entry::at`] answers for one version: [`Status`]
//! supported, unsupported, or unverified when no source says.
//!
//! [`function`] looks a function up by its name without the `_xlfn.` or
//! `_xlws.` prefix. The table checks the functions Excel 2010 and later
//! added against a template's targets, all but ANCHORARRAY, SINGLE and
//! COPILOT; it has no [`Row`] yet for one
//! Excel 2007 already had, and says nothing about which versions support
//! it. [`Feature::row`] gives a feature's row.
//!
//! [`function_safety`] gives the [`Safety`] of every function the table
//! lists, including those Excel 2007 had: reviewed safe
//! ([`Review`] says what was read), dangerous for a kind of [`Danger`], or
//! not reviewed. It gives `None` for a name the table does not list, which
//! verify is to deny. [`safeties`] lists every function's safety.
//!
//! [`Targets`] reads a template's `targets:` and `bestEffort:` lists
//! (`excel >= 2016`, `google-sheets`) into the versions of each application
//! a workbook must, and should, work in, and reports every entry that names
//! none as a `target_invalid` diagnostic; a template without `targets:` is
//! taken to say [`DEFAULT_TARGETS`]. [`Targets::check`] says which of
//! those versions lack a function or a feature, or are not confirmed to
//! have it, the features it needs included ([`Unmet`]);
//! [`Unmet::function_diagnostic`] is the diagnostic a workbook that calls
//! the function gets, and [`Unmet::feature_diagnostic`] the one a workbook
//! that uses the feature gets. [`Targets::check_besides`] leaves out the
//! features a workbook was found to use, each reported on its own.
//!
//! # The data
//!
//! The shape follows MDN's browser-compat-data: a version, `false` or
//! `null` per application, kept apart so that "not known" never reads as
//! "no". Each application's versions are an enum of their own, so a row
//! cannot name a version the application never had: Excel's are 2007 to
//! 2021 (the LTSC version), 2024 and 365, Google Sheets has only the
//! current one, LibreOffice's and ONLYOFFICE's lists start in early 2024
//! and Numbers' at the oldest version Apple's history lists. A cell is filled
//! only from a source the row cites; one no source settles is unknown. Each
//! data file's header says how its rows are written.

mod app;
mod entry;
mod row;
mod safety;
mod table;
mod targets;

pub use app::{App, Excel, ExcelMac, GoogleSheets, LibreOffice, Numbers, OnlyOffice, Version};
pub use entry::{Added, Entry, Fallback, Status};
pub use row::{Row, Support};
pub use safety::{Danger, Dangerous, Review, Safe, Safety, SafetyRow, Scheduled};
pub use table::Feature;
pub use targets::{DEFAULT_TARGETS, Targets, Unmet};

use std::collections::HashMap;
use std::sync::OnceLock;

/// Every function the table has a row for.
#[must_use]
pub fn functions() -> &'static [Row] {
    table::FUNCTIONS
}

/// The row of the function `name`, written without its `_xlfn.` or
/// `_xlws.` prefix, in any case; `None` when the table has no row for it:
/// a function Excel 2007 already had, or a name no application here
/// knows.
#[must_use]
pub fn function(name: &str) -> Option<&'static Row> {
    /// The rows by name, built at the first look-up: a workbook's formulas
    /// look up every function they call.
    static BY_NAME: OnceLock<HashMap<&'static str, &'static Row>> = OnceLock::new();
    let by_name = BY_NAME.get_or_init(|| {
        (table::FUNCTIONS.iter())
            .map(|row| (row.name, row))
            .collect()
    });
    by_name.get(name.to_ascii_uppercase().as_str()).copied()
}

/// Every function's safety: the functions [`functions`] lists, then those
/// the table does not check against a template's targets yet.
#[must_use]
pub fn safeties() -> &'static [SafetyRow] {
    table::SAFETY
}

/// The safety of the function `name`, written without its `_xlfn.` or
/// `_xlws.` prefixes (`_xlfn._xlws.SORT` is `SORT`), in any case; `None`
/// for a name the table does not list, and for a name written with its
/// prefix (`_xlfn.XLOOKUP`). A defined name that holds a LAMBDA and an
/// add-in's function (`_xll.`) are not in the table either.
#[must_use]
pub fn function_safety(name: &str) -> Option<Safety> {
    /// The safeties by name, built at the first look-up.
    static BY_NAME: OnceLock<HashMap<&'static str, Safety>> = OnceLock::new();
    let by_name = BY_NAME.get_or_init(|| {
        (table::SAFETY.iter())
            .map(|row| (row.name, row.safety))
            .collect()
    });
    by_name.get(name.to_ascii_uppercase().as_str()).copied()
}

#[cfg(test)]
mod tests;
