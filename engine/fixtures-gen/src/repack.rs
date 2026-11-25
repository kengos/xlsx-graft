//! `repack`: each workbook of a folder (`fixtures/`) written again with
//! the engine's writer, for Excel to open (`make excel:check`).
//!
//! Each `<name>.xlsx` or `<name>.xlsm` gives two workbooks in the folder
//! written to: `<name>.xlsx`, its package edited by nothing, so every
//! entry is copied as it is in a zip the writer lays out again; and
//! `<name>-app.xlsx`, the same with `docProps/app.xml` rewritten to name
//! `xlsx-graft` as the application, which is a part rewritten and the
//! others copied. A workbook with no `docProps/app.xml` gives the first
//! only.

mod application;

use crate::FixturesGenError;
use application::Application;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package, PartName};
use xlsx_graft_spool::TempFiles;
use xlsx_graft_write::{PackageEdit, WriteError, WriteLimits, write_file_atomically};

/// The part whose application is rewritten.
const APP: &str = "/docProps/app.xml";

/// Runs `repack` for `args`, the two directories that follow it: the one
/// the workbooks are read from and the one they are written to, which is
/// created if it does not exist. Each workbook written is a line of `out`.
pub(crate) fn run(args: &[OsString], out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let [from, to] = args else {
        return Err(FixturesGenError::Usage);
    };
    let to = Path::new(to);
    fs::create_dir_all(to).map_err(io_error(to))?;
    for path in workbooks(Path::new(from))? {
        let name = path.file_name().unwrap_or_default();
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let copy = Book {
            source: &path,
            target: to.join(name),
            rewrite: None,
        };
        let app = Book {
            target: to.join(format!("{stem}-app.{ext}")),
            rewrite: Some(APP),
            ..copy.clone()
        };
        for book in [copy, app] {
            if book.write()? {
                writeln!(out, "{}", book.target.display()).map_err(FixturesGenError::Output)?;
            }
        }
    }
    Ok(())
}

/// The error for `source`, a failure of the file system at `path`.
pub(crate) fn io_error(path: &Path) -> impl FnOnce(io::Error) -> FixturesGenError {
    let path = path.to_owned();
    move |source| FixturesGenError::Io { path, source }
}

/// The `.xlsx` and `.xlsm` files in `folder`, sorted by path.
pub(crate) fn workbooks(folder: &Path) -> Result<Vec<PathBuf>, FixturesGenError> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(folder).map_err(io_error(folder))? {
        let path = entry.map_err(io_error(folder))?.path();
        let ext = path.extension().and_then(OsStr::to_str);
        if ext.is_some_and(|e| e.eq_ignore_ascii_case("xlsx") || e.eq_ignore_ascii_case("xlsm")) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// A workbook to write: the package in the file `source`, to `target`.
#[derive(Clone)]
struct Book<'a> {
    source: &'a Path,
    target: PathBuf,
    /// The part whose application is rewritten, if one is.
    rewrite: Option<&'a str>,
}

impl Book<'_> {
    /// Writes the workbook to its target. `Ok(false)` when it has no
    /// `docProps/app.xml` to rewrite and nothing was written.
    fn write(&self) -> Result<bool, FixturesGenError> {
        let refused = |detail: String| FixturesGenError::Repack {
            path: self.source.to_owned(),
            detail,
        };
        let file = File::open(self.source).map_err(io_error(self.source))?;
        let mut diagnostics = Diagnostics::new();
        let package = Package::open(file, &Limits::default(), &mut diagnostics);
        let package = package.map_err(|error| refused(error.to_string()))?;
        let Some(package) = package else {
            return Err(refused(codes(&diagnostics)));
        };
        let mut edit = PackageEdit::new(package);
        if let Some(part) = self.rewrite {
            let name = PartName::parse(part).ok();
            let replaced = name.map(|name| edit.replace(&name, Box::new(Application)));
            match replaced {
                // The workbook has no such part, or the name is not one.
                None | Some(Err(WriteError::PartMissing { .. })) => return Ok(false),
                Some(Err(error)) => return Err(refused(error.to_string())),
                Some(Ok(())) => {}
            }
        }
        let written = write_file_atomically(&self.target, |file| {
            let finished = edit.finish(
                file,
                &mut TempFiles::new(),
                &WriteLimits::default(),
                &mut diagnostics,
            );
            Ok(finished?.map(|_| ()))
        });
        let written = written.map_err(|error| refused(error.to_string()))?;
        if written.is_none() {
            return Err(refused(codes(&diagnostics)));
        }
        Ok(true)
    }
}

/// Why a workbook is refused, as text: the error of any step of writing it,
/// or the diagnostics the engine pushed.
pub(crate) struct Refusal(pub(crate) String);

impl<E: Display> From<E> for Refusal {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

/// The codes of the diagnostics in `diagnostics`, as a sentence.
pub(crate) fn codes(diagnostics: &Diagnostics) -> String {
    let codes: Vec<String> = (diagnostics.records().iter())
        .map(|record| record.diagnostic.code().to_string())
        .collect();
    format!("the diagnostics are {}", codes.join(", "))
}

#[cfg(test)]
mod tests;
