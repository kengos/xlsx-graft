//! [`RowsWriter`]: the rows of a worksheet's `<sheetData>`, written one at a
//! time and held deflated until the worksheet part is written.

mod extent;
mod spooled;
#[cfg(test)]
mod tests;

use crate::deflate::DeflateWriter;
use crate::error::WriteError;
use extent::Extent;
use std::io::Write;
use xlsx_graft_cell::{CellRef, CellValue, DateSystem, Written, write_cell};
use xlsx_graft_spool::SpoolFactory;

pub use spooled::SpooledRows;

/// A row to write: the row number and its cells, in ascending column order.
/// Made with [`Row::new`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Row<'a> {
    /// The row number, from 1 to 1,048,576.
    pub number: u32,
    /// The cells of the row, each in a column after the column of the cell before it.
    pub cells: &'a [RowCell<'a>],
}

impl<'a> Row<'a> {
    /// The row `number` holding `cells`.
    #[must_use]
    pub fn new(number: u32, cells: &'a [RowCell<'a>]) -> Self {
        Self { number, cells }
    }
}

/// A cell of a [`Row`]: what [`write_cell`] takes besides the row. Made with
/// [`RowCell::new`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RowCell<'a> {
    /// The column, from 1 (A) to 16,384 (XFD).
    pub column: u16,
    /// The index into `cellXfs` of the cell's format; `None` and `Some(0)` write no
    /// `s` attribute.
    pub style: Option<u32>,
    /// What the cell holds.
    pub value: CellValue<'a>,
}

impl<'a> RowCell<'a> {
    /// The cell in `column` with the cell format `style` and the value
    /// `value`.
    #[must_use]
    pub fn new(column: u16, style: Option<u32>, value: CellValue<'a>) -> Self {
        Self {
            column,
            style,
            value,
        }
    }
}

/// The first and last column and the count of cut strings of the row `build`
/// wrote into `buffer`.
struct Built {
    first_column: u16,
    last_column: u16,
    truncated: u64,
}

/// Writes the rows of a worksheet's `<sheetData>`: each [`push`](Self::push)
/// is a `<row>` element with its cells, deflated when it is pushed, so a worksheet
/// does not have to be held whole in memory. [`finish`](Self::finish) returns
/// the rows as [`SpooledRows`].
///
/// Rows come in ascending row order, and the cells of a row in ascending
/// column order. Only the `<row>` elements are written: the `<sheetData>`
/// tags, the `<dimension>` and the rest of the worksheet are written by the caller.
///
/// ```
/// use xlsx_graft_cell::{CellValue, DateSystem};
/// use xlsx_graft_spool::MemorySpools;
/// use xlsx_graft_write::{Row, RowCell, RowsWriter};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut spools = MemorySpools;
/// let mut writer = RowsWriter::new(&mut spools, 1 << 20, DateSystem::Base1900);
/// let cells = [RowCell::new(2, None, CellValue::Integer(7))];
/// writer.push(&Row::new(3, &cells))?;
/// let mut rows = writer.finish()?;
/// let mut xml = Vec::new();
/// rows.write_sheet_data(&mut xml)?;
/// assert_eq!(xml, br#"<row r="3"><c r="B3"><v>7</v></c></row>"#);
/// # Ok(())
/// # }
/// ```
pub struct RowsWriter<'a> {
    deflate: DeflateWriter<'a>,
    system: DateSystem,
    /// The bytes of the row being built, cleared and reused for the next row pushed.
    buffer: Vec<u8>,
    /// The row number of the last row pushed, a row without cells included;
    /// `None` before the first push.
    last_row: Option<u32>,
    extent: Option<Extent>,
    truncated: u64,
    /// Whether writing a row to the deflate stream failed: the rows stored
    /// are then incomplete.
    failed: bool,
}

impl<'a> RowsWriter<'a> {
    /// A writer with no row, which deflates the rows into memory until the
    /// deflated bytes would pass `spill_at` and into a spool of `spools`
    /// after that. `system` is the date system the workbook counts dates in.
    pub fn new(spools: &'a mut dyn SpoolFactory, spill_at: u64, system: DateSystem) -> Self {
        Self {
            deflate: DeflateWriter::new(spools, spill_at),
            system,
            buffer: Vec::new(),
            last_row: None,
            extent: None,
            truncated: 0,
            failed: false,
        }
    }

    /// Writes `row`. A row without cells writes nothing, and the next push
    /// must give a row number after the row number of the row without cells.
    ///
    /// The row is checked and built whole before anything is written, in
    /// this order: the row number is in 1 to 1,048,576, then after the last
    /// row pushed; then for each cell in order, its column is in 1 to 16,384,
    /// then after the column of the cell before it, then
    /// [`write_cell`] accepts its value. The error returned is the one of the first check that fails.
    ///
    /// # Errors
    ///
    /// [`WriteError::RowOutOfRange`], [`WriteError::RowOutOfOrder`],
    /// [`WriteError::ColumnOutOfRange`], [`WriteError::CellOutOfOrder`] and
    /// [`WriteError::CellRefused`] are the caller's mistakes: nothing was
    /// written and the writer is as it was before the call.
    /// [`WriteError::Spool`] when the scratch storage fails. After a `Spool` error,
    /// every later `push` and [`finish`](Self::finish) return
    /// [`WriteError::Unusable`].
    pub fn push(&mut self, row: &Row<'_>) -> Result<(), WriteError> {
        if self.failed {
            return Err(WriteError::Unusable);
        }
        if !(1..=CellRef::MAX_ROW).contains(&row.number) {
            return Err(WriteError::RowOutOfRange { row: row.number });
        }
        let last = self.last_row.unwrap_or(0);
        if row.number <= last {
            return Err(WriteError::RowOutOfOrder {
                row: row.number,
                last,
            });
        }
        let Some(built) = self.build(row)? else {
            self.last_row = Some(row.number);
            return Ok(());
        };
        if let Err(error) = self.deflate.write_all(&self.buffer) {
            self.failed = true;
            return Err(error.into());
        }
        self.last_row = Some(row.number);
        self.extent = Some(Extent::with_row(
            self.extent,
            row.number,
            built.first_column,
            built.last_column,
        ));
        self.truncated = self.truncated.saturating_add(built.truncated);
        Ok(())
    }

    /// Ends the deflate stream and returns the rows written.
    ///
    /// # Errors
    ///
    /// [`WriteError::Spool`] when the scratch storage cannot be made or
    /// written; [`WriteError::Unusable`] when an earlier push failed on the
    /// scratch storage.
    pub fn finish(self) -> Result<SpooledRows, WriteError> {
        let deflated = self.deflate.finish()?;
        Ok(SpooledRows {
            buffer: deflated.buffer,
            extent: self.extent,
            truncated: self.truncated,
        })
    }

    /// Builds `row` as `<row r="N">…</row>` in `buffer`; `None`, with nothing written to
    /// `buffer`, when the row has no cell.
    fn build(&mut self, row: &Row<'_>) -> Result<Option<Built>, WriteError> {
        let (Some(first), Some(last)) = (row.cells.first(), row.cells.last()) else {
            return Ok(None);
        };
        self.buffer.clear();
        write!(self.buffer, r#"<row r="{}">"#, row.number)?;
        let mut previous = 0_u16;
        let mut truncated = 0_u64;
        for cell in row.cells {
            let at = CellRef::new(cell.column, row.number).ok_or(WriteError::ColumnOutOfRange {
                row: row.number,
                column: cell.column,
            })?;
            if cell.column <= previous {
                return Err(WriteError::CellOutOfOrder {
                    row: row.number,
                    column: cell.column,
                    last: previous,
                });
            }
            previous = cell.column;
            let written = write_cell(&mut self.buffer, at, cell.style, &cell.value, self.system)
                .map_err(|source| WriteError::CellRefused { at, source })?;
            truncated += u64::from(written == Written::Truncated);
        }
        self.buffer.extend_from_slice(b"</row>");
        Ok(Some(Built {
            first_column: first.column,
            last_column: last.column,
            truncated,
        }))
    }
}
