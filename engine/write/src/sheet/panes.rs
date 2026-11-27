//! [`FrozenPanes`]: the `pane` and `selection` of a `sheetView` that freeze
//! rows at the top and columns at the left (ECMA-376-1:2016 §18.3.1.66 and
//! §18.3.1.78).

use super::{SheetError, write_attributes};
use std::io::{self, Write};
use xlsx_graft_cell::CellRef;

/// How many rows at the top and columns at the left stay in view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrozenPanes {
    rows: u32,
    columns: u16,
}

type Attributes = Vec<(&'static str, String)>;

impl FrozenPanes {
    /// The most rows frozen: one less than the sheet has.
    const MAX_ROWS: u32 = CellRef::MAX_ROW - 1;
    /// The most columns frozen: one less than the sheet has.
    const MAX_COLUMNS: u16 = CellRef::MAX_COLUMN - 1;

    /// Freezes `rows` rows and `columns` columns; 0 rows and 0 columns
    /// freeze nothing and write no `pane`.
    ///
    /// # Errors
    ///
    /// [`SheetError::FreezeRange`] for more than 1,048,575 rows or 16,383
    /// columns.
    pub fn new(rows: u32, columns: u16) -> Result<Self, SheetError> {
        if rows > Self::MAX_ROWS || columns > Self::MAX_COLUMNS {
            return Err(SheetError::FreezeRange { rows, columns });
        }
        Ok(Self { rows, columns })
    }

    /// The rows frozen at the top.
    #[must_use]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// The columns frozen at the left.
    #[must_use]
    pub fn columns(&self) -> u16 {
        self.columns
    }

    /// The pane that scrolls and holds the selection, and the top-left cell
    /// of that pane; `None` when nothing is frozen.
    fn scrolling(&self) -> Option<(&'static str, CellRef)> {
        let pane = match (self.rows, self.columns) {
            (0, 0) => return None,
            (_, 0) => "bottomLeft",
            (0, _) => "topRight",
            _ => "bottomRight",
        };
        // `new` keeps rows <= 1,048,575 and columns <= 16,383, so
        // `CellRef::new(columns + 1, rows + 1)` always returns `Some`.
        let cell = CellRef::new(self.columns.checked_add(1)?, self.rows.checked_add(1)?)?;
        Some((pane, cell))
    }

    /// The attributes of the `pane`: `xSplit` and `ySplit` for the counts
    /// that are not 0, `topLeftCell`, `activePane` and `state="frozen"`;
    /// `None` when nothing is frozen.
    pub(crate) fn pane_attributes(&self) -> Option<Attributes> {
        let (pane, cell) = self.scrolling()?;
        let split = |name, count: u32| (count > 0).then(|| (name, count.to_string()));
        let splits = [
            split("xSplit", u32::from(self.columns)),
            split("ySplit", self.rows),
        ];
        let mut attributes: Attributes = splits.into_iter().flatten().collect();
        attributes.extend([
            ("topLeftCell", cell.to_string()),
            ("activePane", pane.to_owned()),
            ("state", "frozen".to_owned()),
        ]);
        Some(attributes)
    }

    /// The attributes of the `selection` in the pane that scrolls: its
    /// first cell is selected; `None` when nothing is frozen.
    pub(crate) fn selection_attributes(&self) -> Option<Attributes> {
        let (pane, cell) = self.scrolling()?;
        Some(vec![
            ("pane", pane.to_owned()),
            ("activeCell", cell.to_string()),
            ("sqref", cell.to_string()),
        ])
    }

    /// Writes `<sheetViews>` with one `<sheetView workbookViewId="0">`,
    /// holding the `pane` and the `selection` when something is frozen.
    /// `prefix` goes before the element names. No `tabSelected` is written.
    ///
    /// # Errors
    ///
    /// The error of `out`.
    pub fn write_sheet_views(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        write!(out, "<{prefix}sheetViews>")?;
        write!(out, r#"<{prefix}sheetView workbookViewId="0""#)?;
        match (self.pane_attributes(), self.selection_attributes()) {
            (Some(pane), Some(selection)) => {
                write!(out, "><{prefix}pane")?;
                write_attributes(out, &pane)?;
                write!(out, "/><{prefix}selection")?;
                write_attributes(out, &selection)?;
                write!(out, "/></{prefix}sheetView>")?;
            }
            _ => out.write_all(b"/>")?,
        }
        write!(out, "</{prefix}sheetViews>")
    }
}
