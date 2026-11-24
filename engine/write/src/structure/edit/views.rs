//! The sheets the workbook views point at (`activeTab`, `firstSheet`),
//! pointed at another sheet when the one they name is deleted.

use super::state::{Pointer, State};
use crate::structure::model::SheetState;

impl State<'_> {
    /// Points each view that names the sheet at `position` at the first
    /// visible sheet after it, or else the last visible one before it
    /// (the sheets as they are before the delete). A view stays as it is when
    /// no visible sheet is left, which a delete does not allow.
    pub(super) fn retarget_views(&mut self, position: usize) {
        let visible = |sheet: &&super::state::SheetEntry| sheet.state == SheetState::Visible;
        let (before, from) = self.sheets.split_at(position);
        let after = from.get(1..).unwrap_or_default();
        let target = (after.iter().find(visible))
            .or_else(|| before.iter().rev().find(visible))
            .map(|sheet| sheet.key);
        let key = from.first().map(|sheet| sheet.key);
        let moved = |pointer: Pointer| match (pointer, key, target) {
            (Pointer::Key(at), Some(key), Some(target)) if at == key => Pointer::Key(target),
            _ => pointer,
        };
        for view in &mut self.views {
            view.active = moved(view.active);
            view.first = moved(view.first);
        }
    }
}
