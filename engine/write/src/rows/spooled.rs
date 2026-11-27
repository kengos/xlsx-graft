//! `SpooledRows`: the deflated `<row>` elements a `RowsWriter` collected,
//! with the range of their cells, written out by inflating them again.

use super::extent::Extent;
use crate::error::WriteError;
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZFlush, MZStatus};
use std::io::{self, Write};
use xlsx_graft_cell::CellRef;
use xlsx_graft_spool::SpillBuffer;

/// How many deflated bytes are read from the buffer at once, and how many
/// inflated bytes one call of the decompressor can fill.
const CHUNK: usize = 16_384;

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
    /// [`WriteError::Io`] of kind [`io::ErrorKind::InvalidData`] when the
    /// stored bytes are not a whole deflate stream (the stored bytes differ from the bytes deflated only when the scratch
    /// storage returns other bytes than it was given); an `InvalidData` error
    /// returned by `out` itself is returned as the same variant and kind, so the
    /// caller cannot tell the two apart.
    pub fn write_sheet_data(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
        let mut state = InflateState::new_boxed(DataFormat::Raw);
        let mut input = vec![0; CHUNK];
        let mut output = vec![0; CHUNK];
        let mut offset = 0_u64;
        // `input[start..end]` is the part of the last read the inflater has not consumed.
        let (mut start, mut end) = (0, 0);
        // Each call of `inflate` is given the whole output chunk. When the stored bytes
        // end (a read returns 0), the loop keeps calling the inflater with
        // empty input: it writes any output still pending (`Ok`) and returns
        // `StreamEnd` at the end of a whole stream, or returns
        // `Err(MZError::Buf)` when no progress is possible (a stream cut
        // short), which becomes `InvalidData`.
        loop {
            if start == end {
                end = self.buffer.read_at(offset, &mut input)?;
                offset = offset.saturating_add(end as u64);
                start = 0;
            }
            let pending = input.get(start..end).unwrap_or_default();
            let result = inflate(&mut state, pending, &mut output, MZFlush::None);
            out.write_all(output.get(..result.bytes_written).unwrap_or_default())?;
            start = start.saturating_add(result.bytes_consumed);
            match result.status {
                Ok(MZStatus::StreamEnd) => return Ok(()),
                Ok(_) => {}
                // A stream that ends before its last block, or bytes that are not deflate.
                Err(_) => return Err(io::Error::from(io::ErrorKind::InvalidData).into()),
            }
        }
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
