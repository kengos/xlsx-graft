//! [`RowHeight`]: the `ht` and `customHeight` of a `row`
//! (ECMA-376-1:2016 §18.3.1.73).

use super::SheetError;
use xlsx_graft_cell::number_text;

/// The height of a row, in points, from 0 to 409.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowHeight {
    row: u32,
    height: f64,
}

impl RowHeight {
    /// The last row.
    const MAX_ROW: u32 = 1_048_576;
    /// The tallest a row is, in points.
    const MAX_HEIGHT: f64 = 409.0;

    /// The height of `row` (1 is the first).
    ///
    /// # Errors
    ///
    /// [`SheetError::RowRange`] for a row outside 1 to 1,048,576, checked
    /// first; [`SheetError::RowHeight`] for a height that is not finite, is
    /// below 0 or is above 409.
    pub fn new(row: u32, height: f64) -> Result<Self, SheetError> {
        if !(1..=Self::MAX_ROW).contains(&row) {
            return Err(SheetError::RowRange { row });
        }
        if !(0.0..=Self::MAX_HEIGHT).contains(&height) {
            return Err(SheetError::RowHeight { height });
        }
        Ok(Self { row, height })
    }

    /// The row, from 1.
    #[must_use]
    pub fn row(&self) -> u32 {
        self.row
    }

    /// The height, in points.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.height
    }

    /// The attributes to add to the row's start tag: `ht`, and
    /// `customHeight="1"` to mark the height as set explicitly rather than the
    /// default.
    #[must_use]
    pub fn attributes(&self) -> [(&'static str, String); 2] {
        // `new` keeps the height finite, so `number_text` always returns `Ok`.
        let height = number_text(self.height).unwrap_or_default();
        [("ht", height), ("customHeight", "1".to_owned())]
    }
}
