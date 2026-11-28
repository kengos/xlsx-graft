//! The frozen panes a `SheetEdit` sets, and whether the part has a `pane`
//! now. `panes/stream.rs` writes the `sheetViews`.

mod stream;

use crate::sheet::{FrozenPanes, SheetError};
use std::io::{self, Write};

/// The `freeze_panes` call and what the part had.
#[derive(Debug)]
pub(super) struct PaneEdit {
    /// Whether the first `sheetView` of `workbookViewId` 0 has a `pane`.
    has_pane: bool,
    /// The last `freeze_panes` call.
    request: Option<FrozenPanes>,
}

impl PaneEdit {
    pub(super) fn new(has_pane: bool) -> Self {
        Self {
            has_pane,
            request: None,
        }
    }

    /// Freezes `rows` and `columns`, as `SheetEdit::freeze_panes` says.
    pub(super) fn freeze(&mut self, rows: u32, columns: u16) -> Result<(), SheetError> {
        self.request = Some(FrozenPanes::new(rows, columns)?);
        Ok(())
    }

    /// Whether the call changes the part: it freezes something, or it
    /// freezes nothing in a part that has a `pane`.
    pub(super) fn changed(&self) -> bool {
        let freezes = |panes: &FrozenPanes| panes.pane_attributes().is_some();
        self.request
            .as_ref()
            .is_some_and(|panes| freezes(panes) || self.has_pane)
    }

    /// Writes a new `sheetViews` with the panes, for a part that has none;
    /// nothing when nothing is frozen.
    pub(super) fn write_new(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        let frozen = self
            .request
            .filter(|panes| panes.pane_attributes().is_some());
        (frozen.iter()).try_for_each(|panes| panes.write_sheet_views(out, prefix))
    }
}
