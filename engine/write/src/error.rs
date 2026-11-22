//! [`WriteError`]: the failures that writing a zip can end in.

use std::io;
use thiserror::Error;
use xlsx_graft_package::PackageError;
use xlsx_graft_spool::SpoolError;

/// A failure of the host or a name that no zip can hold, never a problem in
/// a workbook (those are diagnostics).
#[derive(Debug, Error)]
pub enum WriteError {
    /// Writing the zip, or reading the archive an entry is copied from,
    /// failed.
    #[error("writing the zip failed: {0}")]
    Io(#[source] io::Error),
    /// The scratch storage the deflated content is held in failed.
    #[error("the scratch storage of the zip failed: {0}")]
    Spool(#[from] SpoolError),
    /// An earlier call failed or left a partial entry, so what was written
    /// is not a zip, and this call writes nothing.
    #[error("an earlier call failed or left a partial entry: the output is not a zip")]
    Unusable,
    /// An entry name is longer than the 65,535 bytes a zip name field holds
    /// (APPNOTE 6.3.10 §4.4.10).
    #[error("an entry name of {length} bytes is longer than a zip holds")]
    EntryNameTooLong {
        /// The length of the name in bytes.
        length: usize,
    },
}

impl From<PackageError> for WriteError {
    fn from(error: PackageError) -> Self {
        let PackageError::Io(error) = error;
        Self::from(error)
    }
}

impl From<io::Error> for WriteError {
    /// The [`WriteError`] that `error` carries, when it carries one (as the
    /// errors of [`DeflateWriter`](crate::DeflateWriter)'s `write` do); any
    /// other [`io::Error`] as [`WriteError::Io`].
    fn from(error: io::Error) -> Self {
        match error.downcast::<Self>() {
            Ok(carried) => carried,
            Err(error) => Self::Io(error),
        }
    }
}
