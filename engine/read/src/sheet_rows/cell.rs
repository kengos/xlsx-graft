//! [`Open`]: where the rows and cells read so far stand, checked as each
//! starts, and the value of the cell being read, made as it ends, into the
//! row being filled ([`Cells`]).

use super::Halt;
use super::shape::{Kind, Values, reference};
use crate::row::Slot;
use crate::schema::{boolean, is_space, refuse};
use crate::{CellRef, DateSystem, FormatKind, Lookup, Row, SharedStrings, Styles, Value};
use std::cell::RefCell;
use std::io;
use xlsx_graft_diagnostics::{Diagnostic, Echo, Limit, SchemaReason};

/// What a cell's value is made with: the row it goes in, and what the
/// workbook says of its strings, formats and dates.
pub(super) struct Cells<'a> {
    /// The row being filled.
    pub(super) row: &'a mut Row,
    /// The shared strings table.
    pub(super) strings: &'a RefCell<SharedStrings>,
    /// The styles.
    pub(super) styles: &'a Styles,
    /// How the workbook counts days.
    pub(super) date_system: DateSystem,
    /// A shared string looked up.
    pub(super) scratch: &'a mut String,
    /// The most bytes of text the row may hold.
    pub(super) max_row_bytes: Limit,
}

impl Cells<'_> {
    /// Adds `text` to the row's text, held to `maxRowBytes`.
    pub(super) fn push(&mut self, text: &str) -> Result<(), Halt> {
        let size = (self.row.text().len()).saturating_add(text.len()) as u64;
        if size > self.max_row_bytes.value {
            return Err(Halt::from(Diagnostic::RowTooLarge {
                row: self.row.number().into(),
                size,
                limit: self.max_row_bytes,
            }));
        }
        self.row.text().push_str(text);
        Ok(())
    }
}

/// Where the rows and cells read so far stand, and the cell being read.
#[derive(Debug, Default)]
pub(super) struct Open {
    /// The number of the row read last; 0 before the first.
    row: u32,
    /// The column of the cell read last in the row; 0 before its first.
    column: u16,
    /// The type of the cell being read.
    kind: Option<Kind>,
    /// Its number format.
    format: Option<FormatKind>,
    /// Whether its `v` or `is` was read.
    valued: bool,
    /// Where its text starts in the row's text.
    start: usize,
    /// Where it starts in the part.
    offset: u64,
}

impl Open {
    /// Starts the row whose `r` is `number`, or that comes after the one
    /// before when it has none, in `cells`.
    pub(super) fn row(&mut self, number: Option<u32>, cells: &mut Cells<'_>) -> Result<(), Halt> {
        let next = self.row.saturating_add(1);
        let number = match number {
            Some(number) if number <= self.row => {
                return Err(Halt::from(Diagnostic::CellOrderInvalid {
                    reference: Echo::new(&number.to_string()),
                    previous: Echo::new(&self.row.to_string()),
                }));
            }
            Some(number) => number,
            None if next > CellRef::MAX_ROW => return Err(reference(&next.to_string())),
            None => next,
        };
        self.row = number;
        self.column = 0;
        cells.row.begin(number);
        Ok(())
    }

    /// Starts the cell at `offset` whose attributes gave `values`, after
    /// the one before in its row when it has no `r`.
    pub(super) fn cell(
        &mut self,
        values: &Values,
        offset: u64,
        cells: &mut Cells<'_>,
    ) -> Result<(), Halt> {
        let column = match values.cell {
            Some(cell) if cell.row() != self.row => {
                return Err(reference(&name(cell.column(), cell.row())));
            }
            Some(cell) => cell.column(),
            None if self.column >= CellRef::MAX_COLUMN => {
                return Err(reference(&name(CellRef::MAX_COLUMN + 1, self.row)));
            }
            None => self.column + 1,
        };
        if column <= self.column {
            return Err(Halt::from(Diagnostic::CellOrderInvalid {
                reference: Echo::new(&name(column, self.row)),
                previous: Echo::new(&name(self.column, self.row)),
            }));
        }
        self.format = match (values.kind, values.style) {
            (Kind::Number, Some(None)) => {
                return Err(refuse(offset, SchemaReason::InvalidValue, "s").into());
            }
            (Kind::Number, style) => {
                Some(cells.styles.kind(u64::from(style.flatten().unwrap_or(0))))
            }
            _ => None,
        };
        self.column = column;
        self.kind = Some(values.kind);
        self.valued = false;
        self.start = cells.row.text().len();
        self.offset = offset;
        Ok(())
    }

    /// Notes the cell being read has its `v` or `is`.
    pub(super) fn valued(&mut self) {
        self.valued = true;
    }

    /// Ends the cell being read: sets its value in the row.
    pub(super) fn finish(&mut self, cells: &mut Cells<'_>) -> Result<(), Halt> {
        let start = self.start;
        let text = cells.row.text();
        let value = text.get(start..).unwrap_or_default();
        let invalid = || Halt::from(refuse(self.offset, SchemaReason::InvalidValue, "v"));
        let slot = match self.kind {
            _ if !self.valued => Slot::Value(Value::Empty),
            Some(Kind::Number) => {
                // Rust reads `inf` and `NaN` too, which no number written is.
                let number = value.trim_matches(is_space).parse::<f64>().ok();
                let number = number.filter(|number| number.is_finite());
                let kind = self.format.unwrap_or(FormatKind::General);
                let number = number.ok_or_else(invalid)?;
                Slot::Value(Value::from_serial(number, kind, cells.date_system))
            }
            Some(Kind::Bool) => Slot::Value(Value::Bool(boolean(value).ok_or_else(invalid)?)),
            Some(Kind::Date) => Slot::Value(Value::from_iso(value).map_err(|_| invalid())?),
            Some(Kind::Shared) => {
                let index = value.trim_matches(is_space).parse::<u64>();
                let index = index.map_err(|_| invalid())?;
                text.truncate(start);
                return self.shared(index, cells);
            }
            Some(Kind::Error) if value.is_empty() => return Err(invalid()),
            Some(Kind::Error) => Slot::Error(start, text.len()),
            Some(Kind::Formula | Kind::Inline) | None => Slot::String(start, text.len()),
        };
        if let Slot::Value(_) = slot {
            text.truncate(start);
        }
        cells.row.set(self.column, slot);
        Ok(())
    }

    /// Sets the shared string at `index` as the value of the cell being
    /// read.
    fn shared(&self, index: u64, cells: &mut Cells<'_>) -> Result<(), Halt> {
        // Borrowed by no one else: a book's readers take turns.
        let strings = cells.strings.try_borrow_mut().map_err(io::Error::other);
        let mut strings = strings.map_err(Halt::Io)?;
        match strings.get(index, cells.scratch).map_err(Halt::Failed)? {
            Lookup::Found => {}
            Lookup::OutOfRange => return Err(Diagnostic::SharedStringMissing { index }.into()),
            // The table's own diagnostic says why it is not known.
            Lookup::Unknown => return Err(Halt::Refused(None)),
        }
        let start = cells.row.text().len();
        let string = std::mem::take(cells.scratch);
        let pushed = cells.push(&string);
        *cells.scratch = string;
        pushed?;
        let end = cells.row.text().len();
        cells.row.set(self.column, Slot::String(start, end));
        Ok(())
    }
}

/// The A1 reference of the cell at `column` and `row`, `column` written in
/// letters as far past XFD as it goes.
fn name(column: u16, row: u32) -> String {
    let mut letters = Vec::new();
    let mut column = u32::from(column);
    while column > 0 {
        let digit = (column - 1) % 26;
        letters.push(char::from(b'A' + u8::try_from(digit).unwrap_or(0)));
        column = (column - 1) / 26;
    }
    letters.iter().rev().collect::<String>() + &row.to_string()
}
