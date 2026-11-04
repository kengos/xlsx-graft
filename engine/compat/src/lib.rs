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
//! `_xlws.` prefix. The table holds the functions Excel 2010 and later
//! added; it has no row yet for one Excel 2007 already had, and says
//! nothing about it. [`Feature::row`] gives a feature's row.
//!
//! [`Targets`] reads a template's `targets:` and `bestEffort:` lists
//! (`excel >= 2016`, `google-sheets`) into the versions of each application
//! a workbook must, and should, work in, and reports every entry that names
//! none as a `target_invalid` diagnostic.
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
mod table;
mod targets;

pub use app::{App, Excel, ExcelMac, GoogleSheets, LibreOffice, Numbers, OnlyOffice, Version};
pub use entry::{Added, Entry, Fallback, Status};
pub use row::{Row, Support};
pub use table::Feature;
pub use targets::Targets;

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
    table::FUNCTIONS
        .iter()
        .find(|row| row.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests;
