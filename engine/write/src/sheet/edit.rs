//! [`SheetEdit`]: changes to a worksheet that is already in a package: merge
//! and unmerge cells, set column widths and row heights, freeze panes and
//! protect the sheet.
//!
//! `open` reads the part once (`edit/open.rs`) and the tables of the sheet
//! (`edit/tables.rs`); each call checks its arguments against what was read
//! and the calls before, and changes nothing when it returns an error; `apply`
//! passes to `PackageEdit::replace` one rewriter (`edit/apply.rs`) that
//! streams the part and writes what the calls changed, each kind of element by its
//! concern (`merges`, `columns`, `rows`, `panes`, `protection`), and every
//! other byte as it was. An `mc:AlternateContent` is looked ahead to know
//! the kind it wraps (`edit/wrapped.rs`). Call `StructureEdit::apply` before
//! `SheetEdit::apply` ([`SheetEdit::apply`] says why).
//!
//! `set_column_width`, `set_row_height` and `freeze_panes` check their
//! arguments with `ColumnWidth::new`, `RowHeight::new` and `FrozenPanes::new`,
//! and `merge` checks its range as `MergedCells::new` does; each refuses with
//! a [`SheetError`] what Excel cannot hold. `protect` takes a
//! [`SheetProtection`], which was checked when it was made.

mod apply;
mod columns;
mod merges;
mod name;
mod open;
mod panes;
mod pass;
mod protection;
mod refuse;
mod rows;
mod tables;
mod tag;
mod wrapped;

use super::{CellRange, SheetError, SheetProtection};
use crate::error::WriteError;
use crate::limits::WriteLimits;
use columns::ColumnOps;
use merges::Merges;
use panes::PaneEdit;
use protection::ProtectionEdit;
use rows::RowOps;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Package, PartName};

/// Changes to a worksheet of a package: what [`open`](Self::open) read of it
/// and what the calls since changed, written by [`apply`](Self::apply).
///
/// A sheet is edited by its part, and a sheet added in the same edit has no
/// part in the package to open: write the cells, columns and the rest of a new
/// sheet with the `PartWriter` that writes the new sheet.
///
/// A part whose edited element (a kind a call changes) is out of the schema's
/// order, wrapped in `mc:AlternateContent`, or a second element of a kind that
/// does not repeat (`sheetViews`, `sheetData`, `mergeCells`, `sheetProtection`)
/// cannot be edited: `apply` passes `PackageEdit::replace` a rewriter that
/// refuses it, so `finish` reports an `xml_schema_invalid` at the part and
/// writes nothing ([`apply`](Self::apply) says more).
#[derive(Debug)]
pub struct SheetEdit {
    part: PartName,
    limits: WriteLimits,
    merges: Merges,
    columns: ColumnOps,
    rows: RowOps,
    panes: PaneEdit,
    protection: ProtectionEdit,
}

impl SheetEdit {
    /// Reads the worksheet `part` of `package` once, as a stream under
    /// `limits.xml`: the ranges it merges, whether it has frozen panes or a
    /// protection, and the `ref` of each table its Relationships part
    /// targets. Nothing is held of the cells, columns or rows.
    ///
    /// A problem in the part is a diagnostic and `Ok(None)`: the part cannot
    /// be read (damaged, or past the XML limits), its XML is not well formed,
    /// its root is not a `worksheet` of the main namespace (Transitional or
    /// Strict: `xml_schema_invalid`, `unexpected_element`), or a table part
    /// of the sheet cannot be read.
    ///
    /// # Errors
    ///
    /// [`WriteError::PartMissing`] when `package` has no part `part` (a sheet
    /// added in the same edit has none), [`WriteError::PartReserved`] for a
    /// Relationships part, [`WriteError::Io`] when reading the package fails.
    pub fn open<R: Read + Seek>(
        package: &mut Package<R>,
        part: &PartName,
        limits: &WriteLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, WriteError> {
        Self::read(package, part, limits, diagnostics)
    }

    /// Merges the cells of `range`.
    ///
    /// The check is O(n) in the ranges the sheet holds, so a caller that
    /// merges many ranges into a sheet that holds many pays O(k(n + k)); a
    /// `merge_all` that finds an overlap with `first_overlap` in
    /// O((n + k) log (n + k)) can serve it later.
    ///
    /// # Errors
    ///
    /// [`SheetError::MergeSingleCell`] for a range of one cell;
    /// [`SheetError::TooManyMerges`] when the sheet would hold more than
    /// 262,144 ranges (`count` is the number it would hold);
    /// [`SheetError::MergeOverlap`] when `range` shares a cell with a range
    /// the sheet holds now (`other`: the part's that were not unmerged, then
    /// the ranges that `merge` added to this `SheetEdit`; two ranges of the
    /// part that overlap each other are not looked at);
    /// [`SheetError::MergeTable`] when it shares a cell with the `ref` of a
    /// table of the sheet. Nothing changes.
    pub fn merge(&mut self, range: CellRange) -> Result<(), SheetError> {
        self.merges.merge(range)
    }

    /// Unmerges `range`: every range of the sheet with the same corners, the
    /// part's and the ranges that `merge` added to this `SheetEdit`, is
    /// removed. A `mergeCells` left with no range is not written.
    ///
    /// # Errors
    ///
    /// [`SheetError::MergeMissing`] when the sheet holds no such range.
    /// Nothing changes.
    pub fn unmerge(&mut self, range: CellRange) -> Result<(), SheetError> {
        self.merges.unmerge(range)
    }

    /// Sets the width of the columns `first` to `last` (1 is A), in
    /// characters, or removes the width (`None`: no `width` and no
    /// `customWidth` in them). A later call overrides the columns of an
    /// earlier one that it overlaps. A `col` of the part that the columns
    /// cover in part is cut, and each run keeps the other attributes of the
    /// `col`.
    ///
    /// # Errors
    ///
    /// [`SheetError::ColumnRange`] or [`SheetError::ColumnWidth`], as
    /// [`ColumnWidth::new`](super::ColumnWidth::new) says (a `None` width is
    /// checked as 0). Nothing changes.
    pub fn set_column_width(
        &mut self,
        first: u16,
        last: u16,
        width: Option<f64>,
    ) -> Result<(), SheetError> {
        self.columns.set(first, last, width)
    }

    /// Sets the height of `row` (1 is the first), in points, or removes it
    /// (`None`: no `ht` and no `customHeight`). A later call for the row
    /// overrides the earlier ones. A row the part lacks is written when it has a height.
    ///
    /// # Errors
    ///
    /// [`SheetError::RowRange`] or [`SheetError::RowHeight`], as
    /// [`RowHeight::new`](super::RowHeight::new) says (a `None` height is
    /// checked as 0). Nothing changes.
    pub fn set_row_height(&mut self, row: u32, height: Option<f64>) -> Result<(), SheetError> {
        self.rows.set(row, height)
    }

    /// Freezes the top `rows` rows and the left `columns` columns in the
    /// first `sheetView` of `workbookViewId` 0, writing its `pane` and
    /// `selection` and removing the ones it had; 0 and 0 unfreeze it. A
    /// `sheetView` that is not there is added.
    ///
    /// # Errors
    ///
    /// [`SheetError::FreezeRange`], as
    /// [`FrozenPanes::new`](super::FrozenPanes::new) says. Nothing changes.
    pub fn freeze_panes(&mut self, rows: u32, columns: u16) -> Result<(), SheetError> {
        self.panes.freeze(rows, columns)
    }

    /// Protects the sheet: its `sheetProtection` is written as
    /// `protection` says, in place of the one the part has. There is no error
    /// to return: the value was checked when it was made.
    pub fn protect(&mut self, protection: SheetProtection) {
        self.protection.protect(protection);
    }

    /// Removes the sheet's `sheetProtection`.
    pub fn unprotect(&mut self) {
        self.protection.unprotect();
    }
}

#[cfg(test)]
mod tests;
