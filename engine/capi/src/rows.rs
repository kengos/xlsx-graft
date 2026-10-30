//! [`XlsxGraftRows`]: a sheet's rows read one at a time, behind the rows
//! handle; `abi/rows.rs` only marshals.
//!
//! It shares the book's state ([`Shared`]) rather than borrowing the book.
//! The book's close takes its rows away, closing what they read; the
//! handle then reads no more, and only its own close is left to call.

use crate::message::failed;
use crate::shared::{Live, Shared};
use crate::status::XLSX_GRAFT_ERR_CLOSED;
use crate::value::XlsxGraftValue;
use std::cell::RefCell;
use std::rc::Rc;
use xlsx_graft_read::{Row, SheetRows};

/// The rows of one sheet, opened through the C ABI.
pub struct XlsxGraftRows {
    rows: Live,
    /// The row last read, which `values` lends text from.
    row: Row,
    values: Vec<XlsxGraftValue>,
    shared: Rc<Shared>,
}

impl XlsxGraftRows {
    /// # Errors
    ///
    /// As [`Shared::track`].
    pub(crate) fn new(
        rows: SheetRows<crate::book::Reader>,
        shared: Rc<Shared>,
    ) -> Result<Self, i32> {
        let rows = Rc::new(RefCell::new(Some(rows)));
        shared.track(&rows)?;
        Ok(Self {
            rows,
            row: Row::new(),
            values: Vec::new(),
            shared,
        })
    }

    /// What the rows share with their book.
    pub(crate) fn shared(&self) -> &Rc<Shared> {
        &self.shared
    }

    /// The next row: its number and its values from column A to its last
    /// cell, alive until the next call; `None` when the sheet has no more,
    /// or a problem stopped it, which the book's diagnostics say.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_CLOSED`] when the book was closed;
    /// [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) when the host fails,
    /// its message kept, and kept as the book's failure. No row comes after
    /// either.
    pub(crate) fn next(&mut self) -> Result<Option<(u32, &[XlsxGraftValue])>, i32> {
        // The values lend text from the row, which reading the next one
        // overwrites.
        self.values.clear();
        // Nothing else borrows these while a call lasts: a handle is used
        // from one thread at a time, and no call re-enters another.
        let mut rows = self.rows.try_borrow_mut().map_err(failed)?;
        let rows = rows.as_mut().ok_or(XLSX_GRAFT_ERR_CLOSED)?;
        let mut diagnostics = self.shared.diagnostics.try_borrow_mut().map_err(failed)?;
        let more = rows.next_row(&mut self.row, &mut diagnostics);
        drop(diagnostics);
        let more = more.map_err(|error| {
            self.shared.fail("io", error.to_string());
            failed(error)
        })?;
        if !more {
            return Ok(None);
        }
        self.values
            .extend(self.row.values().map(XlsxGraftValue::of));
        Ok(Some((self.row.number(), &self.values)))
    }
}

#[cfg(test)]
mod tests;
