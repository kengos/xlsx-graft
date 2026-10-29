//! `CliError`: why a run failed. [`exit_status`](crate::exit_status) prints
//! it to stderr and makes it exit 1.

use std::io;
use std::path::PathBuf;
use thiserror::Error;
use xlsx_graft_read::ReadError;

/// Why a run failed on the host. A bad command line never reaches here:
/// clap reports it and exits 2. Nor does a problem in a workbook, which is
/// a diagnostic in the output.
#[derive(Debug, Error)]
pub enum CliError {
    /// The workbook's file would not open.
    #[error("failed to open {}: {source}", path.display())]
    Open {
        /// The path given.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// Reading the workbook's file failed, or the temporary file its shared
    /// strings spill to did.
    #[error("failed to read the workbook: {0}")]
    Read(#[from] ReadError),
    /// The output would not serialize to JSON.
    #[error("failed to serialize the output: {0}")]
    Serialize(#[from] serde_json::Error),
    /// stdout or stderr refused the output (a closed pipe, a full disk).
    #[error("failed to write the output: {0}")]
    Write(io::Error),
}

#[cfg(test)]
mod tests;
