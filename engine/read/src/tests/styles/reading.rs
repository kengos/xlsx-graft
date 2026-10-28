//! What styles that read are read to: the kind of each cell format, from a
//! built-in or a defined number format, in either namespace, past what is
//! skipped.

use super::super::zip::Item;
use super::super::{NS, OFFICE_DOCUMENT, STRICT_NS, WORKBOOK_RELS, at, package_of, rel};
use super::{PART, STYLES, cells, kinds, numbers, part, read, styled, styled_with};
use crate::FormatKind::{Date, DateTime, Duration, General, Number, Text, Time, Unknown};
use crate::Styles;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};

#[test]
fn each_cell_format_takes_the_kind_of_its_number_format() {
    let body = format!(
        "{}{}",
        numbers(&[(164, "yyyy-mm-dd"), (165, "[h]:mm:ss"), (166, "0.0")]),
        cells(&[0, 14, 22, 20, 49, 164, 165, 166, 3])
    );
    assert_eq!(
        kinds(&body),
        [
            General, Date, DateTime, Time, Text, Date, Duration, Number, Number
        ]
    );
}

#[test]
fn a_number_format_neither_defined_nor_built_in_is_unknown() {
    assert_eq!(kinds(&cells(&[5, 8, 23, 41, 44, 82, 164])), [Unknown; 7]);
}

#[test]
fn a_defined_format_takes_the_place_of_a_built_in_one() {
    let body = format!(
        "{}{}",
        numbers(&[(0, "0.00"), (14, "h:mm")]),
        cells(&[0, 14])
    );
    assert_eq!(kinds(&body), [Number, Time]);
}

#[test]
fn two_definitions_of_one_id_are_unknown_unless_they_agree() {
    let formats = [(164, "yyyy"), (164, "yyyy-mm"), (165, "0"), (165, "h")];
    let body = format!("{}{}", numbers(&formats), cells(&[164, 165]));
    assert_eq!(kinds(&body), [Date, Unknown]);
}

#[test]
fn an_xf_without_a_number_format_is_general_whatever_it_applies() {
    let body = format!(
        "{}{}",
        numbers(&[(164, "0")]),
        concat!(
            r#"<cellXfs count="9"><xf/>"#,
            r#"<xf numFmtId="164" applyNumberFormat="0"/>"#,
            r#"<xf numFmtId=" +14 " fontId="1" fillId="2" borderId="3" xfId="0" quotePrefix="1" pivotButton="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1" applyProtection="1"/>"#,
            "</cellXfs>",
        )
    );
    assert_eq!(kinds(&body), [General, Number, Date]);
}

#[test]
fn a_format_code_has_its_references_and_escapes_undone() {
    let formats = [
        (164, "&quot;h&quot;0"),
        (165, "&#x22;h&#34;0"),
        (166, "_x0022_h_x0022_0"),
        (167, "yyyy_x003B_0"),
    ];
    let body = format!("{}{}", numbers(&formats), cells(&[164, 165, 166, 167]));
    assert_eq!(kinds(&body), [Number, Number, Number, Unknown]);
}

#[test]
fn what_is_skipped_is_not_read() {
    let body = concat!(
        r#"<numFmts count="0"/>"#,
        r#"<fonts><font><anything at="all"/>text</font></fonts>"#,
        "<fills/><borders/>",
        r#"<cellStyleXfs><xf numFmtId="x"/></cellStyleXfs>"#,
        r#"<cellXfs><xf numFmtId="14"><alignment wrapText="1"/><protection/><extLst><ext/></extLst></xf>"#,
        "<!-- a comment --><?pi?>\n  <xf/></cellXfs>",
        "<cellStyles/><dxfs/><tableStyles/><colors/><extLst><ext>x</ext></extLst>",
    );
    assert_eq!(kinds(body), [Date, General]);
}

#[test]
fn styles_of_the_strict_namespace_and_relationship_read() {
    let strict = "http://purl.oclc.org/ooxml/officeDocument/relationships/styles";
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let related = rel("rId1", strict, "styles.xml");
    let xml = format!(
        r#"<x:styleSheet xmlns:x="{STRICT_NS}"><x:cellXfs><x:xf numFmtId="46"/></x:cellXfs></x:styleSheet>"#
    );
    let (styles, seen) = read(package_of(&main, &related, vec![part(&xml)]));
    assert_eq!(seen, vec![]);
    assert_eq!(styles, Styles::Formats(vec![Duration]));
}

#[test]
fn what_excel_writes_reads_past_its_ignorable_namespaces() {
    let xml = format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            "\r\n",
            r#"<styleSheet xmlns="{NS}" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="x14ac x16r2 xr" "#,
            r#"xmlns:x14ac="http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac" "#,
            r#"xmlns:x16r2="http://schemas.microsoft.com/office/spreadsheetml/2015/02/main" "#,
            r#"xmlns:xr="http://schemas.microsoft.com/office/spreadsheetml/2014/revision">"#,
            r#"<numFmts count="1"><numFmt numFmtId="176" formatCode="yyyy/mm/dd" xr:uid="1"/></numFmts>"#,
            r#"<fonts count="1" x14ac:knownFonts="1"><font><sz val="11"/></font></fonts>"#,
            r#"<x16r2:newThing/>"#,
            r#"<cellXfs count="2"><xf numFmtId="0"/><xr:extra><y/></xr:extra><xf numFmtId="176" applyNumberFormat="1"/></cellXfs>"#,
            "</styleSheet>",
        ),
        NS = NS
    );
    let (styles, seen) = read(styled_with(part(&xml)));
    assert_eq!(seen, vec![]);
    assert_eq!(styles, Styles::Formats(vec![General, Date]));
}

#[test]
fn an_ignorable_namespace_is_ignorable_inside_the_element_that_names_it() {
    let mc = "http://schemas.openxmlformats.org/markup-compatibility/2006";
    let body = format!(
        concat!(
            r#"<cellXfs xmlns:mc="{mc}" xmlns:v="urn:v" v:at="1" mc:Ignorable="v">"#,
            r#"<xf numFmtId="14" v:at="1"><v:x/></xf></cellXfs>"#,
        ),
        mc = mc
    );
    assert_eq!(kinds(&body), [Date]);
}

#[test]
fn a_namespace_understood_named_ignorable_is_still_read() {
    let mc = "http://schemas.openxmlformats.org/markup-compatibility/2006";
    let body = format!(
        concat!(
            r#"<cellXfs xmlns:mc="{mc}" xmlns:s="{NS}" mc:Ignorable="s xml">"#,
            r#"<s:xf numFmtId="14"/></cellXfs>"#,
        ),
        mc = mc,
        NS = NS
    );
    assert_eq!(kinds(&body), [Date]);
    // Its attributes are no more ignorable than they were.
    let xml = body.replace("<s:xf ", r#"<s:xf s:at="1" "#);
    let (styles, seen) = read(styled(&xml));
    assert_eq!(styles, Styles::Unknown);
    let root = format!(r#"<styleSheet xmlns="{NS}">"#).len();
    let diagnostic = Diagnostic::XmlSchemaInvalid {
        offset: (root + xml.find("<s:xf").unwrap()) as u64,
        reason: SchemaReason::UnexpectedAttribute,
        name: Echo::new("s:at"),
    };
    assert_eq!(seen, vec![at(diagnostic, PART)]);
}

#[test]
fn no_styles_relationship_is_absent_and_every_cell_general() {
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let (styles, seen) = read(package_of(&main, "", vec![]));
    assert_eq!(seen, vec![]);
    assert_eq!(styles, Styles::Absent);
    assert_eq!(styles.kind(0), General);
    assert_eq!(styles.kind(u64::MAX), General);
}

#[test]
fn a_cell_format_past_the_last_or_of_unknown_styles_is_unknown() {
    let (styles, _) = read(styled(&cells(&[14])));
    assert_eq!(styles.kind(0), Date);
    assert_eq!(styles.kind(1), Unknown);
    assert_eq!(styles.kind(u64::MAX), Unknown);
    let (styles, _) = read(styled(""));
    assert_eq!(styles, Styles::Formats(vec![]));
    assert_eq!(styles.kind(0), Unknown);
    assert_eq!(Styles::Unknown.kind(0), Unknown);
}

#[test]
fn styles_related_twice_are_a_duplicate_and_unknown() {
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let related = format!(
        "{}{}",
        rel("rId1", STYLES, "styles.xml"),
        rel("rId2", STYLES, "styles2.xml")
    );
    let more = vec![
        part("<styleSheet/>"),
        Item::stored("xl/styles2.xml", b"<styleSheet/>"),
    ];
    let (styles, seen) = read(package_of(&main, &related, more));
    assert_eq!(styles, Styles::Unknown);
    let rel_type = Echo::new(STYLES);
    let diagnostic = Diagnostic::RelationshipTypeDuplicate { rel_type };
    assert_eq!(seen, vec![at(diagnostic, &format!("{WORKBOOK_RELS}#rId2"))]);
}
