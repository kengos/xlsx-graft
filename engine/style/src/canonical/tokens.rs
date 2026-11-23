//! The spelling of each enumeration's values in a styles part (the values of
//! the ST_* simple types the clause of each enumeration names). One list
//! per enumeration gives both `as_str` and the list of values the schema
//! table checks an attribute against, so the two cannot differ.

use crate::{
    BorderStyle, HorizontalAlignment, PatternType, Underline, VerticalAlign, VerticalAlignment,
};

/// For one enumeration, its `as_str` and the list of its values' spellings.
macro_rules! tokens {
    ($type:ident, $list:ident: $($variant:ident => $text:literal),+ $(,)?) => {
        impl $type {
            /// The value as the styles part spells it.
            pub(crate) const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        /// Every value of the enumeration as the styles part spells it.
        pub(super) const $list: &[&str] = &[$($text),+];
    };
}

tokens!(Underline, UNDERLINES:
    Single => "single",
    Double => "double",
    SingleAccounting => "singleAccounting",
    DoubleAccounting => "doubleAccounting",
    None => "none",
);
tokens!(VerticalAlign, VERTICAL_ALIGNS:
    Baseline => "baseline",
    Superscript => "superscript",
    Subscript => "subscript",
);
tokens!(PatternType, PATTERN_TYPES:
    None => "none",
    Solid => "solid",
    MediumGray => "mediumGray",
    DarkGray => "darkGray",
    LightGray => "lightGray",
    DarkHorizontal => "darkHorizontal",
    DarkVertical => "darkVertical",
    DarkDown => "darkDown",
    DarkUp => "darkUp",
    DarkGrid => "darkGrid",
    DarkTrellis => "darkTrellis",
    LightHorizontal => "lightHorizontal",
    LightVertical => "lightVertical",
    LightDown => "lightDown",
    LightUp => "lightUp",
    LightGrid => "lightGrid",
    LightTrellis => "lightTrellis",
    Gray125 => "gray125",
    Gray0625 => "gray0625",
);
tokens!(BorderStyle, BORDER_STYLES:
    None => "none",
    Thin => "thin",
    Medium => "medium",
    Dashed => "dashed",
    Dotted => "dotted",
    Thick => "thick",
    Double => "double",
    Hair => "hair",
    MediumDashed => "mediumDashed",
    DashDot => "dashDot",
    MediumDashDot => "mediumDashDot",
    DashDotDot => "dashDotDot",
    MediumDashDotDot => "mediumDashDotDot",
    SlantDashDot => "slantDashDot",
);
tokens!(HorizontalAlignment, HORIZONTAL_ALIGNMENTS:
    General => "general",
    Left => "left",
    Center => "center",
    Right => "right",
    Fill => "fill",
    Justify => "justify",
    CenterContinuous => "centerContinuous",
    Distributed => "distributed",
);
tokens!(VerticalAlignment, VERTICAL_ALIGNMENTS:
    Top => "top",
    Center => "center",
    Bottom => "bottom",
    Justify => "justify",
    Distributed => "distributed",
);
