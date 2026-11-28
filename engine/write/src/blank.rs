//! The built-in blank workbook: a package of one empty worksheet, `Sheet1`,
//! and a styles part with the one cell format General in the font the caller
//! gives ([`DefaultFont`], Calibri 11 by default), for a workbook that is
//! created rather than read. The bytes are the same on every call for the
//! same [`Defaults`]: the engine reads no clock, no random source and no
//! environment.
//!
//! It holds, in this order, `[Content_Types].xml` (first, as Excel writes
//! it), `_rels/.rels`, `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`,
//! `xl/worksheets/sheet1.xml` and `xl/styles.xml`: no shared strings, no
//! theme and no `docProps`. The worksheet has an empty `sheetData` and a
//! `sheetFormatPr` from [`Defaults::sheet_format`]: 15 points for the default
//! Calibri 11, or the height the caller gave, or 15 points for a font given
//! alone (Calibri 11 included), the last two with `customHeight="1"` since
//! Excel otherwise computes the default row height from the Normal font
//! again. [`styles`] is the text of the styles part.
//!
//! # The create procedure
//!
//! A workbook is created by editing the built-in one: the caller's first
//! sheet is added, the built-in sheet is deleted, and the other sheets are
//! added. A [`StructureEdit`](crate::structure::StructureEdit) refuses a
//! sheet name that another sheet has, so the built-in sheet is renamed first
//! when the caller's first sheet would take its name:
//!
//! 1. Open a `StructureEdit` on [`package`] of the caller's [`Defaults`].
//! 2. When [`check_sheet_name`](crate::structure::check_sheet_name)`(first,
//!    [SHEET])` returns
//!    [`StructureError::SheetNameTaken`](crate::structure::StructureError::SheetNameTaken)
//!    (the comparison `StructureEdit` makes, which ignores letter case),
//!    `rename_sheet(SHEET, "Sheet2")`.
//! 3. `add_sheet(first, None, content)`.
//! 4. `delete_sheet` the built-in sheet, by its name now (`SHEET` or
//!    `Sheet2`).
//! 5. `add_sheet` each further sheet.
//! 6. `apply` the edit to a [`PackageEdit`](crate::PackageEdit) of the
//!    package, and `finish` it.
//!
//! ```
//! use xlsx_graft_diagnostics::Diagnostics;
//! use xlsx_graft_spool::MemorySpools;
//! use xlsx_graft_write::blank;
//! use xlsx_graft_write::structure::{StructureEdit, StructureError, check_sheet_name};
//! use xlsx_graft_write::{PackageEdit, WriteLimits};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let first = "sheet1";
//! let limits = WriteLimits::default();
//! let mut diagnostics = Diagnostics::new();
//! let mut package = blank::package(&blank::Defaults::default())?;
//! let mut structure = StructureEdit::open(&mut package, &limits, &mut diagnostics)?
//!     .ok_or("the built-in workbook is not read")?;
//! let mut built_in = blank::SHEET;
//! if let Err(StructureError::SheetNameTaken { .. }) = check_sheet_name(first, [blank::SHEET]) {
//!     structure.rename_sheet(blank::SHEET, "Sheet2")?;
//!     built_in = "Sheet2";
//! }
//! structure.add_sheet(first, None, None)?;
//! structure.delete_sheet(built_in)?;
//! structure.add_sheet("Data", None, None)?;
//! let mut edit = PackageEdit::new(package);
//! structure.apply(&mut edit)?;
//! let bytes = edit
//!     .finish(Vec::new(), &mut MemorySpools, &limits, &mut diagnostics)?
//!     .ok_or("the workbook is not written")?;
//! assert!(bytes.starts_with(b"PK\x03\x04"));
//! # Ok(())
//! # }
//! ```

mod defaults;
mod font;
mod parts;

use crate::{DeflateWriter, WriteError, ZipWriter};
use std::io::{Cursor, Write};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::MemorySpools;

pub use defaults::Defaults;
pub use font::{DefaultFont, DefaultFontError};

/// The name of the built-in workbook's one sheet.
pub const SHEET: &str = "Sheet1";

/// The text of the built-in workbook's styles part (`xl/styles.xml`) for
/// `font`: one font, the two fills every styles part has, one border with no
/// line, one cell style format, one cell format (General, 0) and the cell
/// style `Normal`. It is valid under ECMA-376-1:2016 §18.8.
#[must_use]
pub fn styles(font: &DefaultFont) -> String {
    parts::styles(font)
}

/// The built-in blank workbook with the font and the `sheetFormatPr` of
/// `defaults`, opened as a package.
///
/// # Errors
///
/// [`WriteError::Io`] or [`WriteError::Spool`] when deflating a part or
/// writing the zip fails, which holds only for a failure of the host, and
/// [`WriteError::BlankUnreadable`] when the zip written cannot be opened as
/// a package, a mistake of the engine.
pub fn package(defaults: &Defaults) -> Result<Package<Cursor<Vec<u8>>>, WriteError> {
    open_blank(bytes(defaults)?)
}

/// The zip of the built-in workbook for `defaults`, the same bytes on every
/// call for the same `defaults`.
fn bytes(defaults: &Defaults) -> Result<Vec<u8>, WriteError> {
    let mut zip = ZipWriter::new(Vec::new());
    for (name, content) in parts::entries(defaults)? {
        let mut spools = MemorySpools;
        let mut out = DeflateWriter::new(&mut spools, u64::MAX);
        out.write_all(&content)?;
        zip.write_deflated(name.as_bytes(), out.finish()?)?;
    }
    zip.finish()
}

/// Opens the zip `bytes` as the built-in workbook's package, under the
/// default limits.
fn open_blank(bytes: Vec<u8>) -> Result<Package<Cursor<Vec<u8>>>, WriteError> {
    let opened = Package::open(
        Cursor::new(bytes),
        &Limits::default(),
        &mut Diagnostics::new(),
    );
    opened?.ok_or(WriteError::BlankUnreadable)
}

#[cfg(test)]
mod defaults_tests;
#[cfg(test)]
mod tests;
