//! `cell_format`: each field of a style, what is reused instead of added, the
//! limits and the range checks (which leave the part as it was), and the base
//! cell format.

use super::{NS, base, part, read, rewritten};
use crate::edit::{StyleError, StyleKind, StyleLimits, StylesEdit};
use crate::{
    Alignment, Border, BorderSide, BorderStyle, Color, Fill, Font, HorizontalAlignment,
    PatternType, Protection, Style, Underline, VerticalAlign,
};

/// `edit.cell_format(0, style)`, which must succeed.
fn add(edit: &mut StylesEdit, style: &Style) -> u32 {
    edit.cell_format(0, style).unwrap()
}

fn bold() -> Style {
    Style {
        font: Some(Font {
            bold: Some(true),
            ..Font::default()
        }),
        ..Style::default()
    }
}

fn solid(color: Color) -> Style {
    Style {
        fill: Some(Fill::Pattern {
            pattern: PatternType::Solid,
            foreground: Some(color),
            background: None,
        }),
        ..Style::default()
    }
}

fn thin_bottom(color: Option<Color>) -> Style {
    Style {
        border: Some(Border {
            bottom: Some(BorderSide {
                style: BorderStyle::Thin,
                color,
            }),
            ..Border::default()
        }),
        ..Style::default()
    }
}

/// Asserts that `style` is refused for `field` and leaves `xml` as it was.
fn invalid(style: &Style, field: &str) {
    let xml = base();
    let mut edit = read(&xml);
    match edit.cell_format(0, style) {
        Err(StyleError::InvalidValue { field: found }) => assert_eq!(found, field, "{style:?}"),
        other => panic!("{style:?}: {other:?}"),
    }
    assert_eq!(rewritten(&edit, &xml), xml);
}

#[test]
fn a_number_format_the_part_lacks_is_added_with_the_next_id() {
    let xml = base();
    let mut edit = read(&xml);
    let index = add(&mut edit, &Style::number_format("0.0000"));
    assert_eq!(index, 2);
    let out = rewritten(&edit, &xml);
    assert!(out.contains(
        r#"<numFmts count="2"><numFmt numFmtId="164" formatCode="0.000"/><numFmt numFmtId="165" formatCode="0.0000"/></numFmts>"#
    ));
    assert!(out.contains(
        r#"<cellXfs count="3"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/><xf numFmtId="165" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/></cellXfs>"#
    ));
    assert_eq!(add(&mut edit, &Style::number_format("0.0000")), index);
    assert_eq!(rewritten(&edit, &xml), out);
    let another = add(&mut edit, &Style::number_format("0.00000"));
    assert_eq!(another, 3);
    assert!(rewritten(&edit, &xml).contains(r#"<numFmt numFmtId="166" formatCode="0.00000"/>"#));
}

#[test]
fn a_number_format_the_part_defines_is_reused() {
    let mut edit = read(&base());
    // The cell format 1 is the base cell format with the format 164.
    assert_eq!(add(&mut edit, &Style::number_format("0.000")), 1);
    assert_eq!(edit.scan.numbers.added.len(), 0);
}

#[test]
fn a_built_in_number_format_is_its_id_and_adds_no_definition() {
    let xml = base();
    let mut edit = read(&xml);
    assert_eq!(add(&mut edit, &Style::number_format("0.00")), 2);
    assert_eq!(add(&mut edit, &Style::number_format("@")), 3);
    // General is the id 0, which the cell format 0 has, but without
    // applyNumberFormat: a cell format of its own.
    assert_eq!(add(&mut edit, &Style::number_format("General")), 4);
    let out = rewritten(&edit, &xml);
    assert!(out.contains(r#"<numFmts count="1">"#));
    assert!(out.contains(
        r#"<xf numFmtId="2" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>"#
    ));
    assert!(out.contains(
        r#"<xf numFmtId="49" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>"#
    ));
    assert!(out.contains(r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/></cellXfs>"#));
}

#[test]
fn a_built_in_id_the_part_defines_itself_is_not_used() {
    let xml = part(&[
        r#"<numFmts count="1"><numFmt numFmtId="2" formatCode="0.0"/></numFmts>"#,
        r#"<cellXfs count="1"><xf numFmtId="0"/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    let index = add(&mut edit, &Style::number_format("0.00"));
    assert_eq!(index, 1);
    let out = rewritten(&edit, &xml);
    // 164: the first id after those reserved for the built-in formats.
    assert!(
        out.contains(r#"<numFmt numFmtId="164" formatCode="0.00"/>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<xf numFmtId="164" applyNumberFormat="1"/></cellXfs>"#),
        "{out}"
    );
    // The format 0.0 the part defines under 2 is found by its code.
    assert_eq!(add(&mut edit, &Style::number_format("0.0")), 2);
    assert!(
        rewritten(&edit, &xml).contains(r#"<xf numFmtId="2" applyNumberFormat="1"/></cellXfs>"#)
    );
}

#[test]
fn the_new_number_format_id_is_greater_than_every_id_in_the_part() {
    let xml = part(&[
        r#"<dxfs><dxf><numFmt numFmtId="300" formatCode="x"/></dxf></dxfs>"#,
        r#"<cellXfs count="1"><xf/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    add(&mut edit, &Style::number_format("0.0"));
    assert!(rewritten(&edit, &xml).contains(r#"<numFmt numFmtId="301" formatCode="0.0"/>"#));
    let small = part(&[
        r#"<numFmts><numFmt numFmtId="5" formatCode="x"/></numFmts><cellXfs><xf/></cellXfs>"#,
    ]);
    let mut edit = read(&small);
    add(&mut edit, &Style::number_format("0.0"));
    assert!(rewritten(&edit, &small).contains(r#"<numFmt numFmtId="164" formatCode="0.0"/>"#));
}

#[test]
fn a_new_number_format_id_that_does_not_fit_in_a_u32_is_too_many() {
    let xml = part(&[
        r#"<numFmts count="1"><numFmt numFmtId="4294967295" formatCode="x"/></numFmts>"#,
        r#"<cellXfs count="1"><xf/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    match edit.cell_format(0, &Style::number_format("0.0")) {
        Err(StyleError::TooMany { kind, limit }) => {
            assert_eq!((kind, limit), (StyleKind::NumberFormat, 200));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(rewritten(&edit, &xml), xml);
    assert_eq!(add(&mut edit, &Style::number_format("x")), 1);
}

#[test]
fn a_font_is_the_base_font_with_the_style_laid_over_it() {
    let xml = base();
    let mut edit = read(&xml);
    let style = Style {
        font: Some(Font {
            name: Some("Arial".into()),
            size: Some(12.5),
            bold: Some(true),
            italic: Some(true),
            strike: Some(true),
            underline: Some(Underline::Double),
            color: Some(Color::Rgb([0xFF, 0x12, 0xAB, 0xCD])),
            vertical_align: Some(VerticalAlign::Superscript),
        }),
        ..Style::default()
    };
    assert_eq!(add(&mut edit, &style), 2);
    let out = rewritten(&edit, &xml);
    assert!(out.contains(concat!(
        r#"<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font>"#,
        r#"<font><b/><i/><strike/><u val="double"/><vertAlign val="superscript"/><sz val="12.5"/>"#,
        r#"<color rgb="FF12ABCD"/><name val="Arial"/></font></fonts>"#,
    )), "{out}");
    assert!(out.contains(
        r#"<xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs>"#
    ));
    let calibri = Style {
        font: Some(Font {
            size: Some(11.0),
            ..Font::default()
        }),
        ..Style::default()
    };
    // The font is the base's own, so no font is added; the cell format is
    // not the base's, for it has applyFont.
    assert_eq!(add(&mut edit, &calibri), 3);
    assert_eq!(edit.scan.fonts.added().len(), 1);
}

#[test]
fn a_fill_is_added_after_the_reserved_ones_and_reused() {
    let xml = base();
    let mut edit = read(&xml);
    let index = add(
        &mut edit,
        &solid(Color::Theme {
            index: 4,
            tint: Some(0.5),
        }),
    );
    assert_eq!(index, 2);
    let out = rewritten(&edit, &xml);
    assert!(out.contains(concat!(
        r#"<fills count="3"><fill><patternFill patternType="none"/></fill>"#,
        r#"<fill><patternFill patternType="gray125"/></fill>"#,
        r#"<fill><patternFill patternType="solid"><fgColor theme="4" tint="0.5"/></patternFill></fill></fills>"#,
    )), "{out}");
    assert!(out.contains(r#"fillId="2" borderId="0" xfId="0" applyFill="1"/></cellXfs>"#));
    assert_eq!(
        add(
            &mut edit,
            &solid(Color::Theme {
                index: 4,
                tint: Some(0.5)
            })
        ),
        index
    );
    let none = Style {
        fill: Some(Fill::Pattern {
            pattern: PatternType::None,
            foreground: None,
            background: None,
        }),
        ..Style::default()
    };
    // The reserved fill 0 is the fill none: none is added, only the cell
    // format is.
    assert_eq!(add(&mut edit, &none), 3);
    assert_eq!(edit.scan.fills.added().len(), 1);
}

#[test]
fn the_reserved_fills_are_added_when_the_part_has_fewer_than_two() {
    let cell_xfs = r#"<cellXfs count="1"><xf/></cellXfs>"#;
    let reserved = concat!(
        r#"<fill><patternFill patternType="none"/></fill>"#,
        r#"<fill><patternFill patternType="gray125"/></fill>"#
    );
    let style = solid(Color::Rgb([0xFF, 0, 0, 0]));
    let new =
        r#"<fill><patternFill patternType="solid"><fgColor rgb="FF000000"/></patternFill></fill>"#;
    let cases = [
        (
            part(&[cell_xfs]),
            format!(r#"<fills count="3">{reserved}{new}</fills>"#),
        ),
        (
            part(&[r#"<fills count="0"/>"#, cell_xfs]),
            format!(r#"<fills count="3">{reserved}{new}</fills>"#),
        ),
        (
            part(&[
                r#"<fills count="1"><fill><patternFill patternType="none"/></fill></fills>"#,
                cell_xfs,
            ]),
            format!(
                r#"<fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>{new}</fills>"#
            ),
        ),
    ];
    for (xml, expected) in cases {
        let mut edit = read(&xml);
        assert_eq!(add(&mut edit, &style), 1);
        let out = rewritten(&edit, &xml);
        assert!(out.contains(&expected), "{out}");
        assert!(out.contains(r#"<xf fillId="2" applyFill="1"/>"#), "{out}");
    }
    // The fill none asked for first is the reserved fill 0: the two reserved
    // fills are added, and nothing else is.
    let none = Style {
        fill: Some(Fill::Pattern {
            pattern: PatternType::None,
            foreground: None,
            background: None,
        }),
        ..Style::default()
    };
    let xml = part(&[cell_xfs]);
    let mut edit = read(&xml);
    assert_eq!(add(&mut edit, &none), 1);
    let out = rewritten(&edit, &xml);
    assert!(
        out.contains(&format!(r#"<fills count="2">{reserved}</fills>"#)),
        "{out}"
    );
    assert!(out.contains(r#"<xf fillId="0" applyFill="1"/>"#), "{out}");
}

#[test]
fn a_border_is_the_base_border_with_the_style_laid_over_it() {
    let xml = base();
    let mut edit = read(&xml);
    let index = add(
        &mut edit,
        &thin_bottom(Some(Color::Rgb([0xFF, 0, 0, 0xFF]))),
    );
    assert_eq!(index, 2);
    let out = rewritten(&edit, &xml);
    assert!(out.contains(concat!(
        r#"<borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border>"#,
        r#"<border><left/><right/><top/><bottom style="thin"><color rgb="FF0000FF"/></bottom><diagonal/></border></borders>"#,
    )), "{out}");
    assert!(out.contains(r#"borderId="1" xfId="0" applyBorder="1"/></cellXfs>"#));
    assert_eq!(
        add(
            &mut edit,
            &thin_bottom(Some(Color::Rgb([0xFF, 0, 0, 0xFF])))
        ),
        index
    );
    let top = Style {
        border: Some(Border {
            top: Some(BorderSide {
                style: BorderStyle::Thick,
                color: None,
            }),
            diagonal_up: Some(true),
            ..Border::default()
        }),
        ..Style::default()
    };
    assert_eq!(add(&mut edit, &top), 3);
    // A border laid over a base cell format whose border is the one added.
    let on_new = edit.cell_format(2, &top).unwrap();
    assert_eq!(on_new, 4);
    assert!(rewritten(&edit, &xml).contains(
        r#"<border diagonalUp="1"><left/><right/><top style="thick"/><bottom style="thin"><color rgb="FF0000FF"/></bottom><diagonal/></border>"#
    ));
}

#[test]
fn an_alignment_and_a_protection_are_set_on_the_cell_format() {
    let xml = base();
    let mut edit = read(&xml);
    let style = Style {
        alignment: Some(Alignment {
            horizontal: Some(HorizontalAlignment::Center),
            wrap_text: Some(true),
            text_rotation: Some(255),
            ..Alignment::default()
        }),
        protection: Some(Protection {
            locked: Some(false),
            hidden: None,
        }),
        ..Style::default()
    };
    assert_eq!(add(&mut edit, &style), 2);
    let out = rewritten(&edit, &xml);
    assert!(out.contains(concat!(
        r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1" applyProtection="1">"#,
        r#"<alignment horizontal="center" textRotation="255" wrapText="1"/><protection locked="0"/></xf></cellXfs>"#,
    )), "{out}");
    assert_eq!(add(&mut edit, &style), 2);
    let wrap_only = Style {
        alignment: Some(Alignment {
            indent: Some(2),
            ..Alignment::default()
        }),
        ..Style::default()
    };
    // Laid over the cell format just added, the alignment keeps what it had.
    let index = edit.cell_format(2, &wrap_only).unwrap();
    assert_eq!(index, 3);
    assert!(rewritten(&edit, &xml).contains(
        r#"<alignment horizontal="center" textRotation="255" wrapText="1" indent="2"/>"#
    ));
}

#[test]
fn a_style_that_changes_nothing_is_the_base_cell_format() {
    let mut edit = read(&base());
    assert_eq!(edit.cell_format(0, &Style::default()).unwrap(), 0);
    assert_eq!(edit.cell_format(1, &Style::default()).unwrap(), 1);
    assert_eq!(edit.cell_format(0, &bold()).unwrap(), 2);
    // The bold cell format laid over again, and bold over the bold one.
    assert_eq!(edit.cell_format(0, &bold()).unwrap(), 2);
    assert_eq!(edit.cell_format(2, &bold()).unwrap(), 2);
}

#[test]
fn the_first_of_two_equal_cell_formats_is_used() {
    let xml = part(&[
        r#"<fonts count="1"><font/></fonts>"#,
        r#"<cellXfs count="3"><xf fontId="0"/><xf fontId="0" applyFont="1"/><xf fontId="0" applyFont="1"/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    let style = Style {
        font: Some(Font::default()),
        ..Style::default()
    };
    // The font is the base's own, so the cell format is the base's with
    // applyFont, which the cell formats 1 and 2 are.
    assert_eq!(edit.cell_format(0, &style).unwrap(), 1);
    assert_eq!(edit.cell_format(2, &style).unwrap(), 1);
    assert_eq!(rewritten(&edit, &xml), xml);
}

#[test]
fn the_same_item_in_a_list_twice_is_found_at_its_first_index() {
    let xml = part(&[
        r#"<fonts count="3"><font><b/></font><font><i/></font><font><b/></font></fonts>"#,
        r#"<cellXfs count="1"><xf fontId="2"/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    assert_eq!(add(&mut edit, &bold()), 1);
    assert!(rewritten(&edit, &xml).contains(r#"<xf fontId="0" applyFont="1"/></cellXfs>"#));
}

#[test]
fn a_base_that_refers_past_its_lists_starts_from_an_empty_item() {
    let xml = part(&[
        r#"<fonts count="1"><font><sz val="11"/></font></fonts>"#,
        r#"<borders count="1"><border><left style="thin"/></border></borders>"#,
        r#"<cellXfs count="1"><xf fontId="9" borderId="9" xfId="3"/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    let style = Style {
        border: thin_bottom(None).border,
        ..bold()
    };
    assert_eq!(add(&mut edit, &style), 1);
    let out = rewritten(&edit, &xml);
    assert!(out.contains(r#"<font><b/></font></fonts>"#), "{out}");
    assert!(
        out.contains(
            r#"<border><left/><right/><top/><bottom style="thin"/><diagonal/></border></borders>"#
        ),
        "{out}"
    );
    assert!(
        out.contains(
            r#"<xf fontId="1" borderId="1" xfId="3" applyFont="1" applyBorder="1"/></cellXfs>"#
        ),
        "{out}"
    );
}

#[test]
fn what_the_base_item_holds_beyond_the_model_stays() {
    let xml = part(&[
        r#"<fonts count="1"><font><b/><outline/><scheme val="minor"/><x:e xmlns:x="urn:x"/></font></fonts>"#,
        r#"<cellXfs count="1"><xf fontId="0" x:q="1" xmlns:x="urn:x"><extLst><ext uri="u"/></extLst></xf></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    let style = Style {
        font: Some(Font {
            italic: Some(true),
            ..Font::default()
        }),
        ..Style::default()
    };
    assert_eq!(add(&mut edit, &style), 1);
    let out = rewritten(&edit, &xml);
    assert!(
        out.contains(
            r#"<font><b/><i/><outline/><scheme val="minor"/><x:e xmlns:x="urn:x"/></font></fonts>"#
        ),
        "{out}"
    );
    assert!(out.contains(r#"<xf fontId="1" applyFont="1" x:q="1" xmlns:x="urn:x"><extLst><ext uri="u"/></extLst></xf></cellXfs>"#), "{out}");
}

#[test]
fn a_font_name_removes_the_theme_scheme() {
    let xml = part(&[
        r#"<fonts count="1"><font><name val="Calibri"/><family val="2"/><scheme val="minor"/></font></fonts>"#,
        r#"<cellXfs count="1"><xf fontId="0"/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    let style = Style {
        font: Some(Font {
            name: Some("Arial".into()),
            ..Font::default()
        }),
        ..Style::default()
    };
    assert_eq!(add(&mut edit, &style), 1);
    assert!(
        rewritten(&edit, &xml)
            .contains(r#"<font><name val="Arial"/><family val="2"/></font></fonts>"#)
    );
}

#[test]
fn a_part_with_no_cell_format_accepts_base_0_as_an_empty_one() {
    for (xml, expected) in [
        (
            part(&[
                r#"<cellStyleXfs count="1"><xf/></cellStyleXfs>"#,
                "<cellXfs/>",
            ]),
            r#"<cellXfs><xf numFmtId="2" xfId="0" applyNumberFormat="1"/></cellXfs>"#,
        ),
        (
            part(&["<cellXfs/>"]),
            r#"<cellXfs><xf numFmtId="2" applyNumberFormat="1"/></cellXfs>"#,
        ),
        (
            part(&["<cellStyleXfs/>", "<cellXfs/>"]),
            r#"<cellXfs><xf numFmtId="2" applyNumberFormat="1"/></cellXfs>"#,
        ),
        (
            part(&[r#"<cellStyleXfs count="0"></cellStyleXfs>"#, "<cellXfs/>"]),
            r#"<cellXfs><xf numFmtId="2" applyNumberFormat="1"/></cellXfs>"#,
        ),
        (
            part(&[r#"<cellXfs count="0"></cellXfs>"#]),
            r#"<cellXfs count="1"><xf numFmtId="2" applyNumberFormat="1"/></cellXfs>"#,
        ),
    ] {
        let mut edit = read(&xml);
        assert_eq!(add(&mut edit, &Style::number_format("0.00")), 0);
        let out = rewritten(&edit, &xml);
        assert!(out.contains(expected), "{out}");
        assert!(matches!(
            edit.cell_format(1, &Style::default()),
            Err(StyleError::NoCellFormat { base: 1 })
        ));
    }
}

#[test]
fn a_base_that_is_not_in_the_cell_formats_is_refused() {
    let xml = base();
    let mut edit = read(&xml);
    for base in [2, 3, u32::MAX] {
        match edit.cell_format(base, &bold()) {
            Err(StyleError::NoCellFormat { base: found }) => assert_eq!(found, base),
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(rewritten(&edit, &xml), xml);
    let empty = part(&["<cellXfs/>"]);
    let mut edit = read(&empty);
    assert!(matches!(
        edit.cell_format(1, &bold()),
        Err(StyleError::NoCellFormat { base: 1 })
    ));
}

#[test]
fn a_call_that_is_refused_for_a_limit_adds_nothing() {
    let xml = base();
    let all = Style {
        number_format: Some("0.0000".into()),
        fill: solid(Color::Rgb([0xFF, 1, 2, 3])).fill,
        border: thin_bottom(None).border,
        ..bold()
    };
    let limits = |max_cell_formats, max_fonts, max_number_formats| StyleLimits {
        max_cell_formats,
        max_fonts,
        max_number_formats,
        ..StyleLimits::default()
    };
    let cases = [
        (limits(2, 512, 200), StyleKind::CellFormat, 2),
        (limits(65_490, 1, 200), StyleKind::Font, 1),
        (limits(65_490, 512, 1), StyleKind::NumberFormat, 1),
    ];
    for (limits, kind, max) in cases {
        let mut edit = StylesEdit::read(xml.as_bytes(), &limits).unwrap();
        match edit.cell_format(0, &all) {
            Err(StyleError::TooMany { kind: found, limit }) => {
                assert_eq!((found, limit), (kind, max));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(rewritten(&edit, &xml), xml, "{kind:?}");
    }
    // At the limit exactly, the call is not refused.
    let mut edit = StylesEdit::read(xml.as_bytes(), &limits(3, 2, 2)).unwrap();
    assert_eq!(edit.cell_format(0, &all).unwrap(), 2);
}

#[test]
fn a_part_over_a_limit_accepts_calls_that_add_nothing() {
    let xml = base();
    let limits = StyleLimits {
        max_cell_formats: 0,
        max_fonts: 0,
        max_number_formats: 0,
        ..StyleLimits::default()
    };
    let mut edit = StylesEdit::read(xml.as_bytes(), &limits).unwrap();
    assert_eq!(edit.cell_format(1, &Style::default()).unwrap(), 1);
    assert_eq!(
        edit.cell_format(0, &Style::number_format("0.000")).unwrap(),
        1
    );
    match edit.cell_format(0, &Style::number_format("0.00")) {
        Err(StyleError::TooMany { kind, limit }) => {
            assert_eq!((kind, limit), (StyleKind::CellFormat, 0));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_value_out_of_range_is_refused_with_the_name_of_its_field() {
    let font = |font: Font| Style {
        font: Some(font),
        ..Style::default()
    };
    let rotation = |degrees| Style {
        alignment: Some(Alignment {
            text_rotation: Some(degrees),
            ..Alignment::default()
        }),
        ..Style::default()
    };
    for degrees in [181, 200, 254] {
        invalid(&rotation(degrees), "textRotation");
    }
    for degrees in [0, 90, 180, 255] {
        assert!(read(&base()).cell_format(0, &rotation(degrees)).is_ok());
    }
    for size in [
        0.0,
        0.99,
        409.5,
        1000.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -3.0,
    ] {
        let style = font(Font {
            size: Some(size),
            ..Font::default()
        });
        invalid(&style, "sz");
    }
    for size in [1.0, 11.5, 409.0] {
        let style = font(Font {
            size: Some(size),
            ..Font::default()
        });
        assert!(read(&base()).cell_format(0, &style).is_ok());
    }
    for name in ["", "a\u{1}b", "a\u{fffe}", "\u{0}"] {
        let style = font(Font {
            name: Some(name.into()),
            ..Font::default()
        });
        invalid(&style, "name");
    }
    for code in ["", "a\u{8}", "\u{ffff}"] {
        invalid(&Style::number_format(code), "formatCode");
    }
    let ok = Style {
        number_format: Some("tab\there\n\u{10000}é".into()),
        font: Some(Font {
            name: Some("é\u{10000}".into()),
            ..Font::default()
        }),
        ..Style::default()
    };
    assert!(read(&base()).cell_format(0, &ok).is_ok());
}

#[test]
fn a_tint_out_of_range_is_refused_wherever_a_color_is() {
    let bad = |tint| Color::Theme {
        index: 1,
        tint: Some(tint),
    };
    for tint in [1.01, -1.01, f64::NAN, f64::INFINITY, 100.0] {
        let color = Some(bad(tint));
        let font = Style {
            font: Some(Font {
                color,
                ..Font::default()
            }),
            ..Style::default()
        };
        invalid(&font, "tint");
        let fill = |foreground, background| Style {
            fill: Some(Fill::Pattern {
                pattern: PatternType::Solid,
                foreground,
                background,
            }),
            ..Style::default()
        };
        invalid(&fill(color, None), "tint");
        invalid(&fill(None, color), "tint");
        for side in 0..5 {
            let one = Some(BorderSide {
                style: BorderStyle::Thin,
                color,
            });
            let mut border = Border::default();
            match side {
                0 => border.left = one,
                1 => border.right = one,
                2 => border.top = one,
                3 => border.bottom = one,
                _ => border.diagonal = one,
            }
            let style = Style {
                border: Some(border),
                ..Style::default()
            };
            invalid(&style, "tint");
        }
    }
    for tint in [None, Some(-1.0), Some(0.0), Some(1.0)] {
        let theme = Color::Theme { index: 1, tint };
        let both = Style {
            font: Some(Font {
                color: Some(theme),
                ..Font::default()
            }),
            ..solid(theme)
        };
        assert!(read(&base()).cell_format(0, &both).is_ok());
    }
}

#[test]
fn the_namespace_of_the_part_does_not_change_what_is_added() {
    let xml = format!(
        r#"<x:styleSheet xmlns:x="{NS}"><x:cellXfs count="1"><x:xf/></x:cellXfs></x:styleSheet>"#
    );
    let mut edit = read(&xml);
    assert_eq!(add(&mut edit, &bold()), 1);
    assert_eq!(edit.scan.fonts.added().len(), 1);
    let out = rewritten(&edit, &xml);
    assert!(
        out.contains(r#"<x:fonts count="1"><x:font><x:b/></x:font></x:fonts>"#),
        "{out}"
    );
}

#[test]
fn an_id_the_part_defines_with_different_codes_is_given_to_no_code() {
    let xml = part(&[
        concat!(
            r#"<numFmts count="4"><numFmt numFmtId="2" formatCode="x"/><numFmt numFmtId="2" formatCode="y"/>"#,
            r#"<numFmt numFmtId="170" formatCode="a"/><numFmt numFmtId="170" formatCode="b"/></numFmts>"#
        ),
        r#"<cellXfs count="1"><xf/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    // 0.00 is the built-in 2, which the part redefines: not used. Neither
    // the id 2 nor the id 170 is found for the codes defined under them.
    // The new ids are greater than the greatest id of the part, 170.
    assert_eq!(add(&mut edit, &Style::number_format("0.00")), 1);
    assert_eq!(add(&mut edit, &Style::number_format("x")), 2);
    assert_eq!(add(&mut edit, &Style::number_format("a")), 3);
    assert_eq!(add(&mut edit, &Style::number_format("b")), 4);
    let out = rewritten(&edit, &xml);
    for added in [
        r#"<numFmt numFmtId="171" formatCode="0.00"/>"#,
        r#"<numFmt numFmtId="172" formatCode="x"/>"#,
        r#"<numFmt numFmtId="173" formatCode="a"/>"#,
        r#"<numFmt numFmtId="174" formatCode="b"/>"#,
    ] {
        assert!(out.contains(added), "{out}");
    }
}

#[test]
fn a_number_format_id_an_xf_refers_to_without_a_definition_is_not_given_to_a_new_format() {
    let xml = part(&[r#"<cellXfs count="2"><xf numFmtId="164"/><xf numFmtId="170"/></cellXfs>"#]);
    let mut edit = read(&xml);
    assert_eq!(add(&mut edit, &Style::number_format("0.0000")), 2);
    assert!(rewritten(&edit, &xml).contains(r#"<numFmt numFmtId="171" formatCode="0.0000"/>"#));
}

#[test]
fn a_base_font_id_that_is_not_a_number_starts_from_an_empty_font() {
    let xml = part(&[
        r#"<fonts count="1"><font><sz val="11"/></font></fonts>"#,
        r#"<cellXfs count="2"><xf fontId="x"/><xf/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    // The first font when the xf has no fontId, an empty font when it has
    // one that is not a number.
    assert_eq!(edit.cell_format(1, &bold()).unwrap(), 2);
    assert_eq!(edit.cell_format(0, &bold()).unwrap(), 3);
    let out = rewritten(&edit, &xml);
    assert!(
        out.contains(r#"<font><b/><sz val="11"/></font><font><b/></font></fonts>"#),
        "{out}"
    );
}
