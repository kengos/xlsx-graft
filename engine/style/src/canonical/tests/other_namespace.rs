//! What the styles editor needs of the canonical form besides comparing it:
//! an element of another namespace keeps its place among the children,
//! removing or replacing the child before it keeps the element, an
//! attribute is read back, an empty item is made, and a list is written
//! around items.

use super::node;
use crate::canonical::write::write_list;
use crate::canonical::{Node, Value};

/// `item` written without a prefix.
fn written(item: &Node) -> String {
    let mut out = Vec::new();
    item.write(b"", &mut out);
    String::from_utf8(out).unwrap()
}

#[test]
fn an_element_of_another_namespace_is_written_after_the_child_before_it() {
    let other = r#"<x:e xmlns:x="urn:x"/>"#;
    let alignment = r#"<alignment wrapText="1"/>"#;
    let ext = r#"<extLst><ext uri="{1}"/></extLst>"#;
    // An element written before the extLst is written before it again, and
    // one written after it after it.
    let before = format!("<xf>{alignment}{other}{ext}</xf>");
    assert_eq!(written(&node(&before)), before);
    let after = format!("<xf>{alignment}{ext}{other}</xf>");
    assert_eq!(written(&node(&after)), after);
    // An element of another namespace that is the first child has the rank 0,
    // the rank of the first child the table names.
    let first = format!(r#"<xf>{other}<protection locked="0"/></xf>"#);
    assert_eq!(written(&node(&first)), first);
}

#[test]
fn an_element_of_another_namespace_is_kept_when_the_child_before_it_is_removed() {
    let mut font = node(r#"<font><b/><x:c xmlns:x="urn:x"/></font>"#);
    font.set_child("b", None);
    assert_eq!(font, node(r#"<font><x:c xmlns:x="urn:x"/></font>"#));
}

#[test]
fn an_element_of_another_namespace_is_kept_when_the_child_before_it_is_replaced() {
    let mut font = node(r#"<font><b/><x:c xmlns:x="urn:x"/></font>"#);
    let italic = node("<font><i/></font>").child("i").cloned();
    font.set_child("b", italic);
    assert_eq!(font, node(r#"<font><x:c xmlns:x="urn:x"/><i/></font>"#));
}

#[test]
fn an_attribute_the_node_holds_is_read_back() {
    let xf = node(r#"<xf numFmtId="3" fontId="x" quotePrefix="0"/>"#);
    assert_eq!(xf.attribute("numFmtId"), Some(&Value::Uint(3)));
    assert_eq!(xf.attribute("fontId"), Some(&Value::Raw("x".into())));
    assert_eq!(xf.attribute("quotePrefix"), None);
    assert_eq!(xf.attribute("fillId"), None);
    assert_eq!(xf.attribute("nothing"), None);
}

#[test]
fn an_empty_item_is_made_for_a_font_a_border_and_a_cell_format() {
    assert_eq!(Node::font(), node("<font/>"));
    assert_eq!(Node::border(), node("<border/>"));
    assert_eq!(Node::xf(), node("<xf/>"));
}

#[test]
fn a_list_is_written_around_its_items_with_the_count() {
    let mut out = Vec::new();
    write_list(b"x", "fonts", 2, b"<x:font/><x:font/>", &mut out);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        r#"<x:fonts count="2"><x:font/><x:font/></x:fonts>"#
    );
    let mut out = Vec::new();
    write_list(b"", "cellXfs", 0, b"", &mut out);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        r#"<cellXfs count="0"></cellXfs>"#
    );
}

#[test]
fn an_element_of_the_table_s_namespace_that_the_table_lacks_is_written_last() {
    let font = node("<font><unknownThing/><b/></font>");
    assert_eq!(written(&font), "<font><b/><unknownThing/></font>");
}
