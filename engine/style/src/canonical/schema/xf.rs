//! The table's cell format: `xf` with its `alignment` and `protection`
//! (ECMA-376-1:2016 §18.8.45).

use super::{Absent, Attribute, Element, Kind};
use crate::canonical::Value;
use crate::canonical::tokens::{HORIZONTAL_ALIGNMENTS, VERTICAL_ALIGNMENTS};

/// A cell format. `extLst` is kept as written.
pub(crate) static XF: Element = element!(
    "xf",
    &[
        attribute!("numFmtId", Kind::Uint),
        attribute!("fontId", Kind::Uint),
        attribute!("fillId", Kind::Uint),
        attribute!("borderId", Kind::Uint),
        attribute!("xfId", Kind::Uint),
        attribute!("quotePrefix", Kind::Bool, Value::Bool(false)),
        attribute!("pivotButton", Kind::Bool, Value::Bool(false)),
        attribute!("applyNumberFormat", Kind::Bool),
        attribute!("applyFont", Kind::Bool),
        attribute!("applyFill", Kind::Bool),
        attribute!("applyBorder", Kind::Bool),
        attribute!("applyAlignment", Kind::Bool),
        attribute!("applyProtection", Kind::Bool),
    ],
    &["alignment", "protection", "extLst"],
    Absent::Never
);
/// `alignment` (ECMA-376-1:2016 §18.8.1).
pub(crate) static ALIGNMENT: Element = element!(
    "alignment",
    &[
        attribute!("horizontal", Kind::Token(HORIZONTAL_ALIGNMENTS)),
        attribute!(
            "vertical",
            Kind::Token(VERTICAL_ALIGNMENTS),
            Value::Token("bottom")
        ),
        attribute!("textRotation", Kind::Uint),
        attribute!("wrapText", Kind::Bool),
        attribute!("indent", Kind::Uint),
        attribute!("relativeIndent", Kind::Int),
        attribute!("justifyLastLine", Kind::Bool),
        attribute!("shrinkToFit", Kind::Bool),
        attribute!("readingOrder", Kind::Uint),
    ],
    &[],
    Absent::Empty
);
/// `protection` (ECMA-376-1:2016 §18.8.33).
pub(crate) static PROTECTION: Element = element!(
    "protection",
    &[
        attribute!("locked", Kind::Bool),
        attribute!("hidden", Kind::Bool)
    ],
    &[],
    Absent::Empty
);

/// The elements of this module.
pub(super) static ELEMENTS: &[&Element] = &[&XF, &ALIGNMENT, &PROTECTION];
