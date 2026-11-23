//! Stating a style on an item: each field when it is `Some` and when it is
//! `None`, the settings that `Some` removes, and what the model does not name
//! staying. After a style is stated, the item equals the item read from the
//! XML that has the same effect.

use super::node;
use crate::canonical::Node;
use crate::canonical::overlay::{
    apply_alignment, apply_border, apply_font, apply_protection, fill_node,
};
use crate::canonical::schema;
use crate::{
    Alignment, Border, BorderSide, BorderStyle, Color, Fill, Font, HorizontalAlignment,
    PatternType, Protection, Underline, VerticalAlign, VerticalAlignment,
};

/// The font `base` with `font` stated, which must equal the font `expected`.
fn font_is(base: &str, font: &Font, expected: &str) {
    let mut item = node(base);
    apply_font(&mut item, font);
    assert_eq!(item, node(expected));
}

/// The border `base` with `border` stated, which must equal `expected`.
fn border_is(base: &str, border: &Border, expected: &str) {
    let mut item = node(base);
    apply_border(&mut item, border);
    assert_eq!(item, node(expected));
}

/// The cell format `base` with `alignment` stated, which must equal `expected`.
fn alignment_is(base: &str, alignment: &Alignment, expected: &str) {
    let mut item = node(base);
    apply_alignment(&mut item, alignment);
    assert_eq!(item, node(expected));
}

fn side(style: BorderStyle, color: Option<Color>) -> Option<BorderSide> {
    Some(BorderSide { style, color })
}

#[test]
fn a_font_with_nothing_stated_is_the_base() {
    let base = r#"<font><b/><sz val="11"/><color theme="1"/><name val="Calibri"/><scheme val="minor"/></font>"#;
    font_is(base, &Font::default(), base);
}

#[test]
fn every_font_field_is_stated() {
    let font = Font {
        name: Some("Arial".into()),
        size: Some(12.5),
        bold: Some(true),
        italic: Some(true),
        strike: Some(true),
        underline: Some(Underline::DoubleAccounting),
        color: Some(Color::Rgb([255, 255, 0, 0])),
        vertical_align: Some(VerticalAlign::Superscript),
    };
    font_is(
        "<font/>",
        &font,
        concat!(
            r#"<font><b/><i/><strike/><u val="doubleAccounting"/><vertAlign val="superscript"/>"#,
            r#"<sz val="12.5"/><color rgb="FFFF0000"/><name val="Arial"/></font>"#,
        ),
    );
}

#[test]
fn a_field_stated_replaces_the_bases_and_leaves_the_others() {
    let base = r#"<font><b/><i/><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/></font>"#;
    let font = Font {
        size: Some(9.0),
        color: Some(Color::Theme {
            index: 4,
            tint: Some(0.4),
        }),
        ..Font::default()
    };
    font_is(
        base,
        &font,
        r#"<font><b/><i/><sz val="9"/><color theme="4" tint="0.4"/><name val="Calibri"/></font>"#,
    );
}

#[test]
fn stating_a_name_removes_the_theme_font_scheme() {
    let font = Font {
        name: Some("Arial".into()),
        ..Font::default()
    };
    let base = r#"<font><name val="Calibri"/><family val="2"/><scheme val="minor"/></font>"#;
    font_is(
        base,
        &font,
        r#"<font><name val="Arial"/><family val="2"/></font>"#,
    );
    let size = Font {
        size: Some(10.0),
        ..Font::default()
    };
    font_is(
        base,
        &size,
        r#"<font><sz val="10"/><name val="Calibri"/><family val="2"/><scheme val="minor"/></font>"#,
    );
}

#[test]
fn some_false_none_and_baseline_remove_the_setting_from_the_base() {
    let base = r#"<font><b/><i/><strike/><u/><vertAlign val="subscript"/></font>"#;
    let font = Font {
        bold: Some(false),
        italic: Some(false),
        strike: Some(false),
        underline: Some(Underline::None),
        vertical_align: Some(VerticalAlign::Baseline),
        ..Font::default()
    };
    font_is(base, &font, "<font/>");
    let single = Font {
        underline: Some(Underline::Single),
        ..Font::default()
    };
    font_is(
        "<font><u val=\"double\"/></font>",
        &single,
        "<font><u/></font>",
    );
    let bold = Font {
        bold: Some(true),
        ..Font::default()
    };
    font_is(r#"<font><b val="0"/></font>"#, &bold, "<font><b/></font>");
}

#[test]
fn a_foreign_attribute_a_raw_child_and_a_table_element_the_model_lacks_stay() {
    let base = r#"<font x:a="1"><b/><x:c xmlns:x="urn:x"/><charset val="128"/></font>"#;
    let font = Font {
        bold: Some(false),
        name: Some("Arial".into()),
        ..Font::default()
    };
    font_is(
        base,
        &font,
        r#"<font x:a="1"><x:c xmlns:x="urn:x"/><name val="Arial"/><charset val="128"/></font>"#,
    );
}

#[test]
fn a_font_property_that_has_no_effect_in_the_base_is_removed() {
    let font = Font {
        size: Some(10.0),
        ..Font::default()
    };
    font_is(
        r#"<font><condense val="0"/></font>"#,
        &font,
        r#"<font><sz val="10"/></font>"#,
    );
}

#[test]
fn every_kind_of_underline_is_stated() {
    for (underline, text) in [
        (Underline::Double, "double"),
        (Underline::SingleAccounting, "singleAccounting"),
        (Underline::DoubleAccounting, "doubleAccounting"),
    ] {
        let font = Font {
            underline: Some(underline),
            ..Font::default()
        };
        font_is(
            "<font/>",
            &font,
            &format!(r#"<font><u val="{text}"/></font>"#),
        );
    }
}

#[test]
fn a_raw_child_of_a_cell_format_stays_through_alignment_and_protection() {
    let base =
        r#"<xf numFmtId="1"><alignment horizontal="left"/><extLst><ext uri="{1}"/></extLst></xf>"#;
    let mut item = node(base);
    let alignment = Alignment {
        wrap_text: Some(true),
        ..Alignment::default()
    };
    apply_alignment(&mut item, &alignment);
    let hidden = Protection {
        hidden: Some(true),
        ..Protection::default()
    };
    apply_protection(&mut item, &hidden);
    let expected = concat!(
        r#"<xf numFmtId="1"><alignment horizontal="left" wrapText="1"/>"#,
        r#"<protection hidden="1"/><extLst><ext uri="{1}"/></extLst></xf>"#,
    );
    assert_eq!(item, node(expected));
}

#[test]
fn a_raw_child_and_a_foreign_attribute_of_a_border_stay() {
    let base = r#"<border x:a="1"><x:c xmlns:x="urn:x"/><left style="thin"/></border>"#;
    let border = Border {
        left: side(BorderStyle::None, None),
        top: side(BorderStyle::Thick, None),
        ..Border::default()
    };
    border_is(
        base,
        &border,
        r#"<border x:a="1"><x:c xmlns:x="urn:x"/><top style="thick"/></border>"#,
    );
}

#[test]
fn a_color_is_stated_by_its_kind() {
    let fill = |color: Color| {
        fill_node(&Fill::Pattern {
            pattern: PatternType::Solid,
            foreground: Some(color),
            background: None,
        })
    };
    let expected = |attributes: &str| {
        node(&format!(
            r#"<fill><patternFill patternType="solid"><fgColor {attributes}/></patternFill></fill>"#
        ))
    };
    assert_eq!(
        fill(Color::Rgb([0, 1, 171, 255])),
        expected(r#"rgb="0001ABFF""#)
    );
    let plain = Color::Theme {
        index: 0,
        tint: None,
    };
    assert_eq!(fill(plain), expected(r#"theme="0""#));
    let zero = Color::Theme {
        index: 3,
        tint: Some(-0.0),
    };
    assert_eq!(fill(zero), expected(r#"theme="3""#));
    let tinted = Color::Theme {
        index: 3,
        tint: Some(-0.25),
    };
    assert_eq!(fill(tinted), expected(r#"theme="3" tint="-0.25""#));
}

#[test]
fn a_fill_is_stated_whole() {
    let fill = Fill::Pattern {
        pattern: PatternType::Solid,
        foreground: Some(Color::Rgb([255, 255, 255, 0])),
        background: Some(Color::Theme {
            index: 1,
            tint: None,
        }),
    };
    assert_eq!(
        fill_node(&fill),
        node(concat!(
            r#"<fill><patternFill patternType="solid"><fgColor rgb="FFFFFF00"/>"#,
            r#"<bgColor theme="1"/></patternFill></fill>"#,
        ))
    );
    let none = Fill::Pattern {
        pattern: PatternType::None,
        foreground: None,
        background: None,
    };
    assert_eq!(
        fill_node(&none),
        node(r#"<fill><patternFill patternType="none"/></fill>"#)
    );
}

#[test]
fn a_border_side_is_replaced_whole_and_the_others_stay() {
    let border = Border {
        left: side(BorderStyle::Thin, Some(Color::Rgb([255, 0, 0, 0]))),
        diagonal: side(BorderStyle::Hair, None),
        diagonal_up: Some(true),
        diagonal_down: Some(false),
        ..Border::default()
    };
    border_is(
        r#"<border><left style="thick"><color indexed="64"/></left><top style="double"/></border>"#,
        &border,
        concat!(
            r#"<border diagonalUp="1" diagonalDown="0"><left style="thin"><color rgb="FF000000"/></left>"#,
            r#"<top style="double"/><diagonal style="hair"/></border>"#,
        ),
    );
    let all = Border {
        right: side(BorderStyle::Dotted, None),
        top: side(BorderStyle::Dashed, None),
        bottom: side(BorderStyle::MediumDashDotDot, None),
        ..Border::default()
    };
    border_is(
        "<border/>",
        &all,
        r#"<border><right style="dotted"/><top style="dashed"/><bottom style="mediumDashDotDot"/></border>"#,
    );
}

#[test]
fn a_side_with_no_line_is_removed_with_its_color() {
    let border = Border {
        left: side(BorderStyle::None, Some(Color::Rgb([255, 0, 0, 0]))),
        right: side(BorderStyle::None, None),
        ..Border::default()
    };
    border_is(
        r#"<border><left style="thin"/><right style="thin"/><top style="thin"/></border>"#,
        &border,
        r#"<border><top style="thin"/></border>"#,
    );
    border_is("<border/>", &border, "<border/>");
}

#[test]
fn a_border_with_nothing_stated_is_the_base() {
    let base = r#"<border diagonalUp="1"><left style="thin"/></border>"#;
    border_is(base, &Border::default(), base);
}

#[test]
fn every_alignment_field_is_stated() {
    let alignment = Alignment {
        horizontal: Some(HorizontalAlignment::CenterContinuous),
        vertical: Some(VerticalAlignment::Distributed),
        wrap_text: Some(true),
        shrink_to_fit: Some(true),
        indent: Some(2),
        text_rotation: Some(255),
    };
    alignment_is(
        "<xf/>",
        &alignment,
        concat!(
            r#"<xf><alignment horizontal="centerContinuous" vertical="distributed" "#,
            r#"textRotation="255" wrapText="1" indent="2" shrinkToFit="1"/></xf>"#,
        ),
    );
}

#[test]
fn an_alignment_keeps_the_base_attributes_it_does_not_state() {
    let base = r#"<xf numFmtId="3"><alignment horizontal="left" readingOrder="1" x:a="1"/></xf>"#;
    let alignment = Alignment {
        horizontal: Some(HorizontalAlignment::Right),
        wrap_text: Some(false),
        ..Alignment::default()
    };
    alignment_is(
        base,
        &alignment,
        r#"<xf numFmtId="3"><alignment horizontal="right" wrapText="0" readingOrder="1" x:a="1"/></xf>"#,
    );
    alignment_is(base, &Alignment::default(), base);
}

#[test]
fn an_alignment_that_has_no_effect_is_not_written() {
    let alignment = Alignment {
        vertical: Some(VerticalAlignment::Bottom),
        ..Alignment::default()
    };
    alignment_is("<xf/>", &alignment, "<xf/>");
    alignment_is("<xf/>", &Alignment::default(), "<xf/>");
}

#[test]
fn protection_is_stated_on_the_cell_format() {
    let mut item = node(r#"<xf><protection locked="0"/></xf>"#);
    let hidden = Protection {
        hidden: Some(true),
        ..Protection::default()
    };
    apply_protection(&mut item, &hidden);
    assert_eq!(
        item,
        node(r#"<xf><protection locked="0" hidden="1"/></xf>"#)
    );
    let locked = Protection {
        locked: Some(true),
        hidden: None,
    };
    apply_protection(&mut item, &locked);
    assert_eq!(
        item,
        node(r#"<xf><protection locked="1" hidden="1"/></xf>"#)
    );
    apply_protection(&mut item, &Protection::default());
    assert_eq!(
        item,
        node(r#"<xf><protection locked="1" hidden="1"/></xf>"#)
    );
    let mut empty = node("<xf/>");
    apply_protection(&mut empty, &Protection::default());
    assert_eq!(empty, node("<xf/>"));
}

#[test]
fn a_child_is_removed_by_name_with_the_raw_one_at_the_rank_of_that_name() {
    let mut item =
        node(r#"<fill><patternFill patternType="solid"/><gradientFill degree="1"/></fill>"#);
    item.set_child("patternFill", None);
    assert_eq!(item, node(r#"<fill><gradientFill degree="1"/></fill>"#));
    item.set_child("gradientFill", None);
    assert_eq!(item, node("<fill/>"));
    let mut other = node(r#"<font><x:a xmlns:x="urn:x"/><b/></font>"#);
    other.set_child("nothing", None);
    assert_eq!(other, node(r#"<font><x:a xmlns:x="urn:x"/><b/></font>"#));
    assert!(other.child("b").is_some());
    assert!(other.child("i").is_none());
}

#[test]
fn a_child_set_replaces_the_raw_child_at_the_rank_of_that_name() {
    let mut item = node(r#"<fill><gradientFill degree="1"/></fill>"#);
    item.push_raw(Some(b"patternFill"), b"<patternFill/>".to_vec());
    let solid = Fill::Pattern {
        pattern: PatternType::Solid,
        foreground: None,
        background: None,
    };
    let pattern = fill_node(&solid).child("patternFill").cloned();
    item.set_child("patternFill", pattern);
    assert_eq!(
        item,
        node(r#"<fill><patternFill patternType="solid"/><gradientFill degree="1"/></fill>"#)
    );
}

#[test]
fn an_attribute_outside_the_table_is_not_set() {
    let bold = node("<font><b/></font>");
    let expected = bold.child("b");
    let mut item = Node::new(&schema::B);
    item.set_attribute("bogus", Some(crate::canonical::Value::Bool(false)));
    assert_eq!(Some(&item), expected);
    item.set_attribute("val", Some(crate::canonical::Value::Bool(false)));
    item.set_attribute("val", None);
    assert_eq!(Some(&item), expected);
}
