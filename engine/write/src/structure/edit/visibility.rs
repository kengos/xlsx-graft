//! The calls that set the visibility of a sheet and make a sheet the active
//! one.
//!
//! Each call checks everything first and changes the state only when every
//! check passed, so a call that returns an `Err` changes nothing.

use super::state::{Pointer, State, ViewEntry};
use crate::structure::StructureError;
use crate::structure::model::{Sheet, SheetState, Workbook};
use std::collections::BTreeMap;

impl State<'_> {
    /// Sets the visibility of the sheet `name` to `new`. The views that
    /// point at a sheet that stops being visible point at another one, as
    /// after a delete. A workbook with no view has the first sheet as its
    /// active sheet (a missing `activeTab` is 0), so hiding the first sheet
    /// of one adds the view [`State::set_active_sheet`] adds first, and that
    /// view is pointed at another sheet.
    pub(super) fn set_sheet_state(
        &mut self,
        name: &str,
        new: SheetState,
    ) -> Result<(), StructureError> {
        let (position, sheet) = self.find_sheet(name)?;
        let (key, was, name) = (sheet.key, sheet.state, sheet.name.clone());
        if was == new {
            return Ok(());
        }
        if was == SheetState::Visible {
            let other_visible = (self.sheets.iter())
                .any(|other| other.key != key && other.state == SheetState::Visible);
            if !other_visible {
                return Err(StructureError::LastVisibleSheet { name });
            }
            if position == 0 && self.views.is_empty() {
                self.add_view(key);
            }
            self.retarget_views(position);
        }
        (self.sheets.iter_mut())
            .filter(|sheet| sheet.key == key)
            .for_each(|sheet| sheet.state = new);
        Ok(())
    }

    /// Makes the sheet `name` the active sheet of every view, and the first
    /// sheet of each view whose first sheet is after it (or a position past
    /// the sheets). A workbook with no view gets one, which names the first
    /// visible sheet as its first sheet.
    pub(super) fn set_active_sheet(&mut self, name: &str) -> Result<(), StructureError> {
        let (position, sheet) = self.find_sheet(name)?;
        if sheet.state != SheetState::Visible {
            let name = sheet.name.clone();
            return Err(StructureError::SheetHidden { name });
        }
        let key = sheet.key;
        if self.views.is_empty() {
            self.add_view(key);
            return Ok(());
        }
        let at: BTreeMap<usize, usize> = (self.sheets.iter().enumerate())
            .map(|(at, sheet)| (sheet.key, at))
            .collect();
        let after = |pointer: Pointer| match pointer {
            Pointer::Raw => true,
            Pointer::Key(other) => at.get(&other).is_some_and(|&at| at > position),
        };
        for view in &mut self.views {
            view.active = Pointer::Key(key);
            view.first = if after(view.first) {
                Pointer::Key(key)
            } else {
                view.first
            };
        }
        Ok(())
    }

    /// Makes a workbook with no view open on a visible sheet. Such a workbook
    /// opens on the sheet at position 0 (a missing `activeTab` is 0), so
    /// when a delete or a move puts a sheet that is not visible first,
    /// `show_first_visible` adds the view [`State::set_active_sheet`] adds:
    /// its `activeTab` and `firstSheet` name the first visible sheet.
    ///
    /// Nothing is added in these cases:
    /// - The workbook has a view, which says what opens: `set_active_sheet`,
    ///   the deletes and the moves have already set its `activeTab` and
    ///   `firstSheet`.
    /// - The sheet at position 0 is visible.
    /// - The sheet at position 0 is the first sheet of the opened workbook
    ///   `book`, in the state `book` has for that sheet: the workbook was
    ///   opened that way, and nothing changed the state.
    ///
    /// [`apply`](super::StructureEdit::apply) calls it once, on the final
    /// order of the sheets.
    pub(super) fn show_first_visible(&mut self, book: &Workbook) {
        if !self.views.is_empty() {
            return;
        }
        let opens_as_before = (self.sheets.first()).is_none_or(|sheet| {
            let opened = book.sheets().first().map(Sheet::state);
            sheet.state == SheetState::Visible || (sheet.key == 0 && opened == Some(sheet.state))
        });
        if opens_as_before {
            return;
        }
        let visible = (self.sheets.iter())
            .find(|sheet| sheet.state == SheetState::Visible)
            .map(|sheet| sheet.key);
        // `for_each` over the `Option`, not an `if let`: no branch is left that
        // no test reaches. `visible` is `None` when the workbook was opened
        // with no visible sheet: no view is added then, as no sheet can open
        // visible.
        visible.into_iter().for_each(|key| self.add_view(key));
    }

    /// Adds the view of a workbook that has none, with the sheet `key` as its
    /// active sheet and the first visible sheet as its first sheet.
    fn add_view(&mut self, key: usize) {
        let first = (self.sheets.iter())
            .find(|sheet| sheet.state == SheetState::Visible)
            .map_or(key, |sheet| sheet.key);
        self.views.push(ViewEntry {
            active: Pointer::Key(key),
            first: Pointer::Key(first),
            // A view that does not write `activeTab` names the first sheet of
            // the workbook that was opened.
            opened_active: Pointer::Key(0),
        });
    }
}
