//! xlsx-graft fixtures-gen: writes large workbooks, too large to commit,
//! for the reader's benchmarks (`make bench:compare`, `make bench:cli`,
//! bench/ruby/) and, later, tests on large workbooks. They are written with
//! rust_xlsxwriter, a writer other than xlsx-graft, so that the reader can
//! be checked against workbooks the xlsx-graft writer did not write.
//!
//! `make fixtures:generate` runs the binary as
//! `xlsx-graft-fixtures-gen <dir>`, which writes [`DEFAULT_BOOKS`] into
//! `<dir>` (`fixtures/generated/`, which git ignores: the workbooks are
//! too large to commit): `rows-100000.xlsx`, `rows-1000000.xlsx`,
//! `shared-100000.xlsx` and `shared-1000000.xlsx`.
//! `xlsx-graft-fixtures-gen <dir> <book>...` writes the workbooks given
//! instead, each `rows-<n>` ([`write_rows_book`]: strings in their cells)
//! or `shared-<n>` ([`write_shared_book`]: strings all different, in the
//! shared strings table), with `<n>` data rows.
//!
//! `xlsx-graft-fixtures-gen repack <fixtures-dir> <dir>` is another
//! command (`make fixtures:repack`): it writes each workbook of
//! `<fixtures-dir>` again with the engine's writer, to see whether Excel
//! opens what the writer makes.
//!
//! `xlsx-graft-fixtures-gen cells <dir>` is the third command
//! (`make fixtures:cells`): it writes `cells-1900.xlsx`,
//! `cells-1900-strict.xlsx` and `cells-1904.xlsx` into `<dir>`, workbooks
//! of cell values (strings, numbers, booleans, error values, dates and
//! times, shared strings) written with the engine's cell writer, to see
//! whether Excel opens what the writer makes.
//!
//! The binary prints the path of each workbook once it is written, and
//! exits 0. It exits 2 when the arguments are not a directory followed by
//! books of that form with `<n>` from 1 to [`MAX_ROWS`], when `repack` is not
//! followed by two directories, or when `cells` is not followed by one
//! directory, and 1 when a workbook cannot be written or its path cannot be
//! printed ([`FixturesGenError::exit_status`]).

mod book;
mod cells;
mod error;
mod repack;
mod shared;
mod spec;

pub use book::{ROWS_HEADER, write_rows_book};
pub use error::FixturesGenError;
pub use shared::{SHARED_HEADER, write_shared_book};
pub use spec::{BookSpec, DEFAULT_BOOKS, Kind};

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The largest data row count a workbook may have: the 1,048,576 rows of
/// an Excel worksheet, which rust_xlsxwriter writes up to, less the header
/// row.
pub const MAX_ROWS: u32 = 1_048_575;

/// Writes the workbooks `args` asks for, as the binary does: `args` is the
/// command line after the program name, a directory followed by books
/// (`rows-<n>` or `shared-<n>`, see [`BookSpec::parse`]), and the
/// directory is created if it does not exist. Each workbook is written to
/// `<file name>.part` in that directory, then renamed to its file name
/// ([`BookSpec::file_name`]), so a write that stops part way leaves a
/// workbook already under that name as it was. Once renamed, the
/// workbook's path is written to `out` as a line.
///
/// When the first argument is `repack`, the command is `repack`'s
/// instead: two directories follow it, and a directory named `repack` is
/// written `./repack`. When it is `cells`, one directory follows it, and
/// a directory named `cells` is written `./cells`.
///
/// # Errors
///
/// [`FixturesGenError`]: the arguments are not a directory followed by
/// books, the directory cannot be created, a workbook cannot be written
/// or renamed, or writing a line to `out` fails.
pub fn run(args: &[OsString], out: &mut dyn Write) -> Result<(), FixturesGenError> {
    if let Some((command, rest)) = args.split_first()
        && command == "repack"
    {
        return repack::run(rest, out);
    }
    if let Some((command, rest)) = args.split_first()
        && command == "cells"
    {
        return cells::run(rest, out);
    }
    let (dir, books) = parse(args)?;
    fs::create_dir_all(&dir).map_err(|source| FixturesGenError::Io {
        path: dir.clone(),
        source,
    })?;
    for book in books {
        let path = dir.join(book.file_name());
        let part = dir.join(format!("{}.part", book.file_name()));
        book.write(&part).map_err(|source| FixturesGenError::Xlsx {
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

/// The directory and the books `args` gives: [`DEFAULT_BOOKS`] when it
/// gives a directory alone.
fn parse(args: &[OsString]) -> Result<(PathBuf, Vec<BookSpec>), FixturesGenError> {
    let (dir, names) = args.split_first().ok_or(FixturesGenError::Usage)?;
    let mut books = names
        .iter()
        .map(|name| BookSpec::parse(name))
        .collect::<Result<Vec<BookSpec>, FixturesGenError>>()?;
    if books.is_empty() {
        books = DEFAULT_BOOKS.to_vec();
    }
    Ok((Path::new(dir).to_path_buf(), books))
}

#[cfg(test)]
mod tests;
