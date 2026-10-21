//! `CliError`: why a run failed. `main.rs` prints it to stderr and exits 1.

use std::io;
use thiserror::Error;

/// Why a run failed. A bad command line never reaches here: clap reports it
/// and exits 2.
#[derive(Debug, Error)]
pub enum CliError {
    /// The output would not serialize to JSON.
    #[error("failed to serialize the output: {0}")]
    Serialize(#[from] serde_json::Error),
    /// stdout refused the output (a closed pipe, a full disk).
    #[error("failed to write the output: {0}")]
    Write(io::Error),
}

#[cfg(test)]
mod tests;
