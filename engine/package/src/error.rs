//! [`PackageError`]: the failures of the host that reading a package can end
//! in.

use thiserror::Error;

/// A failure of the host, not of the workbook. Every problem in the workbook
/// is a diagnostic instead, so an `Err` never stands for one.
#[derive(Debug, Error)]
pub enum PackageError {
    /// Reading or seeking the underlying reader failed.
    #[error("reading the package failed: {0}")]
    Io(#[from] std::io::Error),
}
