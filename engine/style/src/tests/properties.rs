//! `StylesEdit` over a small styles part with random styles: the same style
//! asked twice is at the same index, and the part that is written, read
//! again, gives the same indexes to the same styles and adds nothing.

use crate::{
    Alignment, Border, BorderSide, BorderStyle, Color, Fill, Font, HorizontalAlignment,
    PatternType, Protection, Style, StyleLimits, StylesEdit, Underline, VerticalAlign,
    VerticalAlignment,
};
use proptest::prelude::*;

const PART: &str = concat!(
    r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    r#"<fonts count="1"><font><sz val="11"/><name val="Calibri"/><scheme val="minor"/></font></fonts>"#,
    r#"<fills count="2"><fill><patternFill patternType="none"/></fill>"#,
    r#"<fill><patternFill patternType="gray125"/></fill></fills>"#,
    r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>"#,
    r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
    r#"<cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs>"#,
    "</styleSheet>"
);

fn color() -> impl Strategy<Value = Color> {
    prop_oneof![
        any::<[u8; 4]>().prop_map(Color::Rgb),
        (0..12_u32, proptest::option::of(-1.0..=1.0_f64))
            .prop_map(|(index, tint)| Color::Theme { index, tint }),
    ]
}

fn pick<T: Clone + std::fmt::Debug + 'static>(items: &[T]) -> impl Strategy<Value = T> + use<T> {
    proptest::sample::select(items.to_vec())
}

fn font() -> impl Strategy<Value = Font> {
    let names = ["Arial", "Calibri", "a&b<c>\"d'", "表 é"];
    let underline = pick(&[
        Underline::None,
        Underline::Single,
        Underline::Double,
        Underline::SingleAccounting,
        Underline::DoubleAccounting,
    ]);
    let align = pick(&[
        VerticalAlign::Baseline,
        VerticalAlign::Superscript,
        VerticalAlign::Subscript,
    ]);
    (
        proptest::option::of(pick(&names).prop_map(String::from)),
        proptest::option::of(1.0..=409.0_f64),
        proptest::option::of(any::<bool>()),
        proptest::option::of(any::<bool>()),
        proptest::option::of(any::<bool>()),
        (
            proptest::option::of(underline),
            proptest::option::of(color()),
        ),
        proptest::option::of(align),
    )
        .prop_map(
            |(name, size, bold, italic, strike, (underline, color), vertical_align)| Font {
                name,
                size,
                bold,
                italic,
                strike,
                underline,
                color,
                vertical_align,
            },
        )
}

fn fill() -> impl Strategy<Value = Fill> {
    let patterns = [
        PatternType::None,
        PatternType::Solid,
        PatternType::MediumGray,
        PatternType::Gray125,
        PatternType::LightTrellis,
    ];
    (
        pick(&patterns),
        proptest::option::of(color()),
        proptest::option::of(color()),
    )
        .prop_map(|(pattern, foreground, background)| Fill::Pattern {
            pattern,
            foreground,
            background,
        })
}

fn side() -> impl Strategy<Value = BorderSide> {
    let styles = [
        BorderStyle::None,
        BorderStyle::Thin,
        BorderStyle::Thick,
        BorderStyle::DashDot,
        BorderStyle::Hair,
    ];
    (pick(&styles), proptest::option::of(color()))
        .prop_map(|(style, color)| BorderSide { style, color })
}

fn border() -> impl Strategy<Value = Border> {
    let sides = proptest::collection::vec(proptest::option::of(side()), 5);
    let flags = (
        proptest::option::of(any::<bool>()),
        proptest::option::of(any::<bool>()),
    );
    (sides, flags).prop_map(|(sides, (diagonal_up, diagonal_down))| Border {
        left: sides[0],
        right: sides[1],
        top: sides[2],
        bottom: sides[3],
        diagonal: sides[4],
        diagonal_up,
        diagonal_down,
    })
}

fn alignment() -> impl Strategy<Value = Alignment> {
    let horizontal = pick(&[
        HorizontalAlignment::General,
        HorizontalAlignment::Left,
        HorizontalAlignment::Center,
        HorizontalAlignment::Distributed,
    ]);
    let vertical = pick(&[
        VerticalAlignment::Top,
        VerticalAlignment::Center,
        VerticalAlignment::Bottom,
    ]);
    let rotation = prop_oneof![0..=180_u8, Just(255_u8)];
    (
        proptest::option::of(horizontal),
        proptest::option::of(vertical),
        (
            proptest::option::of(any::<bool>()),
            proptest::option::of(any::<bool>()),
        ),
        proptest::option::of(0..16_u32),
        proptest::option::of(rotation),
    )
        .prop_map(
            |(horizontal, vertical, (wrap_text, shrink_to_fit), indent, text_rotation)| Alignment {
                horizontal,
                vertical,
                wrap_text,
                shrink_to_fit,
                indent,
                text_rotation,
            },
        )
}

fn style() -> impl Strategy<Value = Style> {
    let codes = [
        "General",
        "0.00",
        "@",
        "yyyy-mm-dd",
        "[h]:mm:ss",
        "0.0%",
        "#,##0.00 \"kr\"",
        "0.0000",
    ];
    let protection = (
        proptest::option::of(any::<bool>()),
        proptest::option::of(any::<bool>()),
    )
        .prop_map(|(locked, hidden)| Protection { locked, hidden });
    (
        proptest::option::of(font()),
        proptest::option::of(fill()),
        proptest::option::of(border()),
        proptest::option::of(pick(&codes).prop_map(String::from)),
        proptest::option::of(alignment()),
        proptest::option::of(protection),
    )
        .prop_map(
            |(font, fill, border, number_format, alignment, protection)| Style {
                font,
                fill,
                border,
                number_format,
                alignment,
                protection,
            },
        )
}

proptest! {
    #[test]
    fn the_same_style_asked_twice_is_at_one_index(style in style()) {
        let mut edit = StylesEdit::read(PART.as_bytes(), &StyleLimits::default()).unwrap();
        let first = edit.cell_format(0, &style).unwrap();
        let mut out = Vec::new();
        edit.rewrite(&mut PART.as_bytes(), &mut out).unwrap();
        prop_assert_eq!(edit.cell_format(0, &style).unwrap(), first);
        let mut again = Vec::new();
        edit.rewrite(&mut PART.as_bytes(), &mut again).unwrap();
        prop_assert_eq!(again, out);
    }

    #[test]
    fn the_part_written_is_read_again_to_the_same_indexes(styles in proptest::collection::vec(style(), 1..4)) {
        let limits = StyleLimits::default();
        let mut edit = StylesEdit::read(PART.as_bytes(), &limits).unwrap();
        let indexes: Vec<u32> = styles.iter().map(|s| edit.cell_format(0, s).unwrap()).collect();
        let mut out = Vec::new();
        edit.rewrite(&mut PART.as_bytes(), &mut out).unwrap();
        let mut again = StylesEdit::read(&out, &limits).unwrap();
        let second: Vec<u32> = styles.iter().map(|s| again.cell_format(0, s).unwrap()).collect();
        prop_assert_eq!(second, indexes);
        let mut rewritten = Vec::new();
        again.rewrite(&mut out.as_slice(), &mut rewritten).unwrap();
        prop_assert_eq!(rewritten, out);
    }
}
