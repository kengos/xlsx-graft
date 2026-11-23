//! The table's font: `font` and the elements in it other than `color`
//! (ECMA-376-1:2016 §18.8.22).

use super::{Absent, Attribute, Element, Kind};
use crate::canonical::Value;
use crate::canonical::tokens::{UNDERLINES, VERTICAL_ALIGNS};

/// The attribute `val` of a boolean property (CT_BooleanProperty): true when
/// missing.
const BOOL_VAL: &[Attribute] = &[attribute!("val", Kind::Bool, Value::Bool(true))];

/// A boolean property of a font: `<b val="0"/>` is the same as no `b`.
macro_rules! flag {
    ($name:literal) => {
        element!(
            $name,
            BOOL_VAL,
            &[],
            Absent::When("val", Value::Bool(false))
        )
    };
}

/// An element of a font whose only attribute is `val`, of the given kind
/// with no default.
macro_rules! valued {
    ($name:literal, $kind:expr, $absent:expr) => {
        element!($name, &[attribute!("val", $kind)], &[], $absent)
    };
}

/// A font. CT_Font is an `xsd:choice` that may repeat, so any order of its
/// children is valid; they are written in the order Excel writes them.
pub(crate) static FONT: Element = element!(
    "font",
    &[],
    &[
        "b",
        "i",
        "strike",
        "condense",
        "extend",
        "outline",
        "shadow",
        "u",
        "vertAlign",
        "sz",
        "color",
        "name",
        "family",
        "charset",
        "scheme",
    ],
    Absent::Never
);
/// `b` (ECMA-376-1:2016 §18.8.2).
pub(crate) static B: Element = flag!("b");
/// `i` (ECMA-376-1:2016 §18.8.26).
pub(crate) static I: Element = flag!("i");
/// `strike` (ECMA-376-1:2016 §18.4.10).
pub(crate) static STRIKE: Element = flag!("strike");
/// `condense`, a boolean property of a font.
static CONDENSE: Element = flag!("condense");
/// `extend`, a boolean property of a font.
static EXTEND: Element = flag!("extend");
/// `outline`, a boolean property of a font.
static OUTLINE: Element = flag!("outline");
/// `shadow`, a boolean property of a font.
static SHADOW: Element = flag!("shadow");
/// `u` (ECMA-376-1:2016 §18.4.13): single when `val` is missing.
pub(crate) static U: Element = element!(
    "u",
    &[attribute!(
        "val",
        Kind::Token(UNDERLINES),
        Value::Token("single")
    )],
    &[],
    Absent::When("val", Value::Token("none"))
);
/// `vertAlign` (ECMA-376-1:2016 §18.4.14).
pub(crate) static VERT_ALIGN: Element = valued!(
    "vertAlign",
    Kind::Token(VERTICAL_ALIGNS),
    Absent::When("val", Value::Token("baseline"))
);
/// `sz` (ECMA-376-1:2016 §18.4.11).
pub(crate) static SZ: Element = valued!("sz", Kind::Double, Absent::Never);
/// `name` (ECMA-376-1:2016 §18.8.29).
pub(crate) static NAME: Element = valued!("name", Kind::Text, Absent::Never);
/// `family`, the font family class.
static FAMILY: Element = valued!("family", Kind::Int, Absent::Never);
/// `charset`, the font's character set.
static CHARSET: Element = valued!("charset", Kind::Int, Absent::Never);
/// `scheme` (ECMA-376-1:2016 §18.8.35).
pub(crate) static SCHEME: Element = valued!(
    "scheme",
    Kind::Token(&["none", "major", "minor"]),
    Absent::When("val", Value::Token("none"))
);

/// The elements of this module.
pub(super) static ELEMENTS: &[&Element] = &[
    &FONT,
    &B,
    &I,
    &STRIKE,
    &CONDENSE,
    &EXTEND,
    &OUTLINE,
    &SHADOW,
    &U,
    &VERT_ALIGN,
    &SZ,
    &NAME,
    &FAMILY,
    &CHARSET,
    &SCHEME,
];
