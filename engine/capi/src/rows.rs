//! [`XlsxGraftRows`]: a sheet's rows read one at a time, behind the rows
//! handle; `abi/rows.rs` only marshals.
//!
//! It shares the book's package and diagnostics rather than borrowing the
//! book, so either handle may be closed first.

use crate::book::Reader;
use crate::message::failed;
use crate::value::XlsxGraftValue;
use std::cell::RefCell;
use std::rc::Rc;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_read::{Row, SheetRows};

/// The rows of one sheet, opened through the C ABI.
pub struct XlsxGraftRows {
    rows: SheetRows<Reader>,
    /// The row last read, which `values` lends text from.
    row: Row,
    values: Vec<XlsxGraftValue>,
    diagnostics: Rc<RefCell<Diagnostics>>,
}

impl XlsxGraftRows {
    pub(crate) fn new(rows: SheetRows<Reader>, diagnostics: Rc<RefCell<Diagnostics>>) -> Self {
        Self {
            rows,
            row: Row::new(),
            values: Vec::new(),
            diagnostics,
        }
    }

    /// The next row: its number and its values from column A to its last
    /// cell, alive until the next call; `None` when the sheet has no more,
    /// or a problem stopped it, which the book's diagnostics say.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) when the host fails,
    /// its message kept. No row comes after it.
    pub(crate) fn next(&mut self) -> Result<Option<(u32, &[XlsxGraftValue])>, i32> {
        // The values lend text from the row, which reading the next one
        // overwrites.
        self.values.clear();
        // Nothing else borrows the diagnostics while a call lasts: a handle
        // is used from one thread at a time, and no call re-enters another.
        let mut diagnostics = self.diagnostics.try_borrow_mut().map_err(failed)?;
        let more = self.rows.next_row(&mut self.row, &mut diagnostics);
        drop(diagnostics);
        if !more.map_err(failed)? {
            return Ok(None);
        }
        self.values
            .extend(self.row.values().map(XlsxGraftValue::of));
        Ok(Some((self.row.number(), &self.values)))
    }
}

#[cfg(test)]
mod tests;
