//! `layout`: each workbook of a folder (`fixtures/`) written again with the
//! layout of its first sheet and the visibility of its sheets edited by
//! `xlsx-graft-write`'s `SheetEdit` and `StructureEdit`, for Excel to open
//! (`make excel:check`).
//!
//! Each `<stem>.<ext>` gives the workbook of the same name in the folder
//! written to. On the first sheet that has a part (a sheet whose part is
//! missing is passed over), when that is a worksheet that can be read
//! (`sheet.rs` says what is set):
//!
//! - cells `AA1000:AB1001` merged;
//! - columns B to D 20 wide, and the middle column of the first `col` that
//!   covers two columns or more 30 wide (so that `col` is cut), or column F
//!   30 wide when the sheet has no `col` that covers two columns or more. A
//!   `col` of another namespace may be chosen; it is then not cut, since
//!   `SheetEdit` writes it unchanged and writes a new `col` instead;
//! - row 1 30 points high and row 500 24 points high (a row the sheet lacks
//!   is written);
//! - the first row and the first column frozen;
//! - the sheet protected with the password `xlsx-graft` and the default
//!   permissions.
//!
//! On the workbook (`visibility.rs`): with two sheets or more, the last
//! sheet is the active one; with three or more, the second is hidden; with
//! four or more, the third is very hidden.
//!
//! An edit the engine refuses for a workbook is not made and the output says
//! so: a merge that overlaps a range or a table of the sheet is made
//! elsewhere (`sheet.rs`), a state or an active sheet the workbook does not
//! allow (a sheet that is already hidden, the last visible sheet) is left
//! as it is, and a first sheet that is not a worksheet, or that cannot be
//! read, is not edited (the workbook is still written, with the sheets'
//! visibility). The output has the path of each workbook written and, for
//! each edit not made as said, an indented line. A workbook the engine
//! refuses or whose package cannot be read is skipped: the output says
//! `skipped`, its path and why, and the rest are written.

mod sheet;
mod visibility;

use crate::FixturesGenError;
use crate::repack::{Refusal, codes, io_error, workbooks};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package, PartName};
use xlsx_graft_read::{ReadLimits, SheetKind, Workbook, WorkbookPart};
use xlsx_graft_spool::TempFiles;
use xlsx_graft_write::sheet::SheetEdit;
use xlsx_graft_write::structure::StructureEdit;
use xlsx_graft_write::{PackageEdit, WriteLimits, write_file_atomically};

/// Runs `layout` for `args`, the two directories that follow it: the one
/// the workbooks are read from and the one they are written to, which is
/// created if it does not exist.
pub(crate) fn run(args: &[OsString], out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let [from, to] = args else {
        return Err(FixturesGenError::Usage);
    };
    let to = Path::new(to);
    fs::create_dir_all(to).map_err(io_error(to))?;
    for source in workbooks(Path::new(from))? {
        let target = to.join(source.file_name().unwrap_or_default());
        let text = match write(&source, &target) {
            Ok(notes) => {
                let lines = notes.iter().map(|note| format!("  {note}\n"));
                std::iter::once(format!("{}\n", target.display()))
                    .chain(lines)
                    .collect()
            }
            Err(Refusal(detail)) => format!("skipped {}: {detail}\n", source.display()),
        };
        out.write_all(text.as_bytes())
            .map_err(FixturesGenError::Output)?;
    }
    Ok(())
}

/// Writes the workbook `source` to `target`; the notes are what was not
/// edited as the module says.
fn write(source: &Path, target: &Path) -> Result<Vec<String>, Refusal> {
    let file = File::open(source)?;
    give(file, target, &mut Diagnostics::new())
}

/// [`write`] for the workbook read from `file`, pushing to `diagnostics`.
fn give(file: File, target: &Path, diagnostics: &mut Diagnostics) -> Result<Vec<String>, Refusal> {
    let limits = WriteLimits::default();
    let Some(mut package) = Package::open(file, &Limits::default(), diagnostics)? else {
        return Err(Refusal(codes(diagnostics)));
    };
    let Some(mut structure) = StructureEdit::open(&mut package, &limits, diagnostics)? else {
        return Err(Refusal(codes(diagnostics)));
    };
    // `StructureEdit::open` read the workbook part, so `find` and `read`
    // find it again.
    let workbook = Workbook::find(&package, &mut Diagnostics::new());
    let read = workbook.map(|workbook| {
        WorkbookPart::read(
            &mut package,
            &workbook,
            &ReadLimits::default(),
            &mut Diagnostics::new(),
        )
    });
    let book = read.transpose()?.flatten();
    let sheets = book.iter().flat_map(|book| book.sheets());
    let names: Vec<String> = sheets
        .clone()
        .map(|sheet| sheet.name().to_owned())
        .collect();
    let first = sheets.filter_map(|sheet| sheet.part()).next();
    let first = first.filter(|part| part.kind() == SheetKind::Worksheet);
    let first = first.and_then(|part| package.parts().get(part.index()));
    let first: Option<PartName> = first.map(|part| part.name().clone());
    let mut notes = visibility::change(&mut structure, &names);
    let mut sheet = None;
    let wide = (first.as_ref()).map_or(sheet::DEFAULT_COLUMN, |part| {
        sheet::wide_column(&mut package, part)
    });
    if let Some(part) = &first {
        let mut opened = Diagnostics::new();
        sheet = SheetEdit::open(&mut package, part, &limits, &mut opened)?;
        if sheet.is_none() {
            notes.push(format!("did not edit the first sheet: {}", codes(&opened)));
        }
    } else {
        notes.push("did not edit the first sheet: it is not a worksheet".to_owned());
    }
    let mut package_edit = PackageEdit::new(package);
    // `StructureEdit::apply` first, as `SheetEdit::apply` says.
    structure.apply(&mut package_edit)?;
    if let Some(mut edit) = sheet {
        notes.extend(sheet::edit(&mut edit, wide)?);
        edit.apply(&mut package_edit)?;
    }
    let written = write_file_atomically(target, |file| {
        let finished = package_edit.finish(file, &mut TempFiles::new(), &limits, diagnostics);
        Ok(finished?.map(|_| ()))
    })?;
    if written.is_none() {
        return Err(Refusal(codes(diagnostics)));
    }
    Ok(notes)
}

#[cfg(test)]
mod tests;
