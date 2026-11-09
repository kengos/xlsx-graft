//! `FixturesGenError`: why the workbooks were not all written. `main.rs`
//! prints it to stderr and exits with [`FixturesGenError::exit_status`].

use crate::MAX_ROWS;
use rust_xlsxwriter::XlsxError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Why [`run`](crate::run) stopped. The workbooks written before it
/// stopped stay written.
#[derive(Debug, Error)]
pub enum FixturesGenError {
    /// The command line gives no directory.
    #[error("usage: xlsx-graft-fixtures-gen <dir> [<rows>...]")]
    Usage,
    /// A row count on the command line is not a decimal number from 1 to
    /// [`MAX_ROWS`]; the field is the argument as given.
    #[error("{0:?} is not a row count from 1 to {MAX_ROWS}")]
    RowCount(String),
    /// The directory could not be created, or a workbook written could not
    /// be renamed to its file name; `path` is the directory or the
    /// workbook's path.
    #[error("{}: {source}", path.display())]
    Io {
        /// The directory, or the workbook's path.
        path: PathBuf,
        /// What the file system said.
        source: io::Error,
    },
    /// rust_xlsxwriter could not write the workbook to `path`.
    #[error("{}: {source}", path.display())]
    Xlsx {
        /// The path the workbook was being written to.
        path: PathBuf,
        /// What rust_xlsxwriter said.
        source: XlsxError,
    },
    /// Writing the line that names a written workbook to `out` (standard
    /// output in the binary) failed.
    #[error("writing to the output failed: {0}")]
    Output(io::Error),
}

impl FixturesGenError {
    /// The binary's exit status for this error: 2 for a command line it
    /// cannot read ([`Usage`](Self::Usage), [`RowCount`](Self::RowCount)),
    /// 1 for a workbook it could not write or report.
    #[must_use]
    pub fn exit_status(&self) -> u8 {
        match self {
            Self::Usage | Self::RowCount(_) => 2,
            Self::Io { .. } | Self::Xlsx { .. } | Self::Output(_) => 1,
        }
    }
}
