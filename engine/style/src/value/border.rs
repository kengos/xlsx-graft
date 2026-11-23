//! [`Border`], its sides and their line styles (ECMA-376-1:2016 §18.8.4).

use super::Color;

/// A cell's border. Each side is `None` to keep the base cell format's side
/// or `Some` to replace it whole, color included.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Border {
    /// The left side: `None` keeps the base cell format's left side, `Some`
    /// replaces it; `Some(BorderSide { style: BorderStyle::None, .. })`
    /// removes the base's side, its color too.
    pub left: Option<BorderSide>,
    /// The right side: `None` keeps the base cell format's right side, `Some`
    /// replaces it; `Some(BorderSide { style: BorderStyle::None, .. })`
    /// removes the base's side, its color too.
    pub right: Option<BorderSide>,
    /// The top side: `None` keeps the base cell format's top side, `Some`
    /// replaces it; `Some(BorderSide { style: BorderStyle::None, .. })`
    /// removes the base's side, its color too.
    pub top: Option<BorderSide>,
    /// The bottom side: `None` keeps the base cell format's bottom side,
    /// `Some` replaces it; `Some(BorderSide { style: BorderStyle::None, .. })`
    /// removes the base's side, its color too.
    pub bottom: Option<BorderSide>,
    /// The diagonal line, drawn up, down or both as the next two fields say:
    /// `None` keeps the base cell format's diagonal, `Some` replaces it;
    /// `Some(BorderSide { style: BorderStyle::None, .. })` removes the base's
    /// diagonal, its color too.
    pub diagonal: Option<BorderSide>,
    /// Whether the diagonal goes up, from the bottom left corner: `None`
    /// keeps the base cell format's value, `Some` overrides it;
    /// `Some(false)` writes `diagonalUp="0"`: the diagonal does not go up.
    pub diagonal_up: Option<bool>,
    /// Whether the diagonal goes down, from the top left corner: `None`
    /// keeps the base cell format's value, `Some` overrides it;
    /// `Some(false)` writes `diagonalDown="0"`: the diagonal does not go down.
    pub diagonal_down: Option<bool>,
}

/// One side of a border: its line style and color. In a [`Border`],
/// `Some(BorderSide { style: BorderStyle::None, .. })` removes the base's
/// side, its color too, as `Some(Underline::None)` removes the base's
/// underline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderSide {
    /// The line style.
    pub style: BorderStyle,
    /// The line color; `None` leaves it to the application (automatic).
    pub color: Option<Color>,
}

/// The line style of a border side: ST_BorderStyle
/// (ECMA-376-1:2016 §18.18.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BorderStyle {
    /// No line (`none`). Given in a [`Border`], it removes the base's side,
    /// as `Underline::None` removes the base's underline.
    None,
    /// A thin line (`thin`).
    Thin,
    /// A medium line (`medium`).
    Medium,
    /// A dashed line (`dashed`).
    Dashed,
    /// A dotted line (`dotted`).
    Dotted,
    /// A thick line (`thick`).
    Thick,
    /// A double line (`double`).
    Double,
    /// A hairline (`hair`).
    Hair,
    /// A medium dashed line (`mediumDashed`).
    MediumDashed,
    /// Alternating dashes and dots (`dashDot`).
    DashDot,
    /// Medium alternating dashes and dots (`mediumDashDot`).
    MediumDashDot,
    /// A dash and two dots (`dashDotDot`).
    DashDotDot,
    /// A medium dash and two dots (`mediumDashDotDot`).
    MediumDashDotDot,
    /// A slanted dash and dot (`slantDashDot`).
    SlantDashDot,
}
