//! Writing an item as XML: the bytes of each element, and that the XML read
//! back is the item.

use super::node;
use crate::canonical::write::write_number_format;
use crate::canonical::{Node, schema};

/// The XML of the item `xml`, written under `prefix`.
fn written(xml: &str, prefix: &str) -> String {
    let mut out = Vec::new();
    node(xml).write(prefix.as_bytes(), &mut out);
    String::from_utf8(out).unwrap()
}

/// `xml`, written under a prefix, with the prefix taken off the names that
/// the tests' reader reads, as a reader that resolves the prefix's namespace
/// does.
fn unprefixed(xml: &str) -> String {
    xml.replace("<p:", "<").replace("</p:", "</")
}

#[test]
fn a_font_is_written_with_its_children_in_the_order_excel_writes_them() {
    let font = concat!(
        r#"<font><name val="Arial"/><color rgb="ff00aa00"/><sz val="11.0"/><vertAlign val="superscript"/>"#,
        r#"<u val="double"/><strike/><i/><b/><scheme val="minor"/><charset val="1"/><family val="2"/></font>"#,
    );
    assert_eq!(
        written(font, ""),
        concat!(
            r#"<font><b/><i/><strike/><u val="double"/><vertAlign val="superscript"/><sz val="11"/>"#,
            r#"<color rgb="FF00AA00"/><name val="Arial"/><family val="2"/><charset val="1"/>"#,
            r#"<scheme val="minor"/></font>"#,
        )
    );
}

#[test]
fn a_border_is_written_with_its_empty_sides() {
    assert_eq!(
        written("<border/>", ""),
        "<border><left/><right/><top/><bottom/><diagonal/></border>"
    );
    assert_eq!(
        written(
            r#"<border diagonalDown="1"><bottom style="thin"><color auto="1"/></bottom><top style="thick"/></border>"#,
            "",
        ),
        concat!(
            r#"<border diagonalDown="1"><left/><right/><top style="thick"/>"#,
            r#"<bottom style="thin"><color auto="1"/></bottom><diagonal/></border>"#,
        )
    );
    assert_eq!(
        written(
            r#"<border><vertical style="hair"/><right style="thin"/></border>"#,
            ""
        ),
        concat!(
            r#"<border><left/><right style="thin"/><top/><bottom/><diagonal/>"#,
            r#"<vertical style="hair"/></border>"#,
        )
    );
}

#[test]
fn an_element_with_no_child_is_written_as_an_empty_element() {
    assert_eq!(
        written("<fill><patternFill/></fill>", ""),
        "<fill><patternFill/></fill>"
    );
    assert_eq!(
        written(
            r#"<xf numFmtId="164" applyNumberFormat="1" quotePrefix="1"/>"#,
            ""
        ),
        r#"<xf numFmtId="164" quotePrefix="1" applyNumberFormat="1"/>"#
    );
    assert_eq!(written("<fill/>", ""), "<fill/>");
}

#[test]
fn every_name_is_written_with_the_prefix_when_there_is_one() {
    assert_eq!(
        written("<border><left style=\"thin\"/></border>", "s"),
        r#"<s:border><s:left style="thin"/><s:right/><s:top/><s:bottom/><s:diagonal/></s:border>"#
    );
    assert_eq!(
        written(r#"<xf><alignment wrapText="1"/></xf>"#, "x"),
        r#"<x:xf><x:alignment wrapText="1"/></x:xf>"#
    );
}

#[test]
fn booleans_are_one_or_zero_numbers_decimal_doubles_shortest_and_hex_upper_case() {
    assert_eq!(
        written(
            r#"<font><sz val="11.0"/><b val="true"/><family val="+3"/></font>"#,
            ""
        ),
        r#"<font><b/><sz val="11"/><family val="3"/></font>"#
    );
    assert_eq!(
        written(r#"<font><sz val="1e1"/></font>"#, ""),
        r#"<font><sz val="10"/></font>"#
    );
    assert_eq!(
        written(r#"<font><sz val="0.5"/></font>"#, ""),
        r#"<font><sz val="0.5"/></font>"#
    );
    assert_eq!(
        written(r#"<font><color rgb="ff00aa00" tint="-0.25"/></font>"#, ""),
        r#"<font><color rgb="FF00AA00" tint="-0.25"/></font>"#
    );
    assert_eq!(
        written(r#"<xf quotePrefix="1" fontId="3" xfId="0"/>"#, ""),
        r#"<xf fontId="3" xfId="0" quotePrefix="1"/>"#
    );
}

#[test]
fn a_text_is_escaped_so_that_a_reader_gets_it_back() {
    let xml = r#"<font><name val="A&amp;B&lt;C&gt;D&quot;E&#9;F&#10;G&#13;H'I"/></font>"#;
    assert_eq!(
        written(xml, ""),
        r#"<font><name val="A&amp;B&lt;C&gt;D&quot;E&#9;F&#10;G&#13;H'I"/></font>"#
    );
    let again = written(xml, "");
    assert_eq!(node(&again), node(xml));
}

#[test]
fn what_is_not_in_the_table_is_written_as_it_was() {
    let xml = concat!(
        r#"<xf b:z="2" a:y="1&amp;"><x:extra xmlns:x="urn:x"  k = 'v'/>"#,
        r#"<alignment wrapText="1"/><extLst><ext uri="{1}"/></extLst></xf>"#,
    );
    assert_eq!(
        written(xml, ""),
        concat!(
            r#"<xf a:y="1&amp;" b:z="2"><alignment wrapText="1"/><extLst><ext uri="{1}"/></extLst>"#,
            r#"<x:extra xmlns:x="urn:x"  k = 'v'/></xf>"#,
        )
    );
}

#[test]
fn what_is_written_is_read_back_equal() {
    let items = [
        "<font/>",
        r#"<font x:a="1"><b/><i/><u val="singleAccounting"/><sz val="9.5"/><color theme="1" tint="0.5"/><name val="Meiryo"/><scheme val="major"/></font>"#,
        r#"<fill><patternFill patternType="gray0625"><fgColor rgb="FF010203"/><bgColor indexed="64"/></patternFill></fill>"#,
        r#"<fill><gradientFill degree="90"><stop position="0"/></gradientFill></fill>"#,
        r#"<border outline="0" diagonalUp="1"><left style="thin"><color auto="1"/></left><diagonal style="slantDashDot"/><horizontal style="hair"/></border>"#,
        r#"<xf numFmtId="14" fontId="1" xfId="0" applyAlignment="1"><alignment horizontal="center" vertical="top" wrapText="1" indent="1" textRotation="90"/><protection locked="0"/><extLst/></xf>"#,
        r#"<font><b val="maybe"/><color rgb="FF00AA0"/><sz val="NaN"/></font>"#,
        r#"<font><b/><b/></font>"#,
    ];
    for xml in items {
        let first = written(xml, "");
        assert_eq!(node(&first), node(xml), "{xml}");
        assert_eq!(written(&first, ""), first, "{xml}");
        let prefixed = written(xml, "p");
        assert!(prefixed.starts_with("<p:"), "{xml}");
        assert_eq!(node(&unprefixed(&prefixed)), node(xml), "{xml}");
    }
}

#[test]
fn a_number_format_is_written_with_its_id_and_code() {
    let mut out = Vec::new();
    write_number_format(b"", 164, "yyyy-mm-dd", &mut out);
    assert_eq!(out, br#"<numFmt numFmtId="164" formatCode="yyyy-mm-dd"/>"#);
    let mut out = Vec::new();
    write_number_format(b"s", 4_294_967_295, "\"a\" & <b>", &mut out);
    assert_eq!(
        out,
        br#"<s:numFmt numFmtId="4294967295" formatCode="&quot;a&quot; &amp; &lt;b&gt;"/>"#
    );
}

#[test]
fn a_node_of_the_table_is_written_without_being_read() {
    let mut item = Node::new(&schema::PROTECTION);
    item.set_attribute("locked", Some(crate::canonical::Value::Bool(false)));
    let mut out = Vec::new();
    item.write(b"", &mut out);
    assert_eq!(out, br#"<protection locked="0"/>"#);
}
