//! [`Defaults`]: what the caller gives the built-in workbook: the font of its
//! cell format 0 and the default row height of its worksheet.

use super::DefaultFont;
use crate::sheet::{SheetError, SheetFormat};

/// The font of the built-in workbook's cell format 0 and the
/// `sheetFormatPr` of its worksheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Defaults {
    font: DefaultFont,
    sheet_format: SheetFormat,
}

impl Defaults {
    /// The defaults for a `font` and a `row_height` the caller may give. The
    /// worksheet always has a `sheetFormatPr`:
    ///
    /// - Neither given: Calibri 11 and `defaultRowHeight="15"` with no
    ///   `customHeight`, the height of a row of Calibri 11. The result equals
    ///   [`Defaults::default`].
    /// - Only a `font` given: that font and `defaultRowHeight="15"` with
    ///   `customHeight="1"`. Without `customHeight` Excel computes the row
    ///   height from the font again, so the 15 points would not hold.
    /// - A `row_height` given, with or without a `font`: that font (Calibri 11
    ///   without one) and that height with `customHeight="1"`.
    ///
    /// # Errors
    ///
    /// [`SheetError::RowHeight`] for a `row_height` that is not finite, is
    /// below 0 or is above 409.
    pub fn new(font: Option<DefaultFont>, row_height: Option<f64>) -> Result<Self, SheetError> {
        let sheet_format = match (&font, row_height) {
            (_, Some(height)) => SheetFormat::custom(height)?,
            (None, None) => SheetFormat::blank(),
            (Some(_), None) => SheetFormat::custom(15.0)?,
        };
        Ok(Self {
            font: font.unwrap_or_default(),
            sheet_format,
        })
    }

    /// The font of the cell format 0.
    #[must_use]
    pub fn font(&self) -> &DefaultFont {
        &self.font
    }

    /// The `sheetFormatPr` of the worksheet.
    #[must_use]
    pub fn sheet_format(&self) -> SheetFormat {
        self.sheet_format
    }
}

impl Default for Defaults {
    /// Calibri 11 and a default row height of 15 points.
    fn default() -> Self {
        Self {
            font: DefaultFont::default(),
            sheet_format: SheetFormat::blank(),
        }
    }
}
