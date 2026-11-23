//! Writing the part again: where the added items go, the `count`, a list the
//! part lacks, empty lists, prefixes, the part left as it was when nothing is
//! added, and the failures of the original and of the output.

use super::{NS, base, part, read, rewritten};
use crate::edit::{StyleError, StyleLimits, StylesEdit};
use crate::{Color, Fill, Font, PatternType, Style};
use std::io::{self, BufRead, Read, Write};
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};

/// A style that adds a number format, a font, a fill, a border and a cell
/// format.
fn everything() -> Style {
    Style {
        number_format: Some("0.0000".into()),
        font: Some(Font {
            bold: Some(true),
            ..Font::default()
        }),
        fill: Some(Fill::Pattern {
            pattern: PatternType::Solid,
            foreground: Some(Color::Rgb([0xFF, 1, 2, 3])),
            background: None,
        }),
        border: crate::Border {
            left: Some(crate::BorderSide {
                style: crate::BorderStyle::Thin,
                color: None,
            }),
            ..crate::Border::default()
        }
        .into(),
        ..Style::default()
    }
}

/// The part `xml` with `everything` added to the cell format 0.
fn with_everything(xml: &str) -> String {
    let mut edit = read(xml);
    edit.cell_format(0, &everything()).unwrap();
    rewritten(&edit, xml)
}

const NUMBER: &str = r#"<numFmt numFmtId="164" formatCode="0.0000"/>"#;
const FONT: &str = "<font><b/></font>";
const RESERVED: &str = concat!(
    r#"<fill><patternFill patternType="none"/></fill>"#,
    r#"<fill><patternFill patternType="gray125"/></fill>"#
);
const FILL: &str =
    r#"<fill><patternFill patternType="solid"><fgColor rgb="FF010203"/></patternFill></fill>"#;
const BORDER: &str = r#"<border><left style="thin"/><right/><top/><bottom/><diagonal/></border>"#;

/// The cell format `everything` adds, with the ids of its font, fill and
/// border, and `tail` (such as `xfId="0"` for a part with cell styles)
/// before the `apply` attributes.
fn xf(font: u32, fill: u32, border: u32, tail: &str) -> String {
    format!(
        r#"<xf numFmtId="164" fontId="{font}" fillId="{fill}" borderId="{border}"{tail} applyNumberFormat="1" applyFont="1" applyFill="1" applyBorder="1"/>"#
    )
}

#[test]
fn items_are_added_before_the_end_tag_of_their_list_and_the_count_is_written() {
    let xml = part(&[
        r#"<numFmts count="0"></numFmts>"#,
        r#"<fonts count="1"><font/></fonts>"#,
        r#"<fills count="2"><fill/><fill/></fills>"#,
        r#"<borders count="1"><border/></borders>"#,
        r#"<cellXfs count="1"><xf/></cellXfs>"#,
        "<dxfs/>",
    ]);
    let expected = part(&[
        &format!(r#"<numFmts count="1">{NUMBER}</numFmts>"#),
        &format!(r#"<fonts count="2"><font/>{FONT}</fonts>"#),
        &format!(r#"<fills count="3"><fill/><fill/>{FILL}</fills>"#),
        &format!(r#"<borders count="2"><border/>{BORDER}</borders>"#),
        &format!(r#"<cellXfs count="2"><xf/>{}</cellXfs>"#, xf(1, 2, 1, "")),
        "<dxfs/>",
    ]);
    assert_eq!(with_everything(&xml), expected);
}

#[test]
fn a_list_without_a_count_is_not_given_one_and_other_attributes_stay() {
    let xml = part(&[
        "<fonts x14ac:knownFonts='1' xmlns:x14ac=\"urn:x\"><font/></fonts>",
        r#"<cellXfs count='7' z="1"><xf/></cellXfs>"#,
    ]);
    let out = with_everything(&xml);
    let fonts = format!("<fonts x14ac:knownFonts='1' xmlns:x14ac=\"urn:x\"><font/>{FONT}</fonts>");
    assert!(out.contains(&fonts), "{out}");
    let xfs = format!(
        r#"<cellXfs count='2' z="1"><xf/>{}</cellXfs>"#,
        xf(1, 2, 0, "")
    );
    assert!(out.contains(&xfs), "{out}");
}

#[test]
fn only_the_value_of_the_count_is_written_again() {
    let cases = [
        ("count='7'", "count='2'"),
        ("  count = \"7\"  ", "  count = \"2\"  "),
        (
            "a='x=\"y' count=\"7\"\tb=\"c\"",
            "a='x=\"y' count=\"2\"\tb=\"c\"",
        ),
        (
            "xmlns:x=\"urn:x\" x:count=\"9\" count='7' ",
            "xmlns:x=\"urn:x\" x:count=\"9\" count='2' ",
        ),
        ("count=\"\"", "count=\"2\""),
    ];
    for (before, after) in cases {
        let xml =
            format!("<styleSheet xmlns=\"{NS}\"><fonts {before}><font/></fonts></styleSheet>");
        let out = with_everything(&xml);
        assert!(
            out.contains(&format!("<fonts {after}><font/>{FONT}</fonts>")),
            "{before}: {out}"
        );
    }
}

#[test]
fn a_start_tag_without_a_count_is_as_it_was() {
    use crate::edit::rewrite::counted;
    use xlsx_graft_xml::events::BytesStart;
    for content in [
        "fonts",
        "fonts a='1' b = \"2\"",
        "fonts a=",
        "fonts a='1",
        "fonts x",
    ] {
        let start = BytesStart::from_content(content, 5);
        let again = counted(&start, 3);
        let kept: &str = &again;
        assert_eq!(kept, content);
    }
}

#[test]
fn a_list_the_part_lacks_is_written_where_the_schema_puts_it() {
    let xml = part(&[
        r#"<fonts count="1"><font/></fonts>"#,
        r#"<cellStyleXfs count="1"><xf/></cellStyleXfs>"#,
        r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0"/></cellStyles>"#,
        "<dxfs/>",
        "<extLst/>",
    ]);
    let expected = part(&[
        &format!(r#"<numFmts count="1">{NUMBER}</numFmts>"#),
        &format!(r#"<fonts count="2"><font/>{FONT}</fonts>"#),
        &format!(r#"<fills count="3">{RESERVED}{FILL}</fills>"#),
        &format!(r#"<borders count="1">{BORDER}</borders>"#),
        r#"<cellStyleXfs count="1"><xf/></cellStyleXfs>"#,
        &format!(
            r#"<cellXfs count="1">{}</cellXfs>"#,
            xf(1, 2, 0, r#" xfId="0""#)
        ),
        r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0"/></cellStyles>"#,
        "<dxfs/>",
        "<extLst/>",
    ]);
    assert_eq!(with_everything(&xml), expected);
}

#[test]
fn every_list_goes_before_the_first_element_that_follows_it() {
    let lists = ["numFmts", "fonts", "fills", "borders", "cellXfs"];
    for name in [
        "cellStyleXfs",
        "cellStyles",
        "dxfs",
        "tableStyles",
        "colors",
        "extLst",
    ] {
        let xml = part(&[&format!("<{name}/>")]);
        let out = with_everything(&xml);
        let mut found: Vec<(usize, &str)> = lists
            .iter()
            .map(|list| (out.find(&format!("<{list} ")).unwrap(), *list))
            .collect();
        found.push((out.find(&format!("<{name}/>")).unwrap(), name));
        found.sort();
        let order: Vec<&str> = found.into_iter().map(|(_, label)| label).collect();
        // cellXfs comes after cellStyleXfs and before every other element.
        let expected = if name == "cellStyleXfs" {
            vec!["numFmts", "fonts", "fills", "borders", name, "cellXfs"]
        } else {
            vec!["numFmts", "fonts", "fills", "borders", "cellXfs", name]
        };
        assert_eq!(order, expected, "{out}");
        assert!(out.ends_with("</styleSheet>"), "{out}");
    }
}

#[test]
fn a_list_goes_before_the_end_of_the_style_sheet_when_nothing_follows() {
    let xml = part(&["<x:other xmlns:x=\"urn:x\"/>"]);
    let mut edit = read(&xml);
    edit.cell_format(0, &Style::number_format("0.0000"))
        .unwrap();
    let expected = part(&[
        "<x:other xmlns:x=\"urn:x\"/>",
        &format!(r#"<numFmts count="1">{NUMBER}</numFmts>"#),
        r#"<cellXfs count="1"><xf numFmtId="164" applyNumberFormat="1"/></cellXfs>"#,
    ]);
    assert_eq!(rewritten(&edit, &xml), expected);
}

#[test]
fn an_empty_style_sheet_and_empty_lists_are_expanded() {
    let empty = format!(r#"<styleSheet xmlns="{NS}"/>"#);
    let out = with_everything(&empty);
    let start = format!(r#"<styleSheet xmlns="{NS}"><numFmts count="1">"#);
    assert!(out.starts_with(&start), "{out}");
    assert!(out.ends_with("</cellXfs></styleSheet>"), "{out}");
    let xml = part(&["<numFmts/>", "<fonts />", r#"<cellXfs count="0"/>"#]);
    let out = with_everything(&xml);
    assert!(
        out.contains(&format!("<numFmts>{NUMBER}</numFmts>")),
        "{out}"
    );
    assert!(out.contains(&format!("<fonts >{FONT}</fonts>")), "{out}");
    let xfs = format!(r#"<cellXfs count="1">{}</cellXfs>"#, xf(0, 2, 0, ""));
    assert!(out.contains(&xfs), "{out}");
}

#[test]
fn an_empty_style_sheet_that_gets_nothing_is_written_as_it_was() {
    for xml in [
        format!(r#"<styleSheet xmlns="{NS}"/>"#),
        format!(r#"<styleSheet xmlns="{NS}" />"#),
    ] {
        let edit = read(&xml);
        assert_eq!(rewritten(&edit, &xml), xml);
    }
}

#[test]
fn a_prefixed_part_is_written_with_the_prefix_of_each_list() {
    let xml = format!(
        r#"<x:styleSheet xmlns:x="{NS}"><x:fonts count="1"><x:font/></x:fonts><y:cellXfs xmlns:y="{NS}" count="1"><y:xf/></y:cellXfs></x:styleSheet>"#
    );
    let out = with_everything(&xml);
    assert!(
        out.contains("<x:font/><x:font><x:b/></x:font></x:fonts>"),
        "{out}"
    );
    let xf = r#"<y:xf numFmtId="164" fontId="1" fillId="2" borderId="0" applyNumberFormat="1" applyFont="1" applyFill="1" applyBorder="1"/></y:cellXfs>"#;
    assert!(out.contains(xf), "{out}");
    // A list the part lacks has the prefix of the style sheet.
    let fills = concat!(
        r#"<x:fills count="3"><x:fill><x:patternFill patternType="none"/></x:fill>"#,
        r#"<x:fill><x:patternFill patternType="gray125"/></x:fill>"#,
        r#"<x:fill><x:patternFill patternType="solid"><x:fgColor rgb="FF010203"/></x:patternFill></x:fill></x:fills>"#,
    );
    assert!(out.contains(fills), "{out}");
    let numbers =
        r#"<x:numFmts count="1"><x:numFmt numFmtId="164" formatCode="0.0000"/></x:numFmts>"#;
    assert!(out.contains(numbers), "{out}");
    assert!(out.ends_with("</y:cellXfs></x:styleSheet>"), "{out}");
}

#[test]
fn an_element_in_no_namespace_beside_the_lists_is_written_as_it_was() {
    let xml = part(&[
        r#"<other xmlns=""><fonts/></other>"#,
        r#"<cellXfs count="1"><xf/></cellXfs>"#,
    ]);
    let out = with_everything(&xml);
    // The element in no namespace is not the list `fonts`, so the list
    // `fonts` is added.
    assert!(
        out.contains(r#"<other xmlns=""><fonts/></other><numFmts count="1">"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<fonts count="1"><font><b/></font></fonts>"#),
        "{out}"
    );
}

#[test]
fn an_element_of_another_namespace_named_as_a_list_does_not_change_where_the_lists_are_written() {
    let xml = part(&[r#"<x:dxfs xmlns:x="urn:x"/>"#]);
    let mut edit = read(&xml);
    edit.cell_format(0, &Style::number_format("0.0000"))
        .unwrap();
    let out = rewritten(&edit, &xml);
    assert!(
        out.starts_with(&format!(
            r#"<styleSheet xmlns="{NS}"><x:dxfs xmlns:x="urn:x"/><numFmts"#
        )),
        "{out}"
    );
}

#[test]
fn an_original_that_lacks_a_list_the_part_had_is_refused_by_the_name_of_the_list() {
    let xml = base();
    let mut edit = read(&xml);
    edit.cell_format(0, &everything()).unwrap();
    let other = format!(r#"<styleSheet xmlns="{NS}"><fonts count="0"/></styleSheet>"#);
    let mut out = Vec::new();
    match edit.rewrite(&mut other.as_bytes(), &mut out) {
        Err(StyleError::Invalid(Diagnostic::XmlSchemaInvalid { reason, name, .. })) => {
            assert_eq!(reason, SchemaReason::MissingElement);
            assert_eq!(name.as_str(), "numFmts");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_original_with_a_list_the_part_lacked_is_refused_by_the_name_of_the_list() {
    let xml = part(&[r#"<cellXfs count="1"><xf/></cellXfs>"#]);
    let mut edit = read(&xml);
    edit.cell_format(0, &everything()).unwrap();
    let other = part(&[
        r#"<fonts count="0"/>"#,
        r#"<cellXfs count="1"><xf/></cellXfs>"#,
    ]);
    let mut out = Vec::new();
    match edit.rewrite(&mut other.as_bytes(), &mut out) {
        Err(StyleError::Invalid(Diagnostic::XmlSchemaInvalid { reason, name, .. })) => {
            assert_eq!(reason, SchemaReason::UnexpectedElement);
            assert_eq!(name.as_str(), "fonts");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_strict_part_is_written_in_the_same_way() {
    let xml = r#"<styleSheet xmlns="http://purl.oclc.org/ooxml/spreadsheetml/main"><cellXfs count="1"><xf/></cellXfs></styleSheet>"#;
    let out = with_everything(xml);
    let xfs = format!(r#"<cellXfs count="2"><xf/>{}</cellXfs>"#, xf(0, 2, 0, ""));
    assert!(out.contains(&xfs), "{out}");
    let start = r#"<styleSheet xmlns="http://purl.oclc.org/ooxml/spreadsheetml/main"><numFmts"#;
    assert!(out.starts_with(start), "{out}");
}

#[test]
fn what_nothing_was_added_to_is_written_byte_for_byte() {
    let spaced = format!(
        "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n<!-- c -->\r\n<styleSheet xmlns='{NS}' xmlns:x=\"urn:x\">\r\n  <fonts count='1'  x:k=\"1\">\r\n    <font   />\r\n  </fonts>\r\n  <x:a><![CDATA[ & ]]>&amp;</x:a>\r\n</styleSheet>\r\n"
    );
    for xml in [base(), spaced] {
        assert_eq!(rewritten(&read(&xml), &xml), xml);
    }
    // A call that adds nothing leaves the part alone too.
    let xml = base();
    let mut edit = read(&xml);
    assert_eq!(edit.cell_format(0, &Style::default()).unwrap(), 0);
    assert_eq!(rewritten(&edit, &xml), xml);
}

#[test]
fn a_part_in_utf_16_is_written_in_utf_8() {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?><styleSheet xmlns="{NS}"><cellXfs count="1"><xf/></cellXfs></styleSheet>"#
    );
    let mut bytes = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let mut edit = StylesEdit::read(&bytes, &StyleLimits::default()).unwrap();
    edit.cell_format(0, &Style::number_format("é")).unwrap();
    let mut out = Vec::new();
    edit.rewrite(&mut bytes.as_slice(), &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert_eq!(
        out,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><styleSheet xmlns="{NS}"><numFmts count="1"><numFmt numFmtId="164" formatCode="é"/></numFmts><cellXfs count="2"><xf/><xf numFmtId="164" applyNumberFormat="1"/></cellXfs></styleSheet>"#
        )
    );
}

#[test]
fn an_original_that_is_not_well_formed_is_refused() {
    let edit = read(&base());
    let mut out = Vec::new();
    match edit.rewrite(&mut "<styleSheet".as_bytes(), &mut out) {
        Err(StyleError::Invalid(Diagnostic::XmlMalformed { .. })) => {}
        other => panic!("{other:?}"),
    }
}

/// A reader that fails.
struct Broken;

impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("reading"))
    }
}

impl BufRead for Broken {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        Err(io::Error::other("reading"))
    }

    fn consume(&mut self, _: usize) {}
}

/// A writer that fails.
struct Full;

impl Write for Full {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("writing"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_failure_to_read_the_original_or_to_write_the_output_is_an_io_error() {
    let xml = base();
    let edit = read(&xml);
    match edit.rewrite(&mut Broken, &mut Vec::new()) {
        Err(StyleError::Io(error)) => assert_eq!(error.to_string(), "reading"),
        other => panic!("{other:?}"),
    }
    match edit.rewrite(&mut xml.as_bytes(), &mut Full) {
        Err(StyleError::Io(error)) => assert_eq!(error.to_string(), "writing"),
        other => panic!("{other:?}"),
    }
}
