//! [`DefaultFont`]: the font of the built-in workbook's cell format 0 (the
//! `Normal` style).

use thiserror::Error;
use xlsx_graft_xml::is_xml_char;

/// The font of the cell format 0 of the built-in workbook: a name and a size
/// in points. Excel computes the default height of a row from this font when
/// the worksheet has no `sheetFormatPr`.
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultFont {
    name: String,
    size: f64,
}

/// Why a [`DefaultFont`] was refused.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum DefaultFontError {
    /// The font name is empty or has a character XML 1.0 cannot hold.
    #[error("the font name {name:?} is empty or has a character XML does not allow")]
    Name {
        /// The name as the caller gave it.
        name: String,
    },
    /// The font size is not finite, or is not from 1 to 409 points.
    #[error("the font size {size} is not a finite number from 1 to 409")]
    Size {
        /// The size as the caller gave it.
        size: f64,
    },
}

impl DefaultFont {
    /// The font `name` at `size` points.
    ///
    /// # Errors
    ///
    /// [`DefaultFontError::Name`] for a name that is empty or has a character
    /// XML 1.0 cannot hold (checked first), and [`DefaultFontError::Size`]
    /// for a size that is not finite or is outside 1 to 409. The rules are
    /// those the style crate applies to the font of a cell format.
    pub fn new(name: impl Into<String>, size: f64) -> Result<Self, DefaultFontError> {
        let name = name.into();
        if name.is_empty() || !name.chars().all(is_xml_char) {
            return Err(DefaultFontError::Name { name });
        }
        if !(1.0..=409.0).contains(&size) {
            return Err(DefaultFontError::Size { size });
        }
        Ok(Self { name, size })
    }

    /// The font name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The font size, in points.
    #[must_use]
    pub fn size(&self) -> f64 {
        self.size
    }
}

impl Default for DefaultFont {
    /// Calibri 11, the font of a workbook Excel creates.
    fn default() -> Self {
        Self {
            name: "Calibri".to_owned(),
            size: 11.0,
        }
    }
}
