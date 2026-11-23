//! When two items of the styles part are equal as canonical forms: whatever
//! their order or spelling, they have the same effect; and when they are not.

use super::node;

/// Asserts that the items `a` and `b` are equal.
fn same(a: &str, b: &str) {
    assert_eq!(node(a), node(b), "{a} and {b}");
}

/// Asserts that the items `a` and `b` are not equal.
fn differ(a: &str, b: &str) {
    assert_ne!(node(a), node(b), "{a} and {b}");
}

#[test]
fn the_order_of_children_and_of_attributes_does_not_matter() {
    same(
        r#"<font><b/><i/><sz val="11"/><name val="Arial"/></font>"#,
        r#"<font><name val="Arial"/><sz val="11"/><i/><b/></font>"#,
    );
    same(
        r#"<xf numFmtId="1" fontId="2" applyFont="1"/>"#,
        r#"<xf applyFont="1" fontId="2" numFmtId="1"/>"#,
    );
    differ(r#"<font><b/></font>"#, r#"<font><i/></font>"#);
    differ(r#"<xf numFmtId="1"/>"#, r#"<xf numFmtId="2"/>"#);
}

#[test]
fn a_child_written_twice_stays_twice() {
    differ("<font><b/><b/></font>", "<font><b/></font>");
    same(
        r#"<font><b/><u/><b/></font>"#,
        r#"<font><u/><b/><b/></font>"#,
    );
}

#[test]
fn a_boolean_property_is_true_unless_it_says_false() {
    same("<font><b/></font>", r#"<font><b val="1"/></font>"#);
    same("<font><b/></font>", r#"<font><b val="true"/></font>"#);
    same("<font><b/></font>", r#"<font><b val=" true "/></font>"#);
    same(r#"<font><b val="0"/></font>"#, "<font/>");
    same(r#"<font><i val="false"/><b val="0"/></font>"#, "<font/>");
    differ("<font><b/></font>", "<font/>");
}

#[test]
fn a_value_that_has_no_effect_is_the_same_as_no_element() {
    same("<font><u/></font>", r#"<font><u val="single"/></font>"#);
    differ("<font><u/></font>", r#"<font><u val="double"/></font>"#);
    same(r#"<font><u val="none"/></font>"#, "<font/>");
    same(r#"<font><vertAlign val="baseline"/></font>"#, "<font/>");
    differ(r#"<font><vertAlign val="subscript"/></font>"#, "<font/>");
    same(r#"<font><scheme val="none"/></font>"#, "<font/>");
    differ(r#"<font><scheme val="minor"/></font>"#, "<font/>");
}

#[test]
fn numbers_are_compared_by_value() {
    let rgb = |digits: &str| format!(r#"<font><color rgb="{digits}"/></font>"#);
    same(&rgb("ff00aa00"), &rgb("FF00AA00"));
    differ(&rgb("FF00AA00"), &rgb("FF00AA01"));
    same(
        r#"<font><color theme="1" tint="0"/></font>"#,
        r#"<font><color theme="1"/></font>"#,
    );
    same(
        r#"<font><color theme="1" tint="-0.0"/></font>"#,
        r#"<font><color theme="1"/></font>"#,
    );
    differ(
        r#"<font><color theme="1" tint="0.5"/></font>"#,
        r#"<font><color theme="1"/></font>"#,
    );
    same(
        r#"<font><sz val="11"/></font>"#,
        r#"<font><sz val="11.0"/></font>"#,
    );
    differ(
        r#"<font><sz val="11"/></font>"#,
        r#"<font><sz val="12"/></font>"#,
    );
    same(
        r#"<font><family val="2"/></font>"#,
        r#"<font><family val=" 2"/></font>"#,
    );
    differ(
        r#"<font><family val="2"/></font>"#,
        r#"<font><family val="-2"/></font>"#,
    );
}

#[test]
fn a_text_value_is_compared_after_its_character_references_are_resolved_white_space_included() {
    differ(
        r#"<font><name val="Arial"/></font>"#,
        r#"<font><name val=" Arial"/></font>"#,
    );
    same(
        r#"<font><name val="A&amp;B"/></font>"#,
        r#"<font><name val="A&#38;B"/></font>"#,
    );
}

#[test]
fn a_border_without_lines_says_the_same_however_it_is_written() {
    let empty = "<border/>";
    same(
        empty,
        "<border><left/><right/><top/><bottom/><diagonal/></border>",
    );
    same(empty, r#"<border><left style="none"/></border>"#);
    differ(
        empty,
        r#"<border><left style="none"><color/></left></border>"#,
    );
    differ(empty, r#"<border><left style="thin"/></border>"#);
    differ(
        empty,
        r#"<border><left><color indexed="64"/></left></border>"#,
    );
    differ(r#"<border outline="0"/>"#, empty);
    same(r#"<border outline="1"/>"#, empty);
}

#[test]
fn an_empty_alignment_or_protection_is_the_same_as_none() {
    same("<xf><alignment/></xf>", "<xf/>");
    same("<xf><protection/></xf>", "<xf/>");
    same(r#"<xf><alignment vertical="bottom"/></xf>"#, "<xf/>");
    differ(r#"<xf><alignment vertical="top"/></xf>"#, "<xf/>");
    differ(r#"<xf><alignment horizontal="general"/></xf>"#, "<xf/>");
    differ(r#"<xf><alignment wrapText="0"/></xf>"#, "<xf/>");
    differ(r#"<xf><protection locked="1"/></xf>"#, "<xf/>");
    same(r#"<xf quotePrefix="0"/>"#, "<xf/>");
    differ(r#"<xf applyFont="0"/>"#, "<xf/>");
}

#[test]
fn a_value_of_the_wrong_type_is_raw_and_equal_only_to_the_same_text() {
    let b = |value: &str| format!(r#"<font><b val="{value}"/></font>"#);
    same(&b("maybe"), &b("maybe"));
    differ(&b("maybe"), &b("MAYBE"));
    differ(&b("maybe"), "<font><b/></font>");
    differ(&b("maybe"), "<font/>");
    let rgb = |value: &str| format!(r#"<font><color rgb="{value}"/></font>"#);
    differ(&rgb("FF00AA0"), &rgb("00FF00AA0"));
    same(&rgb("FF00AA0"), &rgb("FF00AA0"));
    differ(&rgb("FFFFFFFFF"), &rgb("FFFFFFFF"));
    differ(&rgb("FF00AA0G"), &rgb("FF00AA00"));
    let pattern = |value: &str| format!(r#"<fill><patternFill patternType="{value}"/></fill>"#);
    same(&pattern("zigzag"), &pattern("zigzag"));
    differ(&pattern("zigzag"), &pattern("solid"));
    differ(&pattern("zigzag"), "<fill><patternFill/></fill>");
    let size = |value: &str| format!(r#"<font><sz val="{value}"/></font>"#);
    differ(&size("NaN"), &size("INF"));
    differ(&size("INF"), &size("inf"));
    differ(&size("abc"), &size("11"));
    let uint = |value: &str| format!(r#"<xf fontId="{value}"/>"#);
    differ(&uint("-1"), &uint("1"));
    differ(&uint("4294967296"), &uint("0"));
    same(&uint("x"), &uint("x"));
}

#[test]
fn what_the_table_does_not_name_is_equal_only_to_the_same_bytes() {
    let font = |attribute: &str| format!(r#"<font {attribute}><b/></font>"#);
    same(&font(r#"x:dyDescent="1""#), &font(r#"x:dyDescent="1""#));
    differ(&font(r#"x:dyDescent="1""#), &font(r#"x:dyDescent="2""#));
    differ(&font(r#"x:dyDescent="1""#), &font(r#"y:dyDescent="1""#));
    differ(&font(r#"x:dyDescent="1""#), "<font><b/></font>");
    same(
        r#"<font a="1" b="2"><b/></font>"#,
        r#"<font b="2" a="1"><b/></font>"#,
    );
    let fill = |extra: &str| format!(r#"<fill><patternFill patternType="solid"/>{extra}</fill>"#);
    let gradient = r#"<gradientFill degree="90"><stop position="0"/></gradientFill>"#;
    same(&fill(gradient), &fill(gradient));
    differ(&fill(gradient), &fill(""));
    differ(&fill(gradient), &fill(r#"<gradientFill degree="45"/>"#));
    let xf = |ext: &str| format!(r#"<xf>{ext}<alignment wrapText="1"/></xf>"#);
    let ext = r#"<extLst><ext uri="{1}"/></extLst>"#;
    same(
        &xf(ext),
        r#"<xf><alignment wrapText="1"/><extLst><ext uri="{1}"/></extLst></xf>"#,
    );
    let other = r#"<x:extra xmlns:x="urn:x"/>"#;
    same(&xf(other), &xf(other));
    differ(&xf(other), &xf(""));
    same(
        r#"<font><x:a xmlns:x="urn:x"/><b/><x:c xmlns:x="urn:x"/></font>"#,
        r#"<font><x:c xmlns:x="urn:x"/><x:a xmlns:x="urn:x"/><b/></font>"#,
    );
}

#[test]
fn an_element_with_other_attributes_or_children_is_not_absent() {
    differ(r#"<font><b val="0" x:y="1"/></font>"#, "<font/>");
    differ(r#"<font><u val="none" x:y="1"/></font>"#, "<font/>");
    differ(r#"<border><left x:y="1"/></border>"#, "<border/>");
    differ(
        r#"<border><left style="none"><x:c/></left></border>"#,
        "<border/>",
    );
    differ(r#"<xf><alignment x:y="1"/></xf>"#, "<xf/>");
    differ(
        r#"<font><vertAlign val="baseline" x:y="1"/></font>"#,
        "<font/>",
    );
}

#[test]
fn a_table_element_under_a_parent_that_does_not_list_it_stays_raw() {
    differ("<font><alignment/></font>", "<font/>");
    differ(
        r#"<font><b/><left style="none"/></font>"#,
        "<font><b/></font>",
    );
    differ("<font><font><b/></font></font>", "<font/>");
    differ("<font><font><b/></font></font>", "<font><b/></font>");
    differ("<fill><font/></fill>", "<fill/>");
    differ("<xf><border/></xf>", "<xf/>");
    same("<font><alignment/></font>", "<font><alignment/></font>");
    differ(
        r#"<font><alignment x="1"/></font>"#,
        "<font><alignment/></font>",
    );
    // The same name under a parent that lists it is a node of the table.
    same("<xf><alignment/></xf>", "<xf/>");
}

#[test]
fn only_an_item_of_the_styles_part_starts_a_node() {
    use crate::canonical::Node;
    for name in ["font", "fill", "border", "xf"] {
        assert!(Node::root(name.as_bytes()).is_some(), "{name}");
    }
    for name in [
        "b",
        "color",
        "patternFill",
        "left",
        "alignment",
        "unknown",
        "",
    ] {
        assert!(Node::root(name.as_bytes()).is_none(), "{name}");
    }
    let font = Node::root(b"font").unwrap();
    assert!(font.child_element(b"b").is_some());
    assert!(font.child_element(b"alignment").is_none());
    assert!(font.child_element(b"gradientFill").is_none());
    let fill = Node::root(b"fill").unwrap();
    assert!(fill.child_element(b"gradientFill").is_none());
    assert!(fill.child_element(b"patternFill").is_some());
}

#[test]
fn a_color_with_no_attribute_is_not_absent() {
    differ("<font><color/></font>", "<font/>");
    same("<font><color/></font>", "<font><color/></font>");
    differ(r#"<font><color auto="0"/></font>"#, "<font><color/></font>");
}

#[test]
fn equal_items_are_one_key_of_a_set_and_ordered_as_equal() {
    use std::cmp::Ordering;
    use std::collections::HashSet;
    let spellings = [
        r#"<font><b/><sz val="11"/></font>"#,
        r#"<font><sz val="11.0"/><b val="1"/></font>"#,
        r#"<font><sz val="11"/><i val="0"/><b/></font>"#,
    ];
    let items: Vec<_> = spellings.into_iter().map(node).collect();
    let set: HashSet<_> = items.iter().cloned().collect();
    assert_eq!(set.len(), 1);
    assert_eq!(items[0].partial_cmp(&items[1]), Some(Ordering::Equal));
    let other = node(r#"<font><sz val="12"/><b/></font>"#);
    assert_ne!(items[0].cmp(&other), Ordering::Equal);
    assert_eq!(items[0].cmp(&other), other.cmp(&items[0]).reverse());
    assert_eq!(items[0].partial_cmp(&other), Some(items[0].cmp(&other)));
}

#[test]
fn elements_of_the_table_are_equal_and_ordered_by_name() {
    use crate::canonical::schema::{B, I};
    use std::cmp::Ordering;
    assert_eq!(B, B);
    assert_ne!(B, I);
    assert_eq!(B.partial_cmp(&I), Some(Ordering::Less));
    assert_eq!(I.partial_cmp(&B), Some(Ordering::Greater));
    assert_eq!(B.partial_cmp(&B), Some(Ordering::Equal));
}

#[test]
fn an_element_is_hashed_by_its_name() {
    use crate::canonical::schema::{B, I};
    use std::hash::{DefaultHasher, Hash, Hasher};
    let hash = |element: &crate::canonical::schema::Element| {
        let mut hasher = DefaultHasher::new();
        element.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(hash(&B), hash(&B));
    assert_ne!(hash(&B), hash(&I));
}
