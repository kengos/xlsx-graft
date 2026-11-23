//! `cells`: three workbooks of cell values written with the engine's cell
//! writer, for Excel to open (`make excel:check`): strings that need
//! `_xHHHH_` escapes or are cut, numbers at their limits, booleans, error
//! values, dates and times in each date system, and shared strings, each
//! in the cell next to a description of it.
//!
//! The workbooks are `cells-1900.xlsx` (no `workbookPr`, so the 1900 date
//! system that counts 1900-02-29), `cells-1900-strict.xlsx`
//! (`conformance="strict"`, the standard's 1900) and `cells-1904.xlsx`
//! (`date1904="1"`). A value the date system or a cell cannot hold is
//! written as the text `refused: <why>` in column B instead of the value.

mod parts;
mod rows;

#[cfg(test)]
mod tests;

use crate::FixturesGenError;
use parts::{CONTENT_TYPES, PACKAGE_RELS, SHEET_END, SHEET_START, STYLES, WORKBOOK_RELS, workbook};
use rows::{Row, SHARED, Sample, rows};
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::iter;
use std::path::Path;
use xlsx_graft_cell::{CellError, CellRef, CellValue, DateSystem, SharedStringsWriter, write_cell};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::{DeflateWriter, Deflated, ZipWriter, write_file_atomically};

/// The workbooks written: the file name and the date system of each.
const BOOKS: [(&str, DateSystem); 3] = [
    ("cells-1900.xlsx", DateSystem::Compatibility1900),
    ("cells-1900-strict.xlsx", DateSystem::Base1900),
    ("cells-1904.xlsx", DateSystem::Base1904),
];

/// Runs `cells` for `args`, the one directory that follows it, created if it
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
    for (name, system) in BOOKS {
        let path = dir.join(name);
        let written = write_book(&path, system);
        written.map_err(|error| FixturesGenError::Cells {
            path: path.clone(),
            detail: error.to_string(),
        })?;
        writeln!(out, "{}", path.display()).map_err(FixturesGenError::Output)?;
    }
    Ok(())
}

/// Writes the workbook in `system` to `path`, replacing the file there only
/// once the whole workbook is written.
fn write_book(path: &Path, system: DateSystem) -> Result<(), Box<dyn Error>> {
    let samples = rows(system)?;
    let parts = [
        ("[Content_Types].xml", text(CONTENT_TYPES)?),
        ("_rels/.rels", text(PACKAGE_RELS)?),
        ("xl/workbook.xml", text(&workbook(system))?),
        ("xl/_rels/workbook.xml.rels", text(WORKBOOK_RELS)?),
        ("xl/styles.xml", text(STYLES)?),
        (
            "xl/worksheets/sheet1.xml",
            deflate(|out| Ok(sheet(out, &samples, system)?))?,
        ),
        (
            "xl/sharedStrings.xml",
            deflate(|out| Ok(shared_strings(out)?))?,
        ),
    ];
    write_file_atomically(path, |out| {
        let mut zip = ZipWriter::new(out);
        for (name, deflated) in parts {
            zip.write_deflated(name.as_bytes(), deflated)?;
        }
        zip.finish()?;
        Ok(Some(()))
    })?;
    Ok(())
}

/// A part whose text is `content`, deflated without changing the text.
fn text(content: &str) -> Result<Deflated, Box<dyn Error>> {
    deflate(|out| Ok(out.write_all(content.as_bytes())?))
}

/// The part `write` writes, deflated in memory.
fn deflate(
    write: impl FnOnce(&mut dyn Write) -> Result<(), Box<dyn Error>>,
) -> Result<Deflated, Box<dyn Error>> {
    let mut spools = MemorySpools;
    let mut out = DeflateWriter::new(&mut spools, u64::MAX);
    write(&mut out)?;
    Ok(out.finish()?)
}

/// The worksheet: a header row, then a row for each of `samples`, the
/// description in column A (an inline string) and the value in column B.
fn sheet(out: &mut dyn Write, samples: &[Row], system: DateSystem) -> io::Result<()> {
    out.write_all(SHEET_START.as_bytes())?;
    let header = Row {
        label: "description".to_owned(),
        sample: Sample::Text("value".to_owned()),
    };
    for (number, row) in (1_u32..).zip(iter::once(&header).chain(samples)) {
        write!(out, "<row r=\"{number}\">")?;
        let cells = [
            (1, None, CellValue::String(&row.label)),
            (2, row.sample.style(), row.sample.value()),
        ];
        for (column, style, value) in cells {
            // The row number counts the rows of the list, far fewer than the
            // 1,048,576 rows a worksheet can hold, and the column is 1 or 2,
            // so the position exists; the error is there so that a position
            // that does not exist fails the command instead of leaving the
            // cell out.
            let outside = io::Error::from(io::ErrorKind::InvalidInput);
            let at = CellRef::new(column, number).ok_or(outside)?;
            cell(out, at, style, &value, system)?;
        }
        out.write_all(b"</row>")?;
    }
    out.write_all(SHEET_END.as_bytes())
}

/// Writes `value` as the cell `at`. A value `write_cell` refuses (it checks
/// the value before writing any byte to `out`) is written as the string
/// `refused: <why>` instead, in the same cell and with no cell format. The
/// cell is written to memory first, where writing cannot fail, so the one
/// error `write_cell` returns is that refusal.
fn cell(
    out: &mut dyn Write,
    at: CellRef,
    style: Option<u32>,
    value: &CellValue<'_>,
    system: DateSystem,
) -> io::Result<()> {
    let mut bytes = Vec::new();
    if let Err(error) = write_cell(&mut bytes, at, style, value, system) {
        let refusal = format!("refused: {error}");
        let _ = write_cell(&mut bytes, at, None, &CellValue::String(&refusal), system);
    }
    out.write_all(&bytes)
}

/// The shared strings part: [`SHARED`] in order.
fn shared_strings(out: &mut dyn Write) -> Result<(), CellError> {
    let mut strings = SharedStringsWriter::new(out)?;
    for text in SHARED {
        // The strings of `SHARED` are far shorter than the 32,767 UTF-16 code
        // units past which a string is cut.
        let (_, _written) = strings.push(text)?;
    }
    strings.finish()?;
    Ok(())
}
