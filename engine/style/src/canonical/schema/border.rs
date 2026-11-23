//! The table's border: `border` and its sides (ECMA-376-1:2016 §18.8.4).

use super::{Absent, Attribute, Element, Kind};
use crate::canonical::Value;
use crate::canonical::tokens::BORDER_STYLES;

/// The attribute `style` of a side of a border or of the diagonal
/// (CT_BorderPr).
const SIDE_ATTRIBUTES: &[Attribute] = &[attribute!(
    "style",
    Kind::Token(BORDER_STYLES),
    Value::Token("none")
)];

/// A side of a border, or the diagonal (CT_BorderPr): a line style and a
/// color, which has no effect without either.
macro_rules! side {
    ($name:literal) => {
        element!($name, SIDE_ATTRIBUTES, &["color"], Absent::Empty)
    };
}

/// A border. Excel writes the four sides and the diagonal even when they are
/// empty, and the schema allows each to be left out.
pub(crate) static BORDER: Element = element!(
    "border",
    &[
        attribute!("diagonalUp", Kind::Bool),
        attribute!("diagonalDown", Kind::Bool),
        attribute!("outline", Kind::Bool, Value::Bool(true)),
    ],
    &[
        "start",
        "end",
        "left",
        "right",
        "top",
        "bottom",
        "diagonal",
        "vertical",
        "horizontal",
    ],
    Absent::Never,
    &["left", "right", "top", "bottom", "diagonal"]
);
/// `start`, a side of a border.
static START: Element = side!("start");
/// `end`, a side of a border.
static END: Element = side!("end");
/// `left`, a side of a border.
pub(crate) static LEFT: Element = side!("left");
/// `right`, a side of a border.
pub(crate) static RIGHT: Element = side!("right");
/// `top`, a side of a border.
pub(crate) static TOP: Element = side!("top");
/// `bottom`, a side of a border.
pub(crate) static BOTTOM: Element = side!("bottom");
/// `diagonal`, the diagonal line of a border.
pub(crate) static DIAGONAL: Element = side!("diagonal");
/// `vertical`, the line between the cells of a differential format.
static VERTICAL: Element = side!("vertical");
/// `horizontal`, the line between the cells of a differential format.
static HORIZONTAL: Element = side!("horizontal");

/// The elements of this module.
pub(super) static ELEMENTS: &[&Element] = &[
    &BORDER,
    &START,
    &END,
    &LEFT,
    &RIGHT,
    &TOP,
    &BOTTOM,
    &DIAGONAL,
    &VERTICAL,
    &HORIZONTAL,
];
