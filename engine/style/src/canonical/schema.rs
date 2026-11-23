//! The table of the styles part's items and the elements in them
//! (ECMA-376-1:2016 §18.8): for each element, its attributes with their
//! types and defaults, the children it admits in the order they are written,
//! and when it has no effect. The canonical form reads and compares by this
//! table; what it does not name is kept as it was written.

/// An attribute, with no default or with the given one.
macro_rules! attribute {
    ($name:literal, $kind:expr) => {
        Attribute {
            name: $name,
            kind: $kind,
            default: None,
        }
    };
    ($name:literal, $kind:expr, $default:expr) => {
        Attribute {
            name: $name,
            kind: $kind,
            default: Some($default),
        }
    };
}

/// An element: its name, attributes, children and when it has no effect, and
/// optionally the children it writes when they are missing. The table is
/// built of struct literals only, so that every reference in it is a
/// constant.
macro_rules! element {
    ($name:literal, $attributes:expr, $children:expr, $absent:expr) => {
        element!($name, $attributes, $children, $absent, &[])
    };
    ($name:literal, $attributes:expr, $children:expr, $absent:expr, $write_empty:expr) => {
        Element {
            name: $name,
            attributes: $attributes,
            children: $children,
            absent: $absent,
            write_empty: $write_empty,
        }
    };
}

mod border;
mod fill;
mod font;
mod xf;

use super::Value;

pub(super) use border::{BORDER, BOTTOM, DIAGONAL, LEFT, RIGHT, TOP};
pub(super) use fill::{BG_COLOR, COLOR, FG_COLOR, FILL, PATTERN_FILL};
pub(super) use font::{B, FONT, I, NAME, SCHEME, STRIKE, SZ, U, VERT_ALIGN};
pub(super) use xf::{ALIGNMENT, PROTECTION, XF};

/// An element the table names. Two are equal when their names are, since the
/// table names each element once.
#[derive(Debug)]
pub(crate) struct Element {
    /// The element's local name.
    pub(super) name: &'static str,
    /// The attributes the table names, in the order they are written.
    pub(super) attributes: &'static [Attribute],
    /// The names of the children the table names, in the order they are
    /// written. A child's rank is its position here.
    pub(super) children: &'static [&'static str],
    /// When the element has no effect, so that it is the same as no element.
    pub(super) absent: Absent,
    /// The children written as an empty element when there is none.
    pub(super) write_empty: &'static [&'static str],
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Element {}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(other.name)
    }
}

impl std::hash::Hash for Element {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

/// An attribute the table names.
#[derive(Debug)]
pub(super) struct Attribute {
    /// The attribute's name; it has no prefix.
    pub(super) name: &'static str,
    /// How the attribute's text is read.
    pub(super) kind: Kind,
    /// The value the schema gives when the attribute is missing; the
    /// canonical form does not hold an attribute that has it.
    pub(super) default: Option<Value>,
}

/// The type of an attribute's value.
#[derive(Debug)]
pub(super) enum Kind {
    /// xsd:boolean: `1`, `true`, `0` or `false`.
    Bool,
    /// xsd:unsignedInt.
    Uint,
    /// xsd:int.
    Int,
    /// xsd:double, finite.
    Double,
    /// ST_UnsignedIntHex (ECMA-376-1:2016 §18.18.86): 8 hexadecimal digits.
    Hex,
    /// A value of an enumeration, from the list.
    Token(&'static [&'static str]),
    /// xsd:string, kept as written, white space included.
    Text,
}

impl Kind {
    /// The value `text` states. Text that is not of the type is
    /// [`Value::Raw`], which is equal only to the same text. Surrounding
    /// white space is not part of a value, except for [`Kind::Text`].
    pub(super) fn read(&self, text: &str) -> Value {
        let trimmed = text.trim_matches([' ', '\t', '\n', '\r']);
        let value = match self {
            Self::Bool => match trimmed {
                "1" | "true" => Some(Value::Bool(true)),
                "0" | "false" => Some(Value::Bool(false)),
                _ => None,
            },
            Self::Uint => trimmed.parse().ok().map(Value::Uint),
            Self::Int => trimmed.parse().ok().map(Value::Int),
            Self::Double => (trimmed.parse::<f64>().ok())
                .filter(|number| number.is_finite())
                .map(Value::double),
            Self::Hex => (trimmed.len() == 8 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| u32::from_str_radix(trimmed, 16).ok().map(Value::Hex))
                .flatten(),
            Self::Token(list) => {
                (list.iter().find(|token| **token == trimmed)).map(|t| Value::Token(t))
            }
            Self::Text => Some(Value::Text(text.to_owned())),
        };
        value.unwrap_or_else(|| Value::Raw(text.to_owned()))
    }
}

/// When an element has no effect.
#[derive(Debug)]
pub(super) enum Absent {
    /// Never: the element is a statement by being there.
    Never,
    /// When it has no attribute and no child.
    Empty,
    /// When its only attribute is the named one, with the given value, and
    /// it has no child: `<b val="0"/>` is the same as no `b`.
    When(&'static str, Value),
}

/// The element of the table that an item of the styles part can be: `font`,
/// `fill`, `border` or `xf`, named `local`.
pub(super) fn root(local: &[u8]) -> Option<&'static Element> {
    const ROOTS: [&str; 4] = ["font", "fill", "border", "xf"];
    lookup(local).filter(|element| ROOTS.contains(&element.name))
}

/// The element of the table whose local name is `local`.
pub(super) fn lookup(local: &[u8]) -> Option<&'static Element> {
    [
        font::ELEMENTS,
        fill::ELEMENTS,
        border::ELEMENTS,
        xf::ELEMENTS,
    ]
    .into_iter()
    .flatten()
    .copied()
    .find(|element| element.name.as_bytes() == local)
}
