//! `SpooledRows`: the deflated `<row>` elements a `RowsWriter` collected,
//! with the range of their cells, written out by inflating them again.

use super::extent::Extent;
use crate::error::WriteError;
use crate::inflate::Inflater;
use std::io::{self, Write};
use xlsx_graft_cell::CellRef;
use xlsx_graft_spool::SpillBuffer;

/// The `<row>` elements of a worksheet's `<sheetData>`, written by a
/// [`RowsWriter`](crate::RowsWriter) and held deflated (in memory, or in a
/// spool once past the size the writer was given), with where their cells
/// are and how many strings were cut.
///
/// A `SpooledRows` holds no borrow, so the caller can keep it until the worksheet part is
/// written.
#[derive(Debug)]
pub struct SpooledRows {
    pub(super) buffer: SpillBuffer,
    pub(super) extent: Option<Extent>,
    pub(super) truncated: u64,
}

impl SpooledRows {
    /// Writes the `<row>` elements, inflated, to `out`: every row that had
    /// a cell, in ascending order, and nothing else. The `<sheetData>` tags
    /// are not written. Every call writes the same bytes.
    ///
    /// # Errors
    ///
    /// [`WriteError::Spool`] when the scratch storage cannot be read.
    /// [`WriteError::Io`] when writing to `out` fails.
    /// [`WriteError::Io`] of kind [`std::io::ErrorKind::InvalidData`] when the
    /// stored bytes are not a whole deflate stream (the stored bytes differ from the bytes deflated only when the scratch
    /// storage returns other bytes than it was given); an `InvalidData` error
    /// returned by `out` itself is returned as the same variant and kind, so the
    /// caller cannot tell the two apart.
    pub fn write_sheet_data(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
        // An error of the inflater carries its `WriteError`, which the
        // conversion returns as it was.
        io::copy(&mut Inflater::new(&mut self.buffer), out)?;
        Ok(())
    }

    /// The top-left and bottom-right cell of every cell written: the first
    /// column and row, and the last column and row, any cell is in. `None`
    /// when no cell was written (rows without cells are not counted).
    #[must_use]
    pub fn cells(&self) -> Option<(CellRef, CellRef)> {
        self.extent.and_then(Extent::corners)
    }

    /// How many cells had a string longer than 32,767 UTF-16 code units,
    /// which [`write_cell`](xlsx_graft_cell::write_cell) cut.
    #[must_use]
    pub fn truncated(&self) -> u64 {
        self.truncated
    }
}
