//! [`CellRef`]: where a cell stands, read from its A1 reference (a cell's
//! `r`, ST_CellRef, ECMA-376-1:2016 §18.18.7, written as §18.17.2.3.1 has
//! it).

use crate::ValueError;

/// Where a cell stands: its column and row, both from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRef {
    column: u16,
    row: u32,
}

impl CellRef {
    /// The last column, XFD (§18.17.5.1).
    pub const MAX_COLUMN: u16 = 16_384;
    /// The last row (§18.17.5.1).
    pub const MAX_ROW: u32 = 1_048_576;

    /// Reads an A1 reference to one cell: column letters, in either case
    /// (§18.17.2.3.1), then the row's digits, leading zeros taken as the
    /// grammar's digit sequence has them (`A01` is `A1`).
    ///
    /// # Errors
    ///
    /// [`ValueError::Form`] for anything else, an absolute reference
    /// (`$A$1`) and a range (`A1:B2`) included: a cell's `r` names the
    /// cell, not a reference to it in a formula. [`ValueError::Range`] for
    /// a column past XFD, or a row of 0 or past 1,048,576 (§18.17.5.1).
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let bytes = text.as_bytes();
        let letters = bytes.iter().take_while(|b| b.is_ascii_alphabetic()).count();
        let (column, row) = bytes.split_at(letters);
        if column.is_empty() || row.is_empty() || !row.iter().all(u8::is_ascii_digit) {
            return Err(ValueError::Form);
        }
        // Saturating: a long run of letters or digits is past the last
        // column or row, not a wrapped number.
        let column = column.iter().fold(0_u32, |value, letter| {
            value
                .saturating_mul(26)
                .saturating_add(u32::from(letter.to_ascii_uppercase() - b'A' + 1))
        });
        let row = row.iter().fold(0_u32, |value, digit| {
            value
                .saturating_mul(10)
                .saturating_add(u32::from(digit - b'0'))
        });
        match u16::try_from(column) {
            Ok(column) if column <= Self::MAX_COLUMN && (1..=Self::MAX_ROW).contains(&row) => {
                Ok(Self { column, row })
            }
            _ => Err(ValueError::Range),
        }
    }

    /// The column, 1 (A) to 16,384 (XFD).
    #[must_use]
    pub fn column(self) -> u16 {
        self.column
    }

    /// The row, 1 to 1,048,576.
    #[must_use]
    pub fn row(self) -> u32 {
        self.row
    }
}
