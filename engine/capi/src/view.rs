//! [`XlsxGraftView`]: the result of one view of a workbook (its tree, or the
//! cells of one sheet), behind the view handle, and which status each fault of
//! the options returns, in safe Rust; `abi/view.rs` only marshals.
//!
//! The handle holds the result's JSON and nothing else: it does not change
//! after it is made, so any thread may read it, and a view shares no state
//! with another, so any number may run at once. The limits a workbook is
//! read under are [`HostLimits::default`], the CLI's, and no policy is
//! taken.

use crate::message::refused;
use crate::status::{XLSX_GRAFT_ERR_INVALID_ARG, XLSX_GRAFT_ERR_PANIC};
use std::path::Path;
use xlsx_graft_host::{
    HostLimits, ViewOptionsError, describe_sheet_bytes, describe_sheet_path, sheet_options,
    tree_bytes, tree_options, tree_path,
};

/// The result of one view of a workbook.
#[derive(Debug)]
pub struct XlsxGraftView {
    /// The result as JSON: `{"ok", "error", "diagnostics", ...}` and then
    /// the keys of what the view shows.
    json: String,
}

impl XlsxGraftView {
    /// The tree of the workbook at `path` under the `options` of a tree.
    ///
    /// # Errors
    ///
    /// The status of each fault of `options` is the table in the VIEWS
    /// section of `include/xlsx_graft.h`; [`XLSX_GRAFT_ERR_PANIC`] when the
    /// result cannot be written as JSON.
    pub(crate) fn tree_path(path: &str, options: &[u8]) -> Result<Self, i32> {
        tree_options(options).map_err(rejected)?;
        Self::of(tree_path(Path::new(path), &HostLimits::default()).to_json())
    }

    /// The tree of the workbook `data` holds, read in place for the call.
    ///
    /// # Errors
    ///
    /// As [`tree_path`](Self::tree_path).
    pub(crate) fn tree_bytes(data: &[u8], options: &[u8]) -> Result<Self, i32> {
        tree_options(options).map_err(rejected)?;
        Self::of(tree_bytes(data, &HostLimits::default()).to_json())
    }

    /// The cells of the sheet the `options` of `describe_sheet` choose in
    /// the workbook at `path`.
    ///
    /// # Errors
    ///
    /// The status of each fault of `options` is the table in the VIEWS
    /// section of `include/xlsx_graft.h` (a fault of the value of a key keeps
    /// its message); [`XLSX_GRAFT_ERR_PANIC`] when the result cannot be
    /// written as JSON.
    pub(crate) fn describe_sheet_path(path: &str, options: &[u8]) -> Result<Self, i32> {
        let request = sheet_options(options).map_err(rejected)?;
        Self::of(describe_sheet_path(Path::new(path), &request, &HostLimits::default()).to_json())
    }

    /// The cells of the sheet the `options` of `describe_sheet` choose in
    /// the workbook `data` holds, read in place for the call.
    ///
    /// # Errors
    ///
    /// As [`describe_sheet_path`](Self::describe_sheet_path).
    pub(crate) fn describe_sheet_bytes(data: &[u8], options: &[u8]) -> Result<Self, i32> {
        let request = sheet_options(options).map_err(rejected)?;
        Self::of(describe_sheet_bytes(data, &request, &HostLimits::default()).to_json())
    }

    /// The result's JSON.
    pub(crate) fn json(&self) -> &str {
        &self.json
    }

    /// A handle holding `json`, the view's result written as JSON.
    /// `View::to_json` fails only when a value cannot be written as JSON,
    /// which no view's content holds; a failure of it is an engine bug,
    /// returned as [`XLSX_GRAFT_ERR_PANIC`] as a caught panic is.
    fn of(json: serde_json::Result<String>) -> Result<Self, i32> {
        let json = json.map_err(|_| XLSX_GRAFT_ERR_PANIC)?;
        Ok(Self { json })
    }
}

/// The status of a fault of the options, as the VIEWS table in
/// `include/xlsx_graft.h` says: a fault of the value of a key is
/// [`XLSX_GRAFT_ERR_REQUEST_INVALID`](crate::XLSX_GRAFT_ERR_REQUEST_INVALID)
/// with the reason kept as the thread's message; any other is
/// [`XLSX_GRAFT_ERR_INVALID_ARG`].
fn rejected(error: ViewOptionsError) -> i32 {
    match error {
        ViewOptionsError::Arguments(error) => refused(error),
        ViewOptionsError::TooLarge
        | ViewOptionsError::NotJson(_)
        | ViewOptionsError::NotObject
        | ViewOptionsError::UnknownKey(_) => XLSX_GRAFT_ERR_INVALID_ARG,
    }
}

#[cfg(test)]
mod tests;
