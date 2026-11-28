//! [`SheetFormat`]: the `sheetFormatPr` of a worksheet, with the default
//! height of its rows (ECMA-376-1:2016 §18.3.1.81).

use super::SheetError;
use std::io::{self, Write};
use xlsx_graft_cell::number_text;

/// The default format of a worksheet's rows: their height in points, from 0
/// to 409, for a row that has no `ht` of its own, and whether the height is
/// marked as set explicitly (`customHeight`) rather than computed by Excel
/// from the Normal font.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetFormat {
    default_row_height: f64,
    custom_height: bool,
}

impl SheetFormat {
    /// The tallest a row is, in points.
    const MAX_HEIGHT: f64 = 409.0;

    /// A sheet format whose rows are `default_row_height` points high, with
    /// no `customHeight` written.
    ///
    /// # Errors
    ///
    /// [`SheetError::RowHeight`] for a height that is not finite, is below 0
    /// or is above 409.
    pub fn new(default_row_height: f64) -> Result<Self, SheetError> {
        Self::checked(default_row_height, false)
    }

    /// A sheet format whose rows are `default_row_height` points high, marked
    /// with `customHeight="1"` as a height set explicitly.
    ///
    /// # Errors
    ///
    /// [`SheetError::RowHeight`], as for [`new`](Self::new).
    pub fn custom(default_row_height: f64) -> Result<Self, SheetError> {
        Self::checked(default_row_height, true)
    }

    /// The format of `default_row_height` and `custom_height`, when the height
    /// is finite and from 0 to 409.
    fn checked(default_row_height: f64, custom_height: bool) -> Result<Self, SheetError> {
        if !(0.0..=Self::MAX_HEIGHT).contains(&default_row_height) {
            return Err(SheetError::RowHeight {
                height: default_row_height,
            });
        }
        Ok(Self {
            default_row_height,
            custom_height,
        })
    }

    /// The sheet format of the built-in blank workbook: 15 points, the
    /// height of a row of Calibri 11, with no `customHeight`.
    pub(crate) const fn blank() -> Self {
        Self {
            default_row_height: 15.0,
            custom_height: false,
        }
    }

    /// The default height of a row, in points.
    #[must_use]
    pub fn default_row_height(&self) -> f64 {
        self.default_row_height
    }

    /// Whether the height is marked as set explicitly (`customHeight="1"`).
    #[must_use]
    pub fn custom_height(&self) -> bool {
        self.custom_height
    }

    /// Writes the `sheetFormatPr` element, with `prefix` before its name.
    ///
    /// # Errors
    ///
    /// The error of `out`.
    pub fn write(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        // `new` keeps the height finite, so `number_text` always returns `Ok`.
        let height = number_text(self.default_row_height).unwrap_or_default();
        let custom = if self.custom_height {
            " customHeight=\"1\""
        } else {
            ""
        };
        write!(
            out,
            "<{prefix}sheetFormatPr defaultRowHeight=\"{height}\"{custom}/>"
        )
    }
}
