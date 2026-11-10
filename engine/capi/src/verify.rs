//! [`XlsxGraftVerify`]: the result of one verification, behind the verify
//! handle, and the deciding logic of the verify entry points in safe Rust;
//! `abi/verify.rs` only marshals.
//!
//! The handle holds the result's JSON and nothing else: it does not change
//! after it is made, so any thread may read it, and a verification shares
//! no state with another, so any number may run at once.

use crate::status::{XLSX_GRAFT_ERR_INVALID_ARG, XLSX_GRAFT_ERR_PANIC};
use std::path::Path;
use xlsx_graft_host::{Input, Prepared, Report, check_policy, prepare_request};

/// The result of one verification, or of one check of a policy.
#[derive(Debug)]
pub struct XlsxGraftVerify {
    /// The result as JSON: `{"ok", "failOn", "counts", "error", "diagnostics"}`.
    json: String,
}

impl XlsxGraftVerify {
    /// Verifies the workbook at `path` under the request `options`.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_INVALID_ARG`] when `options` is not a request (an
    /// object of `policy` and `rules`), and [`XLSX_GRAFT_ERR_PANIC`] when the
    /// result cannot be written as JSON.
    pub(crate) fn path(path: &str, options: &[u8]) -> Result<Self, i32> {
        let prepared = request(options)?;
        Self::of(&xlsx_graft_host::verify_path(Path::new(path), &prepared))
    }

    /// Verifies the workbook `data` holds, read in place for the call.
    ///
    /// # Errors
    ///
    /// As [`path`](Self::path): [`XLSX_GRAFT_ERR_INVALID_ARG`] or
    /// [`XLSX_GRAFT_ERR_PANIC`].
    pub(crate) fn bytes(data: &[u8], options: &[u8]) -> Result<Self, i32> {
        let prepared = request(options)?;
        Self::of(&xlsx_graft_host::verify_bytes(data, &prepared))
    }

    /// Checks the policy `policy` alone: the result is `ok` when the policy
    /// is valid, and holds its `policy_invalid` diagnostics when it is not.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_PANIC`] when the result cannot be written as JSON.
    pub(crate) fn policy(policy: &[u8]) -> Result<Self, i32> {
        Self::of(&check_policy(Input::Bytes(policy)))
    }

    /// The result's JSON.
    pub(crate) fn json(&self) -> &str {
        &self.json
    }

    /// `report` as JSON. `Report::to_json` cannot fail in practice; a
    /// failure of it is an engine bug, and is returned as
    /// [`XLSX_GRAFT_ERR_PANIC`], as a caught panic is.
    fn of(report: &Report) -> Result<Self, i32> {
        let json = report.to_json().map_err(|_| XLSX_GRAFT_ERR_PANIC)?;
        Ok(Self { json })
    }
}

/// The request read, or [`XLSX_GRAFT_ERR_INVALID_ARG`] when `options` is
/// not an object of `policy` and `rules`.
fn request(options: &[u8]) -> Result<Prepared, i32> {
    prepare_request(options).map_err(|_| XLSX_GRAFT_ERR_INVALID_ARG)
}

#[cfg(test)]
mod tests;
