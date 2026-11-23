//! [`Fill`] and its pattern types (ECMA-376-1:2016 §18.8.20).

use super::Color;

/// A cell's fill. A fill is not laid over the base cell format's fill: it
/// replaces it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Fill {
    /// A pattern fill (ECMA-376-1:2016 §18.8.32): a pattern drawn in the
    /// foreground color on the background color. A solid fill shows the
    /// foreground color alone.
    Pattern {
        /// The pattern.
        pattern: PatternType,
        /// The color of the pattern; `None` leaves it to the application.
        foreground: Option<Color>,
        /// The color behind the pattern; `None` leaves it to the application.
        background: Option<Color>,
    },
}

/// The pattern of a pattern fill: ST_PatternType
/// (ECMA-376-1:2016 §18.18.55).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PatternType {
    /// No fill (`none`).
    None,
    /// The foreground color alone (`solid`).
    Solid,
    /// 50% gray (`mediumGray`).
    MediumGray,
    /// 75% gray (`darkGray`).
    DarkGray,
    /// 25% gray (`lightGray`).
    LightGray,
    /// Dark horizontal lines (`darkHorizontal`).
    DarkHorizontal,
    /// Dark vertical lines (`darkVertical`).
    DarkVertical,
    /// Dark lines going down to the right (`darkDown`).
    DarkDown,
    /// Dark lines going up to the right (`darkUp`).
    DarkUp,
    /// A dark grid (`darkGrid`).
    DarkGrid,
    /// A dark trellis (`darkTrellis`).
    DarkTrellis,
    /// Light horizontal lines (`lightHorizontal`).
    LightHorizontal,
    /// Light vertical lines (`lightVertical`).
    LightVertical,
    /// Light lines going down to the right (`lightDown`).
    LightDown,
    /// Light lines going up to the right (`lightUp`).
    LightUp,
    /// A light grid (`lightGrid`).
    LightGrid,
    /// A light trellis (`lightTrellis`).
    LightTrellis,
    /// 12.5% gray (`gray125`).
    Gray125,
    /// 6.25% gray (`gray0625`).
    Gray0625,
}
