//! `--version`: the engine version.

use crate::Report;
use serde::Serialize;
use std::io::{self, Write};

/// The output of `--version`.
///
/// JSON: `{"version":"<semver>"}`.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct VersionReport {
    /// The engine version, from [`xlsx_graft_core::version`].
    pub version: &'static str,
}

impl VersionReport {
    /// The running engine's version.
    #[must_use]
    pub fn current() -> Self {
        Self {
            version: xlsx_graft_core::version(),
        }
    }
}

impl Report for VersionReport {
    fn write_text(&self, out: &mut dyn Write) -> io::Result<()> {
        writeln!(out, "xlsx-graft {}", self.version)
    }
}
