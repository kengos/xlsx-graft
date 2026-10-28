//! [`ReadError`]: the failures of the host that reading a workbook's parts
//! can end in.

use thiserror::Error;
use xlsx_graft_package::PackageError;
use xlsx_graft_spool::SpoolError;

/// A failure of the host, not of the workbook. Every problem in the workbook
/// is a diagnostic instead, so an `Err` never stands for one.
#[derive(Debug, Error)]
pub enum ReadError {
    /// Reading or seeking the package failed.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// The spool the shared strings spill to failed, or memory for them
    /// could not be allocated.
    #[error(transparent)]
    Spool(#[from] SpoolError),
}
