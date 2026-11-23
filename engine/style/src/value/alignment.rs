//! [`Alignment`] and its horizontal and vertical positions
//! (ECMA-376-1:2016 §18.8.1).

/// How a cell's content is placed. Each field is `None` to keep the base
/// cell format's value or `Some` to override it. `Some(false)` writes the
/// flag as `0` (`wrapText="0"`: the text does not wrap).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Alignment {
    /// The horizontal position: `None` keeps the base's position, `Some`
    /// overrides it; `Some(HorizontalAlignment::General)` is the position by
    /// the type of the content.
    pub horizontal: Option<HorizontalAlignment>,
    /// The vertical position: `None` keeps the base's position, `Some`
    /// overrides it; `Some(VerticalAlignment::Bottom)` is the position
    /// Excel uses when none is stated.
    pub vertical: Option<VerticalAlignment>,
    /// Whether text wraps within the cell: `None` keeps the base's value,
    /// `Some(true)` wraps and `Some(false)` writes `wrapText="0"`: the
    /// text does not wrap.
    pub wrap_text: Option<bool>,
    /// Whether text shrinks to fit the cell: `None` keeps the base's value,
    /// `Some(true)` shrinks and `Some(false)` writes
    /// `shrinkToFit="0"`: the text does not shrink.
    pub shrink_to_fit: Option<bool>,
    /// The indent, in steps of about the width of three spaces: `None` keeps
    /// the base's indent and `Some` overrides it: `Some(0)` writes
    /// `indent="0"`, no indent.
    pub indent: Option<u32>,
    /// The text rotation in degrees: 0 to 90 counterclockwise, 91 to 180
    /// clockwise (the angle below horizontal plus 90), or 255 for stacked
    /// text. `None` keeps the base's rotation and `Some` overrides it:
    /// `Some(0)` writes `textRotation="0"`, horizontal text.
    pub text_rotation: Option<u8>,
}

/// The horizontal position: ST_HorizontalAlignment
/// (ECMA-376-1:2016 §18.18.40).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HorizontalAlignment {
    /// By the type of the content: text left, numbers right (`general`).
    General,
    /// Left (`left`).
    Left,
    /// Centered (`center`).
    Center,
    /// Right (`right`).
    Right,
    /// The content repeated to fill the cell (`fill`).
    Fill,
    /// Justified (`justify`).
    Justify,
    /// Centered across the selected cells (`centerContinuous`).
    CenterContinuous,
    /// Spread evenly (`distributed`).
    Distributed,
}

/// The vertical position: ST_VerticalAlignment (ECMA-376-1:2016 §18.18.88).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VerticalAlignment {
    /// At the top (`top`).
    Top,
    /// In the middle (`center`).
    Center,
    /// At the bottom (`bottom`).
    Bottom,
    /// Justified (`justify`).
    Justify,
    /// Spread evenly (`distributed`).
    Distributed,
}
