//! `structure`: each workbook of a folder (`fixtures/`) written again with
//! its sheets changed by `xlsx-graft-write`'s `StructureEdit`, for Excel to
//! open (`make excel:check`).
//!
//! Each `<stem>.<ext>` (`xlsx` or `xlsm`) gives two workbooks of the folder
//! written to, with the `<ext>` kept ([`Variant`]):
//!
//! - `<stem>-rename.<ext>`: its first sheet renamed to `Graft 名前 1`,
//!   a sheet `Graft added` added at position 0, its last sheet moved to
//!   position 1 and the workbook's defined name `graft_name` set to
//!   `'Graft 名前 1'!$A$1`;
//! - `<stem>-delete.<ext>`: a sheet `Graft added` added after the last
//!   sheet and the first sheet deleted.
//!
//! The formulas of the other parts follow the rename and the delete as
//! `StructureEdit::apply` does, and every other entry is copied as it is.
//! For each workbook written, the output has the path and then a line for
//! each reference `apply` kept as it was (a `StructureReport`: reason, part,
//! place, at, sheet). A workbook (or one of its two variants) the edit refuses,
//! or whose package cannot be read, is skipped: the output says `skipped`,
//! its path and why, and the rest are written.

mod variant;

use crate::FixturesGenError;
use crate::repack::{Refusal, codes, io_error, workbooks};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use variant::Variant;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::TempFiles;
use xlsx_graft_write::structure::model::Workbook;
use xlsx_graft_write::structure::{StructureEdit, StructureReport};
use xlsx_graft_write::{PackageEdit, WriteLimits, write_file_atomically};

/// Runs `structure` for `args`, the two directories that follow it: the one
/// the workbooks are read from and the one they are written to, which is
/// created if it does not exist.
pub(crate) fn run(args: &[OsString], out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let [from, to] = args else {
        return Err(FixturesGenError::Usage);
    };
    let to = Path::new(to);
    fs::create_dir_all(to).map_err(io_error(to))?;
    for source in workbooks(Path::new(from))? {
        let stem = source.file_stem().unwrap_or_default().to_string_lossy();
        let ext = source.extension().unwrap_or_default().to_string_lossy();
        for variant in Variant::ALL {
            let target = to.join(format!("{stem}-{}.{ext}", variant.suffix()));
            let text = match write(&source, &target, variant) {
                Ok(report) => described(&target, &report),
                Err(Refusal(detail)) => format!(
                    "skipped {} ({}): {detail}\n",
                    source.display(),
                    variant.suffix()
                ),
            };
            out.write_all(text.as_bytes())
                .map_err(FixturesGenError::Output)?;
        }
    }
    Ok(())
}

/// The path of the workbook written, then a line for each reference of
/// `report` (with the table, for a reference to a table removed), and one
/// for the references the report did not hold, when there are any.
fn described(target: &Path, report: &StructureReport) -> String {
    let kept = report.kept().iter().map(|kept| {
        let table = (kept.table()).map_or(String::new(), |table| format!(", table {table:?}"));
        format!(
            "  kept {:?}: {} {} at {:?}, sheet {:?}{table}\n",
            kept.reason(),
            kept.part(),
            kept.place(),
            kept.at(),
            kept.sheet(),
        )
    });
    let omitted = (report.omitted() > 0).then(|| format!("  omitted {} more\n", report.omitted()));
    std::iter::once(format!("{}\n", target.display()))
        .chain(kept)
        .chain(omitted)
        .collect()
}

/// Writes the workbook `source` to `target` with `variant`'s edit.
fn write(source: &Path, target: &Path, variant: Variant) -> Result<StructureReport, Refusal> {
    let file = File::open(source)?;
    give(file, target, variant, &mut Diagnostics::new())
}

/// [`write`] for the workbook read from `file`, pushing to `diagnostics`.
fn give(
    file: File,
    target: &Path,
    variant: Variant,
    diagnostics: &mut Diagnostics,
) -> Result<StructureReport, Refusal> {
    let limits = WriteLimits::default();
    let Some(mut package) = Package::open(file, &Limits::default(), diagnostics)? else {
        return Err(Refusal(codes(diagnostics)));
    };
    let Some(mut edit) = StructureEdit::open(&mut package, &limits, diagnostics)? else {
        return Err(Refusal(codes(diagnostics)));
    };
    // `Workbook::read` succeeds here because `StructureEdit::open` already
    // read the workbook part.
    let book = Workbook::read(&mut package, &limits, &mut Diagnostics::new())?;
    let names: Vec<String> = (book.iter().flat_map(|book| book.sheets()))
        .map(|sheet| sheet.name().to_owned())
        .collect();
    variant.apply(&mut edit, &names)?;
    let mut package_edit = PackageEdit::new(package);
    let report = edit.apply(&mut package_edit)?;
    let written = write_file_atomically(target, |file| {
        let finished = package_edit.finish(file, &mut TempFiles::new(), &limits, diagnostics);
        Ok(finished?.map(|_| ()))
    })?;
    if written.is_none() {
        return Err(Refusal(codes(diagnostics)));
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
