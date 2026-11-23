//! [`Font`] and the enumerations it uses (ECMA-376-1:2016 §18.8.22).

use super::Color;

/// A font. Each field is `None` to keep the base cell format's value or
/// `Some` to override it. `Some(false)` of `bold`, `italic` or `strike`,
/// `Some(Underline::None)` and `Some(VerticalAlign::Baseline)` remove the
/// setting from the base.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Font {
    /// The font family name, such as `Arial`: `None` keeps the base's name,
    /// `Some` overrides it and also removes the base font's theme font
    /// scheme, which would otherwise override the name.
    pub name: Option<String>,
    /// The size in points (ECMA-376-1:2016 §18.4.11): `None` keeps the base's
    /// size, `Some` overrides it.
    pub size: Option<f64>,
    /// Bold (ECMA-376-1:2016 §18.8.2): `None` keeps the base's setting,
    /// `Some(true)` makes the font bold and `Some(false)` removes the base's
    /// bold.
    pub bold: Option<bool>,
    /// Italic (ECMA-376-1:2016 §18.8.26): `None` keeps the base's setting,
    /// `Some(true)` makes the font italic and `Some(false)` removes the
    /// base's italic.
    pub italic: Option<bool>,
    /// Strike-through (ECMA-376-1:2016 §18.4.10): `None` keeps the base's
    /// setting, `Some(true)` strikes the text through and `Some(false)`
    /// removes the base's strike-through.
    pub strike: Option<bool>,
    /// The kind of underline (ECMA-376-1:2016 §18.4.13): `None` keeps the
    /// base's underline, `Some` overrides it and `Some(Underline::None)`
    /// removes it.
    pub underline: Option<Underline>,
    /// The text color: `None` keeps the base's color, `Some` overrides it.
    pub color: Option<Color>,
    /// Superscript or subscript (ECMA-376-1:2016 §18.4.14): `None` keeps the
    /// base's setting, `Some` overrides it and `Some(VerticalAlign::Baseline)`
    /// removes it.
    pub vertical_align: Option<VerticalAlign>,
}

/// The kind of underline: ST_UnderlineValues (ECMA-376-1:2016 §18.18.85).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Underline {
    /// No underline (`none`); removes the base's underline.
    None,
    /// A single underline (`single`).
    Single,
    /// A double underline (`double`).
    Double,
    /// A single accounting underline, as wide as the cell (`singleAccounting`).
    SingleAccounting,
    /// A double accounting underline, as wide as the cell (`doubleAccounting`).
    DoubleAccounting,
}

/// Superscript or subscript (ECMA-376-1:2016 §18.4.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VerticalAlign {
    /// On the baseline (`baseline`); removes the base's superscript or
    /// subscript.
    Baseline,
    /// Above the baseline (`superscript`).
    Superscript,
    /// Below the baseline (`subscript`).
    Subscript,
}
