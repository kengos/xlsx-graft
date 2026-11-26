//! [`StructureEdit`]: adds, renames, deletes and moves the sheets of a
//! workbook, hides and shows them, makes one the active sheet, and sets and
//! removes its defined names, rewriting the defined names' formulas and the
//! views' sheet positions to match.
//!
//! [`StructureEdit::open`] reads the workbook through
//! [`Workbook::read`](super::model::Workbook::read) and copies what an edit
//! needs from the package, so the edit holds no borrow of it. Each call then
//! checks its arguments against the state the earlier calls made, returns an
//! error and changes nothing when one is wrong, and otherwise changes the
//! state at once, formulas and view pointers included.
//!
//! - **`state.rs`**: the state: sheets, defined names, views and flags.
//! - **`snapshot.rs`**: what `open` copies from the package.
//! - **`apply.rs`**: `apply`, which writes the state to a `PackageEdit`,
//!   with **`cleanup.rs`** (the parts a delete leaves out),
//!   **`new_sheet.rs`** (an empty worksheet), **`app.rs`** (`docProps/app.xml`
//!   without its sheet lists), **`rewrite.rs`** (the workbook part) and
//!   **`ns.rs`** (the namespaces and types they use).
//! - **`sheets.rs`**: add, rename, delete and move a sheet.
//! - **`names.rs`**: set and remove a defined name.
//! - **`change.rs`**: a rename or a delete of a sheet, as it is applied to a
//!   formula.
//! - **`formula.rs`**: the sheets a defined name's formula names, rewritten
//!   when a sheet is renamed or deleted.
//! - **`parts.rs`**: the formulas of the other parts of the package,
//!   rewritten the same way, and what `apply` leaves as it was
//!   ([`StructureReport`](super::StructureReport)).
//! - **`views.rs`**: the views, pointed at another sheet when theirs is
//!   deleted or hidden.
//! - **`visibility.rs`**: set the visibility of a sheet and make a sheet the
//!   active one.
//! - **`tabs.rs`**: which worksheets' `sheetView`s get `tabSelected` set or
//!   removed.

mod app;
mod apply;
mod change;
mod cleanup;
mod formula;
mod names;
mod new_sheet;
mod ns;
mod parts;
mod rewrite;
mod sheets;
mod snapshot;
mod state;
mod tabs;
mod views;
mod visibility;

#[cfg(test)]
mod tests;

use super::StructureError;
use super::model::{SheetState, Workbook};
use crate::{PartWriter, WriteError, WriteLimits};
use snapshot::Snapshot;
use state::State;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::Package;

/// A structure edit of one workbook: the sheets added, renamed, deleted
/// and moved and the defined names set and removed, so far.
pub struct StructureEdit<'r> {
    snapshot: Snapshot,
    state: State<'r>,
}

impl<'r> StructureEdit<'r> {
    /// Opens the workbook of `package` for editing, under the XML limits of
    /// `limits`, pushing each problem found in `diagnostics`. `Ok(None)` when
    /// the workbook cannot be read whole, as
    /// [`Workbook::read`](super::model::Workbook::read) says.
    ///
    /// # Errors
    ///
    /// [`WriteError::Io`] when reading the package fails.
    pub fn open<R: Read + Seek>(
        package: &mut Package<R>,
        limits: &WriteLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, WriteError> {
        let Some(book) = Workbook::read(package, limits, diagnostics)? else {
            return Ok(None);
        };
        let state = State::new(&book);
        let snapshot = Snapshot::read(package, book, limits)?;
        Ok(Some(Self { snapshot, state }))
    }

    /// Adds a sheet named `name` at position `at` (after the last sheet when
    /// `None`), visible, a worksheet with `content` (an empty one when
    /// `None`).
    ///
    /// # Errors
    ///
    /// A name [`check_sheet_name`](super::check_sheet_name) refuses against the
    /// sheets' names, [`StructureError::PositionOutOfRange`] when `at` is past
    /// the number of sheets, [`StructureError::SheetIdExhausted`] when no
    /// `sheetId` is left.
    pub fn add_sheet(
        &mut self,
        name: &str,
        at: Option<usize>,
        content: Option<Box<dyn PartWriter + 'r>>,
    ) -> Result<(), StructureError> {
        self.state.add_sheet(name, at, content)
    }

    /// Renames the sheet `old` (letter case aside) to `new`, and writes `new`
    /// in the defined names' formulas that name it.
    ///
    /// # Errors
    ///
    /// [`StructureError::SheetMissing`] when no sheet is named `old`, or a
    /// name [`check_sheet_name`](super::check_sheet_name) refuses against the
    /// other sheets' names.
    pub fn rename_sheet(&mut self, old: &str, new: &str) -> Result<(), StructureError> {
        self.state.rename_sheet(old, new)
    }

    /// Deletes the sheet `name` (letter case aside): the defined names scoped
    /// to it are removed, a formula that names it gets `#REF!` or, for a 3-D
    /// reference, the next sheet inward, and a view that points at it points
    /// at the next visible sheet (else the last visible sheet before it).
    ///
    /// # Errors
    ///
    /// [`StructureError::SheetMissing`], [`StructureError::LastSheet`] for
    /// the only sheet and [`StructureError::LastVisibleSheet`] when no other
    /// sheet is visible.
    pub fn delete_sheet(&mut self, name: &str) -> Result<(), StructureError> {
        self.state.delete_sheet(name)
    }

    /// Moves the sheet `name` (letter case aside) to position `to`.
    ///
    /// # Errors
    ///
    /// [`StructureError::SheetMissing`], or
    /// [`StructureError::PositionOutOfRange`] when `to` is not a position of
    /// a sheet.
    pub fn move_sheet(&mut self, name: &str, to: usize) -> Result<(), StructureError> {
        self.state.move_sheet(name, to)
    }

    /// Sets the defined name `name`, scoped to the sheet `scope` (the
    /// workbook when `None`), to `formula`, a formula with no leading `=`. A
    /// built-in name is written as ECMA-376-1:2016 §18.2.5 spells it
    /// (`_xlnm.Print_Area`). A name of the same scope and spelling, letter
    /// case aside, is replaced where it is, its other attributes kept. A name
    /// is refused when it equals the name of a table of the opened workbook, a
    /// deleted sheet's included. A name whose `localSheetId` names no sheet of the
    /// opened workbook is not replaced by a name of the same spelling set for
    /// a sheet, even when the sheet is at that position now: the name is added
    /// beside it.
    ///
    /// # Errors
    ///
    /// A name [`check_defined_name`](super::check_defined_name) refuses,
    /// [`StructureError::SheetMissing`] for `scope`,
    /// [`StructureError::FormulaMalformed`],
    /// [`StructureError::FormulaSheetMissing`] and
    /// [`StructureError::DefinedNamesTooMany`].
    pub fn set_defined_name(
        &mut self,
        name: &str,
        scope: Option<&str>,
        formula: &str,
    ) -> Result<(), StructureError> {
        self.state.set_defined_name(name, scope, formula)
    }

    /// Removes the defined name `name` (letter case aside) scoped to the
    /// sheet `scope` (the workbook when `None`).
    ///
    /// # Errors
    ///
    /// [`StructureError::SheetMissing`] for `scope`, or
    /// [`StructureError::DefinedNameMissing`].
    pub fn remove_defined_name(
        &mut self,
        name: &str,
        scope: Option<&str>,
    ) -> Result<(), StructureError> {
        self.state.remove_defined_name(name, scope)
    }

    /// Sets the visibility (`state`) of the sheet `name` (letter case
    /// aside). A sheet already in `state` is left as it is. A view that
    /// points at a visible sheet that is hidden or very hidden points at the
    /// next visible sheet (else the last visible sheet before it), as it does
    /// after a delete; a sheet that is hidden and made very hidden, or shown,
    /// moves no view. A workbook with no `workbookView` gets one when its
    /// first sheet is hidden, since a missing `activeTab` names that sheet.
    ///
    /// # Errors
    ///
    /// [`StructureError::SheetMissing`], and
    /// [`StructureError::LastVisibleSheet`] when `state` hides the only
    /// visible sheet.
    pub fn set_sheet_state(&mut self, name: &str, state: SheetState) -> Result<(), StructureError> {
        self.state.set_sheet_state(name, state)
    }

    /// Makes the sheet `name` (letter case aside) the active sheet of every
    /// workbook view, and the first sheet shown in a view whose first sheet
    /// is after it. A workbook with no `workbookView` gets one, written when
    /// the sheet is not the first.
    ///
    /// # Errors
    ///
    /// [`StructureError::SheetMissing`], and [`StructureError::SheetHidden`]
    /// for a sheet that is hidden or very hidden.
    pub fn set_active_sheet(&mut self, name: &str) -> Result<(), StructureError> {
        self.state.set_active_sheet(name)
    }
}
