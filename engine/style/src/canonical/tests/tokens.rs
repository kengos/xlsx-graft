//! The spelling and the number of each enumeration's values, against the
//! lists of ECMA-376-1:2016: ST_UnderlineValues (ECMA-376-1:2016 §18.18.85),
//! ST_PatternType (ECMA-376-1:2016 §18.18.55), ST_BorderStyle
//! (ECMA-376-1:2016 §18.18.3), ST_HorizontalAlignment (ECMA-376-1:2016
//! §18.18.40) and ST_VerticalAlignment (ECMA-376-1:2016 §18.18.88). The
//! list each enumeration gives the schema table is the one `as_str` is
//! written from.

use crate::canonical::tokens::{
    BORDER_STYLES, HORIZONTAL_ALIGNMENTS, PATTERN_TYPES, UNDERLINES, VERTICAL_ALIGNMENTS,
    VERTICAL_ALIGNS,
};
use crate::{
    BorderStyle, HorizontalAlignment, PatternType, Underline, VerticalAlign, VerticalAlignment,
};

#[test]
fn each_value_is_spelled_as_the_standard_does() {
    use Underline::{Double, DoubleAccounting, None, Single, SingleAccounting};
    let underlines = [Single, Double, SingleAccounting, DoubleAccounting, None];
    assert_eq!(
        underlines.map(Underline::as_str),
        [
            "single",
            "double",
            "singleAccounting",
            "doubleAccounting",
            "none"
        ]
    );
    assert_eq!(UNDERLINES, underlines.map(Underline::as_str));

    let aligns = [
        VerticalAlign::Baseline,
        VerticalAlign::Superscript,
        VerticalAlign::Subscript,
    ];
    assert_eq!(VERTICAL_ALIGNS, aligns.map(VerticalAlign::as_str));
    assert_eq!(VERTICAL_ALIGNS, ["baseline", "superscript", "subscript"]);
}

#[test]
fn a_pattern_type_has_nineteen_values() {
    use PatternType::*;
    let all = [
        None,
        Solid,
        MediumGray,
        DarkGray,
        LightGray,
        DarkHorizontal,
        DarkVertical,
        DarkDown,
        DarkUp,
        DarkGrid,
        DarkTrellis,
        LightHorizontal,
        LightVertical,
        LightDown,
        LightUp,
        LightGrid,
        LightTrellis,
        Gray125,
        Gray0625,
    ];
    assert_eq!(PATTERN_TYPES, all.map(PatternType::as_str));
    assert_eq!(PATTERN_TYPES.len(), 19);
    assert_eq!(Solid.as_str(), "solid");
    assert_eq!(Gray0625.as_str(), "gray0625");
}

#[test]
fn a_border_style_has_fourteen_values() {
    use BorderStyle::*;
    let all = [
        None,
        Thin,
        Medium,
        Dashed,
        Dotted,
        Thick,
        Double,
        Hair,
        MediumDashed,
        DashDot,
        MediumDashDot,
        DashDotDot,
        MediumDashDotDot,
        SlantDashDot,
    ];
    assert_eq!(BORDER_STYLES, all.map(BorderStyle::as_str));
    assert_eq!(BORDER_STYLES.len(), 14);
    assert_eq!(MediumDashDotDot.as_str(), "mediumDashDotDot");
}

#[test]
fn the_alignments_have_eight_and_five_values() {
    use HorizontalAlignment::*;
    let horizontal = [
        General,
        Left,
        Center,
        Right,
        Fill,
        Justify,
        CenterContinuous,
        Distributed,
    ];
    assert_eq!(
        HORIZONTAL_ALIGNMENTS,
        horizontal.map(HorizontalAlignment::as_str)
    );
    assert_eq!(HORIZONTAL_ALIGNMENTS.len(), 8);
    assert_eq!(CenterContinuous.as_str(), "centerContinuous");
    let vertical = [
        VerticalAlignment::Top,
        VerticalAlignment::Center,
        VerticalAlignment::Bottom,
        VerticalAlignment::Justify,
        VerticalAlignment::Distributed,
    ];
    assert_eq!(VERTICAL_ALIGNMENTS, vertical.map(VerticalAlignment::as_str));
    assert_eq!(VERTICAL_ALIGNMENTS.len(), 5);
}
