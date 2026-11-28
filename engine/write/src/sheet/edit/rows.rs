//! The row heights a `SheetEdit` sets: a table by row, where a later call
//! for a row overrides the earlier ones. The `row`s of the part are not held; `rows/stream.rs`
//! applies the table to them as they are read.

mod stream;

use crate::sheet::{RowHeight, SheetError, write_attributes};
use std::collections::BTreeMap;
use std::io::{self, Write};

/// The heights to set by row, `None` for a row whose height is removed.
#[derive(Debug, Default)]
pub(super) struct RowOps(BTreeMap<u32, Option<RowHeight>>);

/// The attributes of a new `row` of `height`.
fn row_attributes(height: &RowHeight) -> Vec<(&'static str, String)> {
    let mut attributes = vec![("r", height.row().to_string())];
    attributes.extend(height.attributes());
    attributes
}

impl RowOps {
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Sets the height of `row`, as `SheetEdit::set_row_height` says.
    pub(super) fn set(&mut self, row: u32, height: Option<f64>) -> Result<(), SheetError> {
        let checked = RowHeight::new(row, height.unwrap_or(0.0))?;
        self.0.insert(row, height.map(|_| checked));
        Ok(())
    }

    /// Whether any row has a height to set.
    fn has_heights(&self) -> bool {
        self.0.values().any(Option::is_some)
    }

    /// Writes a new `sheetData` of the rows with a height to set, for a part
    /// that has none; nothing when no row has one.
    pub(super) fn write_new(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        if !self.has_heights() {
            return Ok(());
        }
        write!(out, "<{prefix}sheetData>")?;
        for height in self.0.values().flatten() {
            write!(out, "<{prefix}row")?;
            write_attributes(out, &row_attributes(height))?;
            out.write_all(b"/>")?;
        }
        write!(out, "</{prefix}sheetData>")
    }
}
