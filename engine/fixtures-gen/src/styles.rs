//! `styles`: each workbook of a folder (`fixtures/`) written again with the
//! cell formats of `xlsx-graft-style` given to the cells A1 and on of its
//! first worksheet, for Excel to open (`make excel:check`).
//!
//! Each `<name>.xlsx` or `<name>.xlsm` gives a workbook of the same name in
//! the folder written to, with:
//!
//! - its styles part written by a [`StylesEdit`], which adds a cell format
//!   for each style of [`catalog::entries`] (the 18 of them, to the cell
//!   format 0, which a cell without an `s` has) and nothing else;
//! - its first worksheet written by a [`SheetCells`], which gives cell A`n`
//!   the `n`th cell format and a value that shows it (the number
//!   45292.75 for a number format, a string that names the style
//!   otherwise);
//! - every other entry copied as it is.
//!
//! A workbook with no worksheet (one whose sheets are all not worksheets,
//! such as chartsheets only), or no styles part, gives nothing. A workbook
//! whose package or `workbook.xml` cannot be read (no `officeDocument`
//! relationship, or a `workbook.xml` that is not XML or that the schema does
//! not allow, such as one whose `<sheets>` has no `<sheet>`), or one with two
//! `styles` relationships to different parts, is refused with the codes of
//! the diagnostics, as a package that cannot be opened is.

mod catalog;
mod sheet;

use crate::FixturesGenError;
use crate::repack::{codes, io_error, workbooks};
use sheet::SheetCells;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package, Presence};
use xlsx_graft_read::{ReadLimits, STYLES, SheetKind, Workbook, WorkbookPart};
use xlsx_graft_spool::TempFiles;
use xlsx_graft_style::{StyleLimits, StylesEdit};
use xlsx_graft_write::{PackageEdit, WriteLimits, write_file_atomically};

/// Runs `styles` for `args`, the two directories that follow it: the one
/// the workbooks are read from and the one they are written to, which is
/// created if it does not exist. Each workbook written is a line of `out`.
pub(crate) fn run(args: &[OsString], out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let [from, to] = args else {
        return Err(FixturesGenError::Usage);
    };
    let to = Path::new(to);
    fs::create_dir_all(to).map_err(io_error(to))?;
    for source in workbooks(Path::new(from))? {
        let target = to.join(source.file_name().unwrap_or_default());
        if write(&source, &target)? {
            writeln!(out, "{}", target.display()).map_err(FixturesGenError::Output)?;
        }
    }
    Ok(())
}

/// Why a workbook is refused, as text: the error of any step of [`give`],
/// or the diagnostics the engine pushed.
struct Refusal(String);

impl<E: Display> From<E> for Refusal {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

/// Writes the workbook `source` to `target` as the module says. `Ok(false)`
/// when it has no worksheet or no styles part and nothing was written; an
/// `Err` when it is refused, its package or `workbook.xml` that cannot be
/// read included.
fn write(source: &Path, target: &Path) -> Result<bool, FixturesGenError> {
    let file = File::open(source).map_err(io_error(source))?;
    let mut diagnostics = Diagnostics::new();
    let given = give(file, target, &mut diagnostics);
    given.map_err(|Refusal(detail)| FixturesGenError::Repack {
        path: source.to_owned(),
        detail,
    })
}

/// [`write`] for the workbook read from `file`, pushing to `diagnostics`.
fn give(file: File, target: &Path, diagnostics: &mut Diagnostics) -> Result<bool, Refusal> {
    let Some(mut package) = Package::open(file, &Limits::default(), diagnostics)? else {
        return Err(Refusal(codes(diagnostics)));
    };
    let limits = ReadLimits::default();
    let Some(workbook) = Workbook::find(&package, diagnostics) else {
        return Err(Refusal(codes(diagnostics)));
    };
    let Some(part) = WorkbookPart::read(&mut package, &workbook, &limits, diagnostics)? else {
        return Err(Refusal(codes(diagnostics)));
    };
    let mut sheets = part.sheets().iter().filter_map(|sheet| sheet.part());
    let sheet = sheets.find(|part| part.kind() == SheetKind::Worksheet);
    let styles = workbook.related(&package, &STYLES, diagnostics);
    if styles == Presence::Unknown {
        return Err(Refusal(codes(diagnostics)));
    }
    let found = match (sheet, styles) {
        (Some(sheet), Presence::Part(styles)) => Some((sheet.index(), styles)),
        _ => None,
    };
    let parts = package.parts();
    let names = found.and_then(|(sheet, styles)| {
        let sheet_name = parts.get(sheet)?.name().clone();
        Some((sheet_name, parts.get(styles)?.name().clone(), styles))
    });
    let Some((sheet_name, styles_name, styles)) = names else {
        return Ok(false);
    };
    let Some(bytes) = package.read_xml_part(styles, limits.xml.max_bytes, diagnostics)? else {
        return Err(Refusal(codes(diagnostics)));
    };
    let mut edit = StylesEdit::read(&bytes, &StyleLimits::default())?;
    let mut cells = Vec::new();
    for entry in catalog::entries() {
        cells.push((edit.cell_format(0, &entry.style)?, entry.value));
    }
    let mut package_edit = PackageEdit::new(package);
    package_edit.replace(&styles_name, Box::new(edit))?;
    package_edit.replace(&sheet_name, Box::new(SheetCells::new(cells)))?;
    let written = write_file_atomically(target, |file| {
        let finished = package_edit.finish(
            file,
            &mut TempFiles::new(),
            &WriteLimits::default(),
            diagnostics,
        );
        Ok(finished?.map(|_| ()))
    })?;
    if written.is_none() {
        return Err(Refusal(codes(diagnostics)));
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
