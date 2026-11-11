//! xlsx-graft host: what the hosts (the CLI, the C ABI and the MCP server)
//! share to verify a workbook, so that each reads the policy and the rules,
//! and writes the result, the same way.
//!
//! A verification has three parts:
//!
//! 1. The **policy** ([`Policy`]) is the host's: the limits a workbook is
//!    held to ([`LIMITS`] lists each with its hard max), what it
//!    forbids, whether it denies the scheduled functions (`denyScheduled`)
//!    before the version that will deny them, and how strict the result is.
//! 2. The **rules** are the caller's, with the template's keys: `targets`,
//!    `bestEffort`, `functions.allow`, `parts.allow` and `failOn`.
//! 3. [`prepare`], [`prepare_parts`] or [`prepare_request`] reads both,
//!    and [`verify_path`], [`verify_bytes`] or [`verify`] runs the check
//!    and gives the [`Report`], which [`Report::to_json`] writes as the
//!    JSON each host returns; [`ReportText`] writes it as the text a person
//!    reads.
//!
//! A fault of the policy or the rules is reported as a diagnostic
//! (`policy_invalid`, `rules_invalid`, `target_invalid`), all of them at
//! once, and the workbook is not opened; the report then holds only the
//! `policy_invalid`, `rules_invalid` and `target_invalid` records (and the
//! `diagnostics_truncated` notice, when there are more than it lists).
//! The engine failing to open or read the package is the report's `error`
//! (`io`), not a Rust error, so that every host returns the same object.
//!
//! The policy and the rules are JSON a host passes in: they are read as
//! untrusted input, under a size bound ([`MAX_SETTING_BYTES`],
//! [`MAX_REQUEST_BYTES`]), and an unknown key is a fault.

mod input;
mod policy;
mod prepare;
mod report;
mod rules;
#[cfg(test)]
mod support;
mod text;
mod walk;

pub use input::{Input, MAX_REQUEST_BYTES, MAX_SETTING_BYTES};
pub use policy::Policy;
pub use policy::limits::{HostLimits, LIMITS, LimitKey};
pub use prepare::{
    Options, Prepared, RequestError, check_policy, prepare, prepare_parts, prepare_request,
};
pub use report::{Counts, FailOn, Failure, Report};
pub use text::{DiagnosticsText, ReportText};

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_verify::{DefaultFunctions, VerifyError};

/// Verifies the workbook `reader` holds under `prepared`.
///
/// Nothing is read when `prepared` is invalid; the report holds its faults.
/// A failure to read the package is the report's `error`.
pub fn verify<R: Read + Seek>(reader: R, prepared: &Prepared) -> Report {
    let options = match prepared {
        Prepared::Ready(options) => options,
        Prepared::Invalid {
            diagnostics,
            fail_on,
        } => return Report::refused(diagnostics.clone(), *fail_on),
    };
    let mut diagnostics = Diagnostics::with_limit(options.limits.max_diagnostics);
    let run = xlsx_graft_verify::verify(
        reader,
        &options.limits.verify,
        &options.rules,
        &DefaultFunctions,
        &options.targets,
        &mut diagnostics,
    );
    let error = run.err().map(|VerifyError::Package(error)| {
        Failure::io(format!("the package could not be read: {error}"))
    });
    Report::finished(diagnostics, options.fail_on, error)
}

/// Verifies the workbook at `path` under `prepared`; a file that cannot be
/// opened is the report's `error`.
pub fn verify_path(path: &Path, prepared: &Prepared) -> Report {
    let options = match prepared {
        Prepared::Ready(options) => options,
        Prepared::Invalid { .. } => return verify(Cursor::new(&[][..]), prepared),
    };
    match File::open(path) {
        Ok(file) => verify(BufReader::new(file), prepared),
        Err(error) => {
            let message = format!("{} could not be opened: {error}", path.display());
            let failure = Some(Failure::io(message));
            Report::finished(Diagnostics::new(), options.fail_on, failure)
        }
    }
}

/// Verifies the workbook `bytes` holds under `prepared`, without copying it.
pub fn verify_bytes(bytes: &[u8], prepared: &Prepared) -> Report {
    verify(Cursor::new(bytes), prepared)
}

#[cfg(test)]
mod tests;
