//! [`Style`], the values of a cell's style stated by a caller, with the
//! [`Color`] and [`Protection`] it holds. The font, the fill, the border and
//! the alignment are in the modules below.

mod alignment;
mod border;
mod fill;
mod font;

pub use alignment::{Alignment, HorizontalAlignment, VerticalAlignment};
pub use border::{Border, BorderSide, BorderStyle};
pub use fill::{Fill, PatternType};
pub use font::{Font, Underline, VerticalAlign};

/// What to change in a cell's format. Each field that is `Some` replaces
/// the base cell format's own; `None` keeps it. The crate root says what
/// `Some` can state to remove a setting, and that nothing is checked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    /// The font; `None` keeps the base cell format's font.
    pub font: Option<Font>,
    /// The fill; `None` keeps the base cell format's fill.
    pub fill: Option<Fill>,
    /// The border; `None` keeps the base cell format's border.
    pub border: Option<Border>,
    /// The number format code (such as [`DATE`](crate::DATE) or `0.00`);
    /// `None` keeps the base cell format's number format.
    pub number_format: Option<String>,
    /// The alignment; `None` keeps the base cell format's alignment.
    pub alignment: Option<Alignment>,
    /// The protection; `None` keeps the base cell format's protection.
    pub protection: Option<Protection>,
}

impl Style {
    /// A style that changes only the number format, to `code`.
    #[must_use]
    pub fn number_format(code: impl Into<String>) -> Self {
        Self {
            number_format: Some(code.into()),
            ..Self::default()
        }
    }
}

/// A color: an ARGB value or a theme color (ECMA-376-1:2016 §18.3.1.15).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Color {
    /// An ARGB color: alpha, red, green and blue, in that order. Excel
    /// ignores the alpha byte, and writes it as `FF`.
    Rgb([u8; 4]),
    /// A color of the workbook's theme.
    Theme {
        /// The position of the color in the theme: 0 to 11 for the twelve
        /// colors a theme holds.
        index: u32,
        /// How much lighter (positive) or darker (negative) the theme color
        /// is made, from -1.0 to 1.0; `None` is the theme color as it is.
        tint: Option<f64>,
    },
}

/// Cell protection (ECMA-376-1:2016 §18.8.33). It takes effect only on a
/// sheet that is protected.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Protection {
    /// Whether the cell is locked: `None` keeps the base cell format's
    /// value, `Some` overrides it; `Some(false)` writes `locked="0"`: the
    /// cell is not locked.
    pub locked: Option<bool>,
    /// Whether the cell's formula is hidden: `None` keeps the base cell
    /// format's value, `Some` overrides it; `Some(false)` writes
    /// `hidden="0"`: the formula is shown.
    pub hidden: Option<bool>,
}
