//! [`ColumnWidth`] and [`Columns`]: the widths of `cols`, written as
//! ECMA-376-1:2016 §18.3.1.17 and §18.3.1.13 have them.

use super::{SheetError, write_attributes};
use std::io::{self, Write};
use xlsx_graft_cell::number_text;

/// The width of columns `first` to `last` (1 is A), in characters of the
/// default font, from 0 to 255.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnWidth {
    first: u16,
    last: u16,
    width: f64,
}

impl ColumnWidth {
    /// The last column, XFD.
    const MAX_COLUMN: u16 = 16_384;
    /// The widest a column is, in characters.
    const MAX_WIDTH: f64 = 255.0;

    /// The width of the columns from `first` to `last`.
    ///
    /// # Errors
    ///
    /// [`SheetError::ColumnRange`] for a column outside 1 to 16,384 or a
    /// `first` after `last`, checked first; [`SheetError::ColumnWidth`] for a
    /// width that is not finite, is below 0 or is above 255.
    pub fn new(first: u16, last: u16, width: f64) -> Result<Self, SheetError> {
        let columns = 1..=Self::MAX_COLUMN;
        if !(columns.contains(&first) && columns.contains(&last) && first <= last) {
            return Err(SheetError::ColumnRange { first, last });
        }
        if !(0.0..=Self::MAX_WIDTH).contains(&width) {
            return Err(SheetError::ColumnWidth { width });
        }
        Ok(Self { first, last, width })
    }

    /// The first column, from 1.
    #[must_use]
    pub fn first(&self) -> u16 {
        self.first
    }

    /// The last column.
    #[must_use]
    pub fn last(&self) -> u16 {
        self.last
    }

    /// The width, in characters.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.width
    }
}

/// The column widths of a sheet, in ascending order of column without
/// overlap.
#[derive(Debug, Clone, PartialEq)]
pub struct Columns {
    widths: Vec<ColumnWidth>,
}

impl Columns {
    /// The widths of a sheet's columns, as given.
    ///
    /// # Errors
    ///
    /// [`SheetError::ColumnsOverlap`] for the first width whose first column
    /// is not past the last column of the one before it.
    pub fn new(widths: Vec<ColumnWidth>) -> Result<Self, SheetError> {
        let broken = widths
            .windows(2)
            .find(|pair| pair[1].first <= pair[0].last)
            .map(|pair| SheetError::ColumnsOverlap {
                first: pair[1].first,
                last: pair[1].last,
            });
        match broken {
            Some(error) => Err(error),
            None => Ok(Self { widths }),
        }
    }

    /// The widths, in the order given.
    #[must_use]
    pub fn widths(&self) -> &[ColumnWidth] {
        &self.widths
    }

    /// Writes `<cols>` with a `<col>` for each width, `customWidth="1"` to
    /// mark the width as set explicitly rather than the default; nothing for
    /// no widths. `prefix` goes before the element names.
    ///
    /// # Errors
    ///
    /// The error of `out`.
    pub fn write(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        if self.widths.is_empty() {
            return Ok(());
        }
        write!(out, "<{prefix}cols>")?;
        for column in &self.widths {
            write!(out, "<{prefix}col")?;
            // `new` keeps the width finite, so `number_text` always returns
            // `Ok`.
            let width = number_text(column.width).unwrap_or_default();
            let attributes = [
                ("min", column.first.to_string()),
                ("max", column.last.to_string()),
                ("width", width),
                ("customWidth", "1".to_owned()),
            ];
            write_attributes(out, &attributes)?;
            out.write_all(b"/>")?;
        }
        write!(out, "</{prefix}cols>")
    }
}
