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
    /// The command line gives no directory, `repack`, `styles` or `structure`
    /// is not followed by two directories, or `cells` is not followed by exactly
    /// one directory.
    #[error(
        "usage: xlsx-graft-fixtures-gen <dir> [rows-<n>|shared-<n>...] | repack <fixtures-dir> <dir> | cells <dir> | styles <fixtures-dir> <dir> | structure <fixtures-dir> <dir>"
    )]
    Usage,
    /// A book on the command line is not `rows-<n>` or `shared-<n>` with
    /// `<n>` a decimal number from 1 to [`MAX_ROWS`]; the field is the
    /// argument as given.
    #[error("{0:?} is not rows-<n> or shared-<n> with <n> from 1 to {MAX_ROWS}")]
    Book(String),
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
    /// `repack` or `styles` could not write the workbook at `path` again: it
    /// is not a package the engine reads, or it holds an entry that cannot be
    /// read (for `styles`, a styles part or a worksheet it refuses), or the
    /// engine failed to write it. `detail` says which.
    #[error("{}: {detail}", path.display())]
    Repack {
        /// The workbook that was being repacked or given styles.
        path: PathBuf,
        /// The diagnostics or the error, as text.
        detail: String,
    },
    /// `cells` could not write the workbook at `path`: its parts could not
    /// be written, or the zip or the file could not. `detail` says which.
    #[error("{}: {detail}", path.display())]
    Cells {
        /// The workbook that was being written.
        path: PathBuf,
        /// The error, as text.
        detail: String,
    },
    /// Writing the line that names a written workbook to `out` (standard
    /// output in the binary) failed.
    #[error("writing to the output failed: {0}")]
    Output(io::Error),
}

impl FixturesGenError {
    /// The binary's exit status for this error: 2 for a command line it
    /// cannot read ([`Usage`](Self::Usage), [`Book`](Self::Book)),
    /// 1 for a workbook it could not write or report.
    #[must_use]
    pub fn exit_status(&self) -> u8 {
        match self {
            Self::Usage | Self::Book(_) => 2,
            Self::Io { .. }
            | Self::Xlsx { .. }
            | Self::Repack { .. }
            | Self::Cells { .. }
            | Self::Output(_) => 1,
        }
    }
}
