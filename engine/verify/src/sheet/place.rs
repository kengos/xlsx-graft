//! [`Place`]: where the rows and cells of a worksheet stand, checked as
//! each starts, as [`SheetCheck`](super::SheetCheck) says.

use crate::count::Unsigned;
use xlsx_graft_diagnostics::{Diagnostic, Echo};
use xlsx_graft_read::CellRef;

/// A row or cell refused: why, and the place it names, as written or as
/// it would be without `r`.
#[derive(Debug)]
pub(super) struct Refused {
    pub(super) diagnostic: Diagnostic,
    pub(super) place: String,
}

/// Where the rows and cells read so far stand.
#[derive(Debug, Default)]
pub(super) struct Place {
    /// The number of the last row kept; 0 before the first.
    last_row: u32,
    /// The column of the last cell kept in its row; 0 before its first.
    last_column: u16,
    /// The number of the row being read, unless it is not kept.
    row: Option<u32>,
}

impl Place {
    /// Starts the row whose `r` is `written`, or that comes after the last
    /// one kept when it has none: its number. A row not in the sheet, or at
    /// or before the last one kept, is not kept, and is refused.
    pub(super) fn row(&mut self, written: Option<&str>) -> Result<u32, Refused> {
        self.row = None;
        let number = match written {
            Some(text) => Unsigned::parse(text)
                .and_then(|number| u32::try_from(number).ok())
                .filter(|number| (1..=CellRef::MAX_ROW).contains(number))
                .ok_or_else(|| invalid(text))?,
            None if self.last_row >= CellRef::MAX_ROW => {
                return Err(invalid(&self.last_row.saturating_add(1).to_string()));
            }
            None => self.last_row.saturating_add(1),
        };
        if number <= self.last_row {
            return Err(order(number.to_string(), &self.last_row.to_string()));
        }
        self.last_row = number;
        self.last_column = 0;
        self.row = Some(number);
        Ok(number)
    }

    /// Starts the cell whose `r` is `written`, or that comes after the last
    /// one kept in its row when it has none: its A1 reference, or `None`
    /// in a row not kept, whose cells are not placed. A cell not in the
    /// sheet, in another row, or at or before the last one kept, is not
    /// kept, and is refused.
    pub(super) fn cell(&mut self, written: Option<&str>) -> Result<Option<String>, Refused> {
        let Some(row) = self.row else {
            return Ok(None);
        };
        let column = match written {
            Some(text) => match CellRef::parse(text) {
                Ok(cell) if cell.row() == row => cell.column(),
                Ok(cell) => return Err(invalid(&name(cell.column(), cell.row()))),
                Err(_) => return Err(invalid(text)),
            },
            None if self.last_column >= CellRef::MAX_COLUMN => {
                return Err(invalid(&name(CellRef::MAX_COLUMN.saturating_add(1), row)));
            }
            None => self.last_column.saturating_add(1),
        };
        if column <= self.last_column {
            return Err(order(name(column, row), &name(self.last_column, row)));
        }
        self.last_column = column;
        Ok(Some(name(column, row)))
    }
}

/// `cell_reference_invalid` at `place`.
fn invalid(place: &str) -> Refused {
    Refused {
        diagnostic: Diagnostic::CellReferenceInvalid {
            reference: Echo::new(place),
        },
        place: place.to_owned(),
    }
}

/// `cell_order_invalid` at `place`, after `previous`.
fn order(place: String, previous: &str) -> Refused {
    Refused {
        diagnostic: Diagnostic::CellOrderInvalid {
            reference: Echo::new(&place),
            previous: Echo::new(previous),
        },
        place,
    }
}

/// The A1 reference of the cell at `column` and `row`.
pub(super) fn name(column: u16, row: u32) -> String {
    letters(column) + &row.to_string()
}

/// The letters of `column`, as far past XFD as it goes.
pub(super) fn letters(column: u16) -> String {
    let mut letters = Vec::new();
    let mut column = u32::from(column);
    while column > 0 {
        let digit = (column - 1) % 26;
        letters.push(char::from(b'A' + u8::try_from(digit).unwrap_or(0)));
        column = (column - 1) / 26;
    }
    letters.iter().rev().collect()
}
