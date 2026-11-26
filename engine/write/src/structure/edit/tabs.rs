//! Which `sheetView`s of which worksheets get `tabSelected` set or removed
//! when `apply` writes the state: the tab of a workbook view's active sheet
//! is selected, and the tab of every other sheet of that view is not, so the
//! sheet a view opens on is the one its tab says (ECMA-376-1:2016 §18.3.1.87).
//!
//! A workbook view's active sheet changed when its `activeTab` points at
//! another sheet than the one it named when the workbook was opened
//! (`ViewEntry::opened_active`): after `set_active_sheet`, or after a hidden
//! or deleted sheet made `retarget_views` point it at another. A sheet that
//! was visible when the workbook was opened and is not now also loses
//! `tabSelected` in every `sheetView` whose `workbookViewId` can be read,
//! which is how a group of selected sheets is written, whatever the views'
//! active sheets are. A `sheetView`
//! that a sheet of the opened workbook does not have is not added, and a
//! sheet the edit added or deleted is not changed here.
//!
//! [`tab_changes`] gives each worksheet part its [`Tabs`]; `parts.rs` reads
//! the parts that have one and `parts/walk/retag.rs` writes the attribute.

use super::state::{Origin, Pointer, State};
use crate::structure::model::{SheetState, Workbook};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// What to do to the `tabSelected` of the `sheetView`s of one sheet part,
/// each `sheetView` named by its `workbookViewId`.
#[derive(Debug)]
pub(super) struct Tabs {
    /// The views whose active sheet changed, by the view's position among
    /// the workbook views, each with the key of its new active sheet: one map
    /// for all the sheets, so what is held grows with the views once.
    changed: Rc<BTreeMap<u32, usize>>,
    /// The key of the sheet this part is.
    key: usize,
    /// Whether the sheet is not visible now and was when opened: every
    /// `sheetView` loses `tabSelected`.
    clear_all: bool,
}

impl Tabs {
    /// What the `sheetView` of the workbook view `id` should have:
    /// `Some(true)` a selected tab, `Some(false)` an unselected one, `None`
    /// what it has.
    pub(super) fn wants(&self, id: u32) -> Option<bool> {
        if self.clear_all {
            return Some(false);
        }
        (self.changed.get(&id)).map(|&active| active == self.key)
    }
}

/// The [`Tabs`] of each sheet of the opened workbook `book` that has one
/// to apply, by the index of its part, for the views and sheets of `state`.
pub(super) fn tab_changes(state: &State<'_>, book: &Workbook) -> BTreeMap<usize, Tabs> {
    let changed: BTreeMap<u32, usize> = (state.views.iter().enumerate())
        .filter_map(|(id, view)| match view.active {
            Pointer::Key(key) if view.active != view.opened_active => {
                Some((u32::try_from(id).ok()?, key))
            }
            _ => None,
        })
        .collect();
    let changed = Rc::new(changed);
    // A sheet added has a key past the sheets of `book`.
    let hidden: BTreeSet<usize> = (state.sheets.iter())
        .filter(|sheet| sheet.state != SheetState::Visible)
        .filter(|sheet| {
            let opened = book.sheets().get(sheet.key);
            opened.is_some_and(|opened| opened.state() == SheetState::Visible)
        })
        .map(|sheet| sheet.key)
        .collect();
    let mut tabs = BTreeMap::new();
    for sheet in &state.sheets {
        let Origin::Original { part, .. } = sheet.origin else {
            continue;
        };
        let clear_all = hidden.contains(&sheet.key);
        if changed.is_empty() && !clear_all {
            continue;
        }
        let changed = Rc::clone(&changed);
        let key = sheet.key;
        tabs.insert(
            part,
            Tabs {
                changed,
                key,
                clear_all,
            },
        );
    }
    tabs
}
