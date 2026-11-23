//! The table's colors and fill: `color`, `fgColor`, `bgColor` and `fill` with
//! its `patternFill` (ECMA-376-1:2016 §18.8.20).

use super::{Absent, Attribute, Element, Kind};
use crate::canonical::Value;
use crate::canonical::tokens::PATTERN_TYPES;

/// A color (CT_Color), the same in a font, a fill and a border.
const COLOR_ATTRIBUTES: &[Attribute] = &[
    attribute!("auto", Kind::Bool),
    attribute!("indexed", Kind::Uint),
    attribute!("rgb", Kind::Hex),
    attribute!("theme", Kind::Uint),
    attribute!("tint", Kind::Double, Value::Double(0)),
];

/// A color element (CT_Color).
macro_rules! color {
    ($name:literal) => {
        element!($name, COLOR_ATTRIBUTES, &[], Absent::Never)
    };
}

/// `color` (ECMA-376-1:2016 §18.3.1.15).
pub(crate) static COLOR: Element = color!("color");
/// `fgColor` (ECMA-376-1:2016 §18.8.19).
pub(crate) static FG_COLOR: Element = color!("fgColor");
/// `bgColor` (ECMA-376-1:2016 §18.8.3).
pub(crate) static BG_COLOR: Element = color!("bgColor");
/// A fill; a `gradientFill` is kept as written.
pub(crate) static FILL: Element =
    element!("fill", &[], &["patternFill", "gradientFill"], Absent::Never);
/// `patternFill` (ECMA-376-1:2016 §18.8.32).
pub(crate) static PATTERN_FILL: Element = element!(
    "patternFill",
    &[attribute!("patternType", Kind::Token(PATTERN_TYPES))],
    &["fgColor", "bgColor"],
    Absent::Never
);

/// The elements of this module.
pub(super) static ELEMENTS: &[&Element] = &[&COLOR, &FG_COLOR, &BG_COLOR, &FILL, &PATTERN_FILL];
