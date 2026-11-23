//! Reading a styles part: the shape of each list, what is refused, the
//! greatest `numFmtId`, and the limit on the items read.

use super::{NS, base, part, read, refused, refused_as, rewritten};
use crate::edit::{StyleError, StyleKind, StyleLimits, StylesEdit};
use crate::{Font, Style};
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};
use xlsx_graft_xml::XmlLimits;

#[test]
fn the_lists_of_a_part_are_read_with_their_shape() {
    let edit = read(&base());
    let scan = &edit.scan;
    assert_eq!(scan.numbers.read, 1);
    assert_eq!(scan.numbers.max_id, 164);
    assert_eq!(scan.fonts.items.len(), 1);
    assert_eq!(scan.fills.items.len(), 2);
    assert_eq!(scan.borders.items.len(), 1);
    assert_eq!(scan.cell_xfs.items.len(), 2);
    assert!(scan.cell_style_xfs);
    for shape in [
        &scan.numbers.shape,
        &scan.fonts.shape,
        &scan.fills.shape,
        &scan.borders.shape,
        &scan.cell_xfs.shape,
    ] {
        assert!(shape.present);
        assert!(shape.count);
        assert_eq!(shape.prefix, b"");
    }
    assert_eq!(scan.prefix, b"");
}

#[test]
fn a_list_the_part_lacks_is_not_present() {
    let edit = read(&part(&["<dxfs/>"]));
    let scan = &edit.scan;
    assert!(!scan.numbers.shape.present);
    assert!(!scan.fonts.shape.present);
    assert!(!scan.fills.shape.present);
    assert!(!scan.borders.shape.present);
    assert!(!scan.cell_xfs.shape.present);
    assert!(!scan.cell_style_xfs);
    assert_eq!(scan.fonts.items.len(), 0);
}

#[test]
fn the_prefix_and_the_count_of_a_list_are_noted() {
    let xml = format!(
        r#"<x:styleSheet xmlns:x="{NS}"><x:fonts><x:font/></x:fonts><x:fills count="1"><x:fill/></x:fills></x:styleSheet>"#
    );
    let edit = read(&xml);
    let scan = &edit.scan;
    assert_eq!(scan.prefix, b"x");
    assert_eq!(scan.fonts.shape.prefix, b"x");
    assert!(!scan.fonts.shape.count);
    assert!(scan.fills.shape.count);
    assert_eq!(scan.fonts.items.len(), 1);
    assert_eq!(scan.fills.items.len(), 1);
}

#[test]
fn a_strict_part_is_read() {
    let xml = r#"<styleSheet xmlns="http://purl.oclc.org/ooxml/spreadsheetml/main"><fonts count="1"><font/></fonts></styleSheet>"#;
    assert_eq!(read(xml).scan.fonts.items.len(), 1);
}

#[test]
fn an_empty_root_and_empty_lists_are_read() {
    let edit = read(&format!(r#"<styleSheet xmlns="{NS}"/>"#));
    assert!(!edit.scan.fonts.shape.present);
    let edit = read(&part(&[
        "<numFmts/>",
        "<fonts/>",
        "<fills/>",
        "<borders/>",
        "<cellXfs/>",
        "<cellStyleXfs/>",
    ]));
    assert!(edit.scan.fonts.shape.present);
    assert!(edit.scan.cell_xfs.shape.present);
    assert!(edit.scan.numbers.shape.present);
    assert!(!edit.scan.cell_style_xfs);
    assert_eq!(edit.scan.cell_xfs.items.len(), 0);
}

#[test]
fn what_the_part_says_beside_its_lists_is_skipped() {
    let xml = format!(
        concat!(
            r#"<?xml version="1.0"?><!--a--><?pi x?>"#,
            r#"<styleSheet xmlns="{ns}" xmlns:x="urn:x"><x:fonts><x:font/></x:fonts>"#,
            r#"<fonts count="1">  <!--b-->  <font><b/></font>  </fonts><x:foo/><colors><a><b/></a></colors>"#,
            "</styleSheet>"
        ),
        ns = NS
    );
    let edit = read(&xml);
    assert_eq!(edit.scan.fonts.items.len(), 1);
}

#[test]
fn a_part_that_is_not_a_style_sheet_is_refused() {
    refused_as(
        &format!(r#"<workbook xmlns="{NS}"/>"#),
        SchemaReason::UnexpectedElement,
    );
    refused_as(
        r#"<styleSheet xmlns="urn:other"/>"#,
        SchemaReason::UnexpectedElement,
    );
    refused_as("<styleSheet/>", SchemaReason::UnexpectedElement);
}

#[test]
fn a_part_that_is_not_xml_is_refused_with_the_diagnostic_of_the_xml_reader() {
    assert!(matches!(
        refused("<styleSheet"),
        Diagnostic::XmlMalformed { .. }
    ));
    assert!(matches!(
        refused(&format!(r#"<styleSheet xmlns="{NS}"><fonts></styleSheet>"#)),
        Diagnostic::XmlMalformed { .. }
    ));
    let limits = StyleLimits {
        xml: XmlLimits {
            max_bytes: xlsx_graft_diagnostics::Limit::default_of(10),
            ..XmlLimits::default()
        },
        ..StyleLimits::default()
    };
    match StylesEdit::read(base().as_bytes(), &limits) {
        Err(StyleError::Invalid(Diagnostic::XmlTooLarge { .. })) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_child_that_is_not_the_item_of_its_list_is_refused() {
    let lists = [
        ("numFmts", "<font/>"),
        ("fonts", "<fill/>"),
        ("fills", "<font/>"),
        ("borders", "<xf/>"),
        ("cellXfs", "<border/>"),
        ("fonts", r#"<mc:AlternateContent xmlns:mc="urn:mc"/>"#),
        ("fonts", r#"<x:font xmlns:x="urn:x"/>"#),
        (
            "numFmts",
            r#"<mc:AlternateContent xmlns:mc="urn:mc"><a/></mc:AlternateContent>"#,
        ),
    ];
    for (list, child) in lists {
        let xml = part(&[&format!("<{list}>{child}</{list}>")]);
        refused_as(&xml, SchemaReason::UnexpectedElement);
        let unknown = part(&[&format!("<{list}><unknown/></{list}>")]);
        refused_as(&unknown, SchemaReason::UnexpectedElement);
    }
}

#[test]
fn text_in_a_list_or_an_item_is_refused() {
    for body in [
        "<fonts>text<font/></fonts>",
        "<numFmts>&amp;</numFmts>",
        "<fonts><![CDATA[x]]></fonts>",
        "<fonts><font>text</font></fonts>",
        "<fills><fill><patternFill>x</patternFill></fill></fills>",
        "<cellXfs><xf><alignment>&#65;</alignment></xf></cellXfs>",
    ] {
        refused_as(&part(&[body]), SchemaReason::UnexpectedText);
    }
    let blank = part(&["<fonts> \n\t<font> <![CDATA[]]> </font> </fonts>"]);
    assert_eq!(read(&blank).scan.fonts.items.len(), 1);
}

#[test]
fn a_list_that_is_there_twice_is_refused() {
    for list in ["numFmts", "fonts", "fills", "borders", "cellXfs"] {
        let xml = part(&[&format!("<{list}/><{list}/>")]);
        refused_as(&xml, SchemaReason::UnexpectedElement);
    }
}

#[test]
fn the_greatest_number_format_id_is_taken_from_the_whole_part() {
    let xml = part(&[
        r#"<numFmts count="2"><numFmt numFmtId="170" formatCode="a"/><numFmt numFmtId="166" formatCode="b"/></numFmts>"#,
        r#"<dxfs count="1"><dxf><numFmt numFmtId="300" formatCode="c"/></dxf></dxfs>"#,
        r#"<extLst><ext xmlns:x14="urn:x14"><x14:dxf><x14:numFmt numFmtId="500" formatCode="d"/></x14:dxf></ext></extLst>"#,
    ]);
    assert_eq!(read(&xml).scan.numbers.max_id, 500);
    let in_item = part(&[
        r#"<cellXfs><xf><extLst><ext><numFmt numFmtId="900"/></ext></extLst></xf></cellXfs>"#,
        r#"<dxfs><dxf><numFmt numFmtId="x"/><numFmt/><numFmt numFmtId="-1"/></dxf></dxfs>"#,
    ]);
    assert_eq!(read(&in_item).scan.numbers.max_id, 900);
    let unparsable = part(&[r#"<dxfs><dxf><numFmt numFmtId="x"/><numFmt/></dxf></dxfs>"#]);
    assert_eq!(read(&unparsable).scan.numbers.max_id, 0);
    let spaced = part(&[r#"<dxfs><dxf><numFmt numFmtId=" 77 "/></dxf></dxfs>"#]);
    assert_eq!(read(&spaced).scan.numbers.max_id, 77);
}

#[test]
fn a_number_format_with_no_id_or_code_is_counted_and_not_defined() {
    let xml = part(&[concat!(
        r#"<numFmts><numFmt formatCode="a"/><numFmt numFmtId="170"/><numFmt numFmtId="x" formatCode="b"/>"#,
        r#"<numFmt numFmtId="171" formatCode="c"/><numFmt numFmtId="171" formatCode="d"/></numFmts>"#
    )]);
    let edit = read(&xml);
    let numbers = &edit.scan.numbers;
    assert_eq!(numbers.read, 5);
    assert!(!numbers.defines(170));
    assert!(numbers.defines(171));
    assert_eq!(numbers.id_of("a"), None);
    // 171 is defined as c and as d: it is given to neither.
    assert_eq!(numbers.id_of("c"), None);
    assert_eq!(numbers.id_of("d"), None);
    assert_eq!(numbers.max_id, 171);
}

#[test]
fn a_number_format_code_is_read_with_its_references_resolved() {
    let xml = part(&[
        r#"<numFmts count="1"><numFmt numFmtId="170" formatCode="&quot;a&quot; &amp; b"/></numFmts>"#,
    ]);
    assert_eq!(read(&xml).scan.numbers.id_of(r#""a" & b"#), Some(170));
}

#[test]
fn more_items_in_a_list_than_the_limit_is_too_many() {
    let limits = StyleLimits {
        max_items_read: 2,
        ..StyleLimits::default()
    };
    let lists = [
        (
            "numFmts",
            r#"<numFmt numFmtId="1" formatCode="a"/>"#,
            StyleKind::NumberFormat,
        ),
        ("fonts", "<font/>", StyleKind::Font),
        ("fills", "<fill/>", StyleKind::Fill),
        ("borders", "<border/>", StyleKind::Border),
        ("cellXfs", "<xf/>", StyleKind::CellFormat),
    ];
    for (list, item, kind) in lists {
        let within = part(&[&format!("<{list}>{item}{item}</{list}>")]);
        assert!(
            StylesEdit::read(within.as_bytes(), &limits).is_ok(),
            "{list}"
        );
        let over = part(&[&format!("<{list}>{item}{item}{item}</{list}>")]);
        match StylesEdit::read(over.as_bytes(), &limits) {
            Err(StyleError::TooMany { kind: found, limit }) => {
                assert_eq!((found, limit), (kind, 2), "{list}");
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn what_an_item_holds_that_the_table_lacks_is_kept_as_written() {
    let font = concat!(
        r#"<font><b/><x:ext xmlns:x="urn:x" a='1'><!--c--><?p q?><![CDATA[<z>]]>&amp;t<y/> <w k="v">x</w></x:ext>"#,
        r#"<outline/></font>"#
    );
    let xml = part(&[
        &format!(r#"<fonts count="1">{font}</fonts>"#),
        r#"<cellXfs count="1"><xf fontId="0"/></cellXfs>"#,
    ]);
    let mut edit = read(&xml);
    let style = Style {
        font: Some(Font {
            italic: Some(true),
            ..Font::default()
        }),
        ..Style::default()
    };
    assert_eq!(edit.cell_format(0, &style).unwrap(), 1);
    let out = rewritten(&edit, &xml);
    let raw = r#"<x:ext xmlns:x="urn:x" a='1'><!--c--><?p q?><![CDATA[<z>]]>&amp;t<y/> <w k="v">x</w></x:ext>"#;
    assert!(
        out.contains(&format!("<font><b/>{raw}<i/><outline/></font></fonts>")),
        "{out}"
    );
}

#[test]
fn a_list_named_element_inside_another_element_is_not_a_list_of_the_part() {
    let xml = part(&[
        r#"<numFmts><numFmt numFmtId="170" formatCode="a"><fonts><font/></fonts></numFmt></numFmts>"#,
        r#"<cellStyleXfs><xf><fills><fill/></fills></xf></cellStyleXfs>"#,
        r#"<dxfs><dxf><fonts><font/></fonts><borders><border/></borders></dxf></dxfs>"#,
        r#"<extLst><ext><cellXfs><xf/></cellXfs></ext></extLst>"#,
    ]);
    let edit = read(&xml);
    let scan = &edit.scan;
    assert!(!scan.fonts.shape.present);
    assert!(!scan.fills.shape.present);
    assert!(!scan.borders.shape.present);
    assert!(!scan.cell_xfs.shape.present);
    assert_eq!(scan.numbers.read, 1);
}

#[test]
fn an_id_defined_twice_is_conflicting_only_when_the_codes_differ() {
    let xml = part(&[concat!(
        r#"<numFmts><numFmt numFmtId="170" formatCode="a"/><numFmt numFmtId="170" formatCode="a"/>"#,
        r#"<numFmt numFmtId="171" formatCode="b"/><numFmt numFmtId="171" formatCode="c"/>"#,
        r#"<numFmt numFmtId="171" formatCode="b"/><numFmt numFmtId="172" formatCode="b"/></numFmts>"#
    )]);
    let numbers = read(&xml).scan.numbers;
    assert_eq!(numbers.id_of("a"), Some(170));
    assert_eq!(numbers.id_of("c"), None);
    assert!(numbers.defines(171));
    // The code b is not found under 171, which is conflicting, but under 172.
    assert_eq!(numbers.id_of("b"), Some(172));
    assert_eq!(numbers.max_id, 172);
}

#[test]
fn a_cell_style_is_noted_only_when_the_cell_styles_have_an_xf() {
    let has = |children: &str| {
        let xml = part(&[&format!("<cellStyleXfs>{children}</cellStyleXfs>")]);
        read(&xml).scan.cell_style_xfs
    };
    assert!(has("<xf/>"));
    assert!(has(r#"<x:y xmlns:x="urn:x"/><xf><alignment/></xf>"#));
    assert!(!has(""));
    assert!(!has(r#"<x:xf xmlns:x="urn:x"/><other/>"#));
    assert!(!read(&part(&["<dxfs/>"])).scan.cell_style_xfs);
}

#[test]
fn many_ids_defined_with_different_codes_are_all_given_to_no_code() {
    let mut formats = String::new();
    for id in 164..3164_u32 {
        formats += &format!(r#"<numFmt numFmtId="{id}" formatCode="a{id}"/>"#);
        formats += &format!(r#"<numFmt numFmtId="{id}" formatCode="b{id}"/>"#);
    }
    formats += r#"<numFmt numFmtId="5000" formatCode="ok"/>"#;
    let edit = read(&part(&[&format!("<numFmts>{formats}</numFmts>")]));
    let numbers = &edit.scan.numbers;
    assert_eq!(numbers.read, 6001);
    assert_eq!(numbers.id_of("ok"), Some(5000));
    for id in [164_u32, 1000, 3163] {
        assert!(numbers.defines(id));
        assert_eq!(numbers.id_of(&format!("a{id}")), None);
        assert_eq!(numbers.id_of(&format!("b{id}")), None);
    }
}

#[test]
fn the_greatest_number_format_id_includes_the_attribute_of_any_element() {
    let xml = part(&[
        r#"<cellXfs><xf numFmtId="164"/><xf numFmtId="x"/><xf x:numFmtId="999" xmlns:x="urn:x"/></cellXfs>"#,
        r#"<dxfs><dxf numFmtId="250"/></dxfs>"#,
        r#"<extLst><ext><y:z xmlns:y="urn:y" numFmtId="400"/></ext></extLst>"#,
    ]);
    assert_eq!(read(&xml).scan.numbers.max_id, 400);
}

#[test]
fn a_code_keeps_its_first_id_when_a_later_id_with_that_code_is_conflicting() {
    let xml = part(&[concat!(
        r#"<numFmts><numFmt numFmtId="171" formatCode="b"/><numFmt numFmtId="172" formatCode="b"/>"#,
        r#"<numFmt numFmtId="172" formatCode="c"/></numFmts>"#
    )]);
    let numbers = read(&xml).scan.numbers;
    assert_eq!(numbers.id_of("b"), Some(171));
    assert_eq!(numbers.id_of("c"), None);
}

#[test]
fn a_list_inside_an_element_of_another_namespace_is_refused() {
    for list in ["numFmts", "fonts", "fills", "borders", "cellXfs"] {
        for inside in [
            format!("<{list}/>"),
            format!("<{list} count=\"0\"><x/></{list}>"),
        ] {
            let xml = part(&[&format!(
                r#"<mc:AlternateContent xmlns:mc="urn:mc"><mc:Choice Requires="x14"><b>{inside}</b></mc:Choice></mc:AlternateContent>"#
            )]);
            // The diagnostic names the list, not the element of another namespace.
            match refused(&xml) {
                Diagnostic::XmlSchemaInvalid { reason, name, .. } => {
                    assert_eq!(reason, SchemaReason::UnexpectedElement);
                    assert_eq!(name.as_str(), list);
                }
                other => panic!("{other:?}"),
            }
        }
    }
    // A list that is a child of the element of another namespace is refused
    // too, also after other children.
    let direct = part(&[r#"<y:z xmlns:y="urn:y"><y:a/><fonts/></y:z>"#]);
    refused_as(&direct, SchemaReason::UnexpectedElement);
    // After an element that has children of its own.
    let nested = part(&[r#"<y:z xmlns:y="urn:y"><y:a><y:b/></y:a><fonts/></y:z>"#]);
    refused_as(&nested, SchemaReason::UnexpectedElement);
}

#[test]
fn an_element_of_another_namespace_with_no_list_of_the_part_inside_is_read() {
    let xml = part(&[
        r#"<mc:AlternateContent xmlns:mc="urn:mc"><mc:Choice><x14:foo xmlns:x14="urn:x14"/><y:fonts xmlns:y="urn:y"><font/></y:fonts></mc:Choice></mc:AlternateContent>"#,
        r#"<other xmlns=""><fonts/><cellXfs/></other>"#,
        r#"<z xmlns:q="urn:q"/>"#,
        r#"<fonts count="1"><font/></fonts>"#,
    ]);
    let edit = read(&xml);
    assert_eq!(edit.scan.fonts.items.len(), 1);
    assert!(!edit.scan.cell_xfs.shape.present);
}
