//! `blank`: four workbooks made from the built-in blank workbook of
//! `xlsx-graft-write`'s `blank` module, for Excel to open
//! (`make excel:check`).
//!
//! - `blank.xlsx`: the built-in workbook as it is (`Defaults::default()`:
//!   Calibri 11, `defaultRowHeight="15"`), its one sheet `Sheet1` empty.
//! - `blank-created.xlsx`: the create procedure of that module with the same
//!   defaults: a first sheet `Sheet1` (the name of the built-in sheet, so
//!   the built-in sheet is renamed first) with a header row and rows of a
//!   string, a number and a boolean, then a second sheet `Data`, empty.
//! - `blank-font.xlsx`: the same procedure with the font Yu Gothic 11 alone:
//!   `defaultRowHeight="15" customHeight="1"`, which Excel is to show as rows
//!   15 points high rather than the height it computes from the font.
//! - `blank-row-height.xlsx`: the same procedure with Yu Gothic 11 and the row
//!   height 20: `defaultRowHeight="20" customHeight="1"`.
//!
//! The first sheet written by the procedure has the `sheetFormatPr` of the
//! defaults; the sheet `Data` is the empty worksheet `StructureEdit` writes.

#[cfg(test)]
mod tests;

use crate::FixturesGenError;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;
use xlsx_graft_cell::{CellValue, DateSystem};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::blank::{self, DefaultFont, DefaultFontError, Defaults};
use xlsx_graft_write::sheet::SheetFormat;
use xlsx_graft_write::structure::{StructureEdit, StructureError, check_sheet_name};
use xlsx_graft_write::{
    PackageEdit, PartWriter, Row, RowCell, RowsWriter, WriteError, WriteLimits,
    write_file_atomically,
};

/// A workbook written: its file name, whether the create procedure is run
/// on the built-in workbook (else it is written as it is), and the font and
/// the row height given to `Defaults::new`.
struct Book {
    name: &'static str,
    created: bool,
    font: Option<(&'static str, f64)>,
    row_height: Option<f64>,
}

/// The workbooks written, in order.
const BOOKS: [Book; 4] = [
    Book {
        name: "blank.xlsx",
        created: false,
        font: None,
        row_height: None,
    },
    Book {
        name: "blank-created.xlsx",
        created: true,
        font: None,
        row_height: None,
    },
    Book {
        name: "blank-font.xlsx",
        created: true,
        font: Some(("Yu Gothic", 11.0)),
        row_height: None,
    },
    Book {
        name: "blank-row-height.xlsx",
        created: true,
        font: Some(("Yu Gothic", 11.0)),
        row_height: Some(20.0),
    },
];

/// Runs `blank` for `args`, the one directory that follows it, created if it
/// does not exist. Each workbook written is a line of `out`.
pub(crate) fn run(args: &[OsString], out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let [dir] = args else {
        return Err(FixturesGenError::Usage);
    };
    let dir = Path::new(dir);
    fs::create_dir_all(dir).map_err(|source| FixturesGenError::Io {
        path: dir.to_owned(),
        source,
    })?;
    for book in BOOKS {
        let path = dir.join(book.name);
        let written = write_book(&path, &book);
        written.map_err(|error| FixturesGenError::Blank {
            path: path.clone(),
            detail: error.to_string(),
        })?;
        writeln!(out, "{}", path.display()).map_err(FixturesGenError::Output)?;
    }
    Ok(())
}

/// The font of `name` and `size`.
fn font((name, size): (&str, f64)) -> Result<DefaultFont, DefaultFontError> {
    DefaultFont::new(name, size)
}

/// Writes `book` to `path`, replacing the file there only once the whole
/// workbook is written.
fn write_book(path: &Path, book: &Book) -> Result<(), Box<dyn Error>> {
    let font = book.font.map(font).transpose()?;
    let defaults = Defaults::new(font, book.row_height)?;
    let limits = WriteLimits::default();
    let mut diagnostics = Diagnostics::new();
    let mut package = blank::package(&defaults)?;
    let mut structure = StructureEdit::open(&mut package, &limits, &mut diagnostics)?
        .ok_or("the built-in workbook is not read")?;
    if book.created {
        create(&mut structure, &defaults, "Sheet1")?;
    }
    let mut edit = PackageEdit::new(package);
    structure.apply(&mut edit)?;
    let written = write_file_atomically(path, |file| {
        let finished = edit.finish(file, &mut MemorySpools, &limits, &mut diagnostics);
        Ok(finished?.map(|_| ()))
    })?;
    written.ok_or("the workbook is not written")?;
    Ok(())
}

/// The create procedure of `blank`'s module documentation, for the first
/// sheet `first` and a further sheet `Data`.
fn create(
    structure: &mut StructureEdit<'_>,
    defaults: &Defaults,
    first: &str,
) -> Result<(), StructureError> {
    let mut built_in = blank::SHEET;
    if let Err(StructureError::SheetNameTaken { .. }) = check_sheet_name(first, [blank::SHEET]) {
        structure.rename_sheet(blank::SHEET, "Sheet2")?;
        built_in = "Sheet2";
    }
    let content = Content {
        format: defaults.sheet_format(),
    };
    structure.add_sheet(first, None, Some(Box::new(content)))?;
    structure.delete_sheet(built_in)?;
    structure.add_sheet("Data", None, None)
}

/// The first sheet: the `sheetFormatPr` of the defaults and a header row and
/// rows of a string, a number and a boolean.
struct Content {
    format: SheetFormat,
}

impl PartWriter for Content {
    fn write(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
        let mut spools = MemorySpools;
        let mut rows = RowsWriter::new(&mut spools, u64::MAX, DateSystem::Compatibility1900);
        let cells = |name, value| {
            [
                RowCell::new(1, None, CellValue::String(name)),
                RowCell::new(2, None, value),
            ]
        };
        rows.push(&Row::new(1, &cells("name", CellValue::String("value"))))?;
        rows.push(&Row::new(2, &cells("a string", CellValue::String("text"))))?;
        rows.push(&Row::new(3, &cells("a number", CellValue::Number(12.5))))?;
        rows.push(&Row::new(4, &cells("a boolean", CellValue::Bool(true))))?;
        let mut rows = rows.finish()?;
        out.write_all(
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
                "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
            )
            .as_bytes(),
        )?;
        self.format.write(out, "")?;
        out.write_all(b"<sheetData>")?;
        rows.write_sheet_data(out)?;
        out.write_all(b"</sheetData></worksheet>")?;
        Ok(())
    }
}
