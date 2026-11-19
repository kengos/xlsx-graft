//! `BookSpec`: which workbook to write (its kind and its rows), as the
//! command line names it.

use crate::{FixturesGenError, MAX_ROWS, write_rows_book, write_shared_book};
use rust_xlsxwriter::XlsxError;
use std::ffi::OsStr;
use std::path::Path;

/// Which workbook [`BookSpec`] asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// [`write_rows_book`]: strings written in their cells.
    Rows,
    /// [`write_shared_book`]: strings all different, in the shared strings
    /// table.
    Shared,
}

impl Kind {
    /// The word that starts the book's name on the command line and its
    /// file name.
    fn prefix(self) -> &'static str {
        match self {
            Self::Rows => "rows",
            Self::Shared => "shared",
        }
    }
}

/// A workbook to write: its [`Kind`] and its number of data rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookSpec {
    /// Which workbook to write.
    pub kind: Kind,
    /// The data row count, from 1 to [`MAX_ROWS`].
    pub rows: u32,
}

/// The workbooks written when the command line names none: 100,000 and
/// 1,000,000 rows of each kind, `rows` first.
pub const DEFAULT_BOOKS: [BookSpec; 4] = [
    BookSpec {
        kind: Kind::Rows,
        rows: 100_000,
    },
    BookSpec {
        kind: Kind::Rows,
        rows: 1_000_000,
    },
    BookSpec {
        kind: Kind::Shared,
        rows: 100_000,
    },
    BookSpec {
        kind: Kind::Shared,
        rows: 1_000_000,
    },
];

impl BookSpec {
    /// The file name of the workbook: `<prefix>-<rows>.xlsx`, such as
    /// `shared-100000.xlsx`.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}-{}.xlsx", self.kind.prefix(), self.rows)
    }

    /// Writes the workbook to `path` with [`write_rows_book`] or
    /// [`write_shared_book`], by its kind.
    ///
    /// # Errors
    ///
    /// [`XlsxError`], as the function called gives it.
    pub fn write(&self, path: &Path) -> Result<(), XlsxError> {
        match self.kind {
            Kind::Rows => write_rows_book(path, self.rows),
            Kind::Shared => write_shared_book(path, self.rows),
        }
    }

    /// The workbook `arg` names: `rows-<n>` or `shared-<n>`, with `<n>` a
    /// decimal number from 1 to [`MAX_ROWS`].
    ///
    /// # Errors
    ///
    /// [`FixturesGenError::Book`] for any other argument.
    pub fn parse(arg: &OsStr) -> Result<Self, FixturesGenError> {
        let spec = arg.to_str().and_then(|text| {
            let (kind, count) = [Kind::Rows, Kind::Shared].into_iter().find_map(|kind| {
                let count = text.strip_prefix(kind.prefix())?.strip_prefix('-')?;
                Some((kind, count))
            })?;
            let all_digits = count.bytes().all(|byte| byte.is_ascii_digit());
            let rows = count.parse::<u32>().ok().filter(|_| all_digits)?;
            (1..=MAX_ROWS)
                .contains(&rows)
                .then_some(Self { kind, rows })
        });
        spec.ok_or_else(|| FixturesGenError::Book(arg.to_string_lossy().into_owned()))
    }
}
