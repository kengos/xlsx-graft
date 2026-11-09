//! xlsx-graft fixtures-gen: writes large workbooks, too large to commit,
//! for the reader's tests and benchmarks on large workbooks (none reads
//! them yet). They are written with rust_xlsxwriter, a writer other than
//! xlsx-graft, so that the reader can be checked against workbooks the
//! xlsx-graft writer did not write.
//!
//! `make fixtures:generate` runs the binary as
//! `xlsx-graft-fixtures-gen <dir>`, which writes
//! `rows-100000.xlsx` and `rows-1000000.xlsx` into `<dir>`
//! (`fixtures/generated/`, which git ignores: the workbooks are too large
//! to commit). `xlsx-graft-fixtures-gen <dir> <rows>...` writes one
//! workbook of each row count given instead. [`write_book`] says what a
//! workbook holds.
//!
//! The binary prints the path of each workbook once it is written, and
//! exits 0. It exits 2 when the arguments are not a directory followed by
//! row counts from 1 to [`MAX_ROWS`], and 1 when a workbook cannot be
//! written or its path cannot be printed
//! ([`FixturesGenError::exit_status`]).

mod book;
mod error;

pub use book::{HEADER, write_book};
pub use error::FixturesGenError;

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The data row counts of the workbooks written when the arguments give
/// none: a hundred thousand and a million.
pub const DEFAULT_ROWS: [u32; 2] = [100_000, 1_000_000];

/// The largest data row count a workbook may have: the 1,048,576 rows of
/// an Excel worksheet, which rust_xlsxwriter writes up to, less the header
/// row.
pub const MAX_ROWS: u32 = 1_048_575;

/// Writes the workbooks `args` asks for, as the binary does: `args` is the
/// command line after the program name, a directory followed by row
/// counts, and the directory is created if it does not exist. Each
/// workbook is written to `rows-<rows>.xlsx.part` in that directory, then
/// renamed to `rows-<rows>.xlsx` ([`file_name`]), so a write that stops
/// part way leaves a workbook already under that name as it was. Once
/// renamed, the workbook's path is written to `out` as a line.
///
/// # Errors
///
/// [`FixturesGenError`]: the arguments are not a directory followed by
/// row counts, the directory cannot be created, a workbook cannot be
/// written or renamed, or writing a line to `out` fails.
pub fn run(args: &[OsString], out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let (dir, counts) = parse(args)?;
    fs::create_dir_all(&dir).map_err(|source| FixturesGenError::Io {
        path: dir.clone(),
        source,
    })?;
    for rows in counts {
        let path = dir.join(file_name(rows));
        let part = dir.join(format!("{}.part", file_name(rows)));
        write_book(&part, rows).map_err(|source| FixturesGenError::Xlsx {
            path: part.clone(),
            source,
        })?;
        fs::rename(&part, &path).map_err(|source| FixturesGenError::Io {
            path: path.clone(),
            source,
        })?;
        writeln!(out, "{}", path.display()).map_err(FixturesGenError::Output)?;
    }
    Ok(())
}

/// The file name of the workbook with `rows` data rows:
/// `rows-<rows>.xlsx`.
#[must_use]
pub fn file_name(rows: u32) -> String {
    format!("rows-{rows}.xlsx")
}

/// The directory and the data row counts `args` gives: [`DEFAULT_ROWS`]
/// when it gives a directory alone.
fn parse(args: &[OsString]) -> Result<(PathBuf, Vec<u32>), FixturesGenError> {
    let (dir, counts) = args.split_first().ok_or(FixturesGenError::Usage)?;
    let mut rows = counts
        .iter()
        .map(|count| row_count(count))
        .collect::<Result<Vec<u32>, FixturesGenError>>()?;
    if rows.is_empty() {
        rows = DEFAULT_ROWS.to_vec();
    }
    Ok((Path::new(dir).to_path_buf(), rows))
}

/// The row count `arg` gives, a decimal number from 1 to [`MAX_ROWS`].
fn row_count(arg: &OsStr) -> Result<u32, FixturesGenError> {
    arg.to_str()
        .and_then(|text| text.parse::<u32>().ok())
        .filter(|rows| (1..=MAX_ROWS).contains(rows))
        .ok_or_else(|| FixturesGenError::RowCount(arg.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests;
