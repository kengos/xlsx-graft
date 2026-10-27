//! [`SpoolError`]: the failures of the host that spooling can end in.

use thiserror::Error;

/// A failure of the host, never of a workbook: the storage under a spool
/// failed, or memory could not be allocated.
#[derive(Debug, Error)]
pub enum SpoolError {
    /// Creating, writing or reading the storage failed, or allocating the
    /// memory a buffer grows into (`std::io::ErrorKind::OutOfMemory`).
    #[error("the spool's storage failed: {0}")]
    Io(#[from] std::io::Error),
}
