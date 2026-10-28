//! What a string's value is, and how one is looked up.

use super::zip::Item;
use super::{
    NS, OFFICE_DOCUMENT, STRICT_NS, book, book_with, clean, package_of, read, rel, values,
};
use crate::{Lookup, SharedStrings};

#[test]
fn a_string_is_its_text_and_the_text_of_each_run() {
    let body = concat!(
        "<si><t>United States</t></si>",
        r#"<si><r><t xml:space="preserve">United </t></r>"#,
        r#"<r><rPr><b/><sz val="11"/><color rgb="FFFF0000"/></rPr><t>States</t></r></si>"#,
        "<si><t>a</t><r><t>b</t></r><r><t>c</t></r></si>",
    );
    assert_eq!(clean(body), ["United States", "United States", "abc"]);
}

#[test]
fn phonetic_runs_and_their_properties_are_not_part_of_the_value() {
    let body = concat!(
        r#"<si><t>課きく 毛こ</t><rPh sb="0" eb="1"><t>カ</t></rPh>"#,
        r#"<rPh sb="4" eb="5"><t>ケ</t></rPh><phoneticPr fontId="1" type="Hiragana"/></si>"#,
        r#"<si><r><t>漢</t></r><rPh sb="0" eb="1"><t>かん</t></rPh><phoneticPr fontId="1"/></si>"#,
    );
    assert_eq!(clean(body), ["課きく 毛こ", "漢"]);
}

#[test]
fn escapes_are_undone_and_what_is_not_one_is_kept() {
    let body = concat!(
        "<si><t>a_x0009_b_x000D__x000a_c</t></si>",
        "<si><t>_x005F_x0041_ is literal</t></si>",
        "<si><t>_x00G1_ _x41_ _x0041</t></si>",
        "<si><r><t>_x00</t></r><r><t>41_</t></r></si>",
    );
    assert_eq!(
        clean(body),
        [
            "a\tb\r\nc",
            "_x0041_ is literal",
            "_x00G1_ _x41_ _x0041",
            "_x0041_",
        ]
    );
}

#[test]
fn an_escape_is_read_after_references_and_cdata_are() {
    let body = concat!(
        "<si><t>_x&#48;041_ &lt;&gt;&amp;&apos;&quot;&#x3042;</t></si>",
        "<si><t><![CDATA[<b>_x0042_]]></t></si>",
    );
    assert_eq!(clean(body), ["A <>&'\"あ", "<b>B"]);
}

#[test]
fn text_after_what_was_held_back_reads_on() {
    let long = "あいう".repeat(20);
    let body = format!(
        "<si><t>a_&amp;{long}_x0041_</t></si><si><t>_x&#48;041_{long}</t></si><si><t>_&amp;b</t></si>",
    ) + "<si><t>_&amp;a_x0041_bcdefghijklmnop</t></si>";
    assert_eq!(
        clean(&body),
        [
            format!("a_&{long}A"),
            format!("A{long}"),
            String::from("_&b"),
            // Reading stops inside the text joined to what was held back,
            // and reads on from there.
            String::from("_&aAbcdefghijklmnop"),
        ]
    );
}

#[test]
fn empty_strings_and_white_space_are_kept_as_written() {
    let body = concat!(
        "<si/>",
        "<si></si>",
        "<si><t/></si>",
        "<si><t>  </t></si>",
        "<si><t xml:space=\"default\"> a  b </t></si>",
        "<si><r><t/></r></si>",
    );
    assert_eq!(clean(body), ["", "", "", "  ", " a  b ", ""]);
}

#[test]
fn line_ends_are_normalized_as_xml_has_them() {
    let body = "<si><t>a\r\nb\rc<!-- x -->\nd<![CDATA[\r\n]]></t></si>";
    assert_eq!(clean(body), ["a\nb\nc\nd\n"]);
}

#[test]
fn the_extension_list_and_white_space_between_elements_are_skipped() {
    let body = concat!(
        "\n  <si>\n    <t>a</t>\n  </si>\n  <?pi x?>",
        r#"<extLst><ext uri="{x}"><x:y xmlns:x="urn:x">text</x:y></ext></extLst>"#,
    );
    assert_eq!(clean(body), ["a"]);
}

#[test]
fn counts_are_not_checked() {
    let (mut strings, seen) = read(book_with(Item::stored(
        "xl/sharedStrings.xml",
        format!(r#"<sst xmlns="{NS}" count="x" uniqueCount="9"><si><t>a</t></si></sst>"#)
            .as_bytes(),
    )));
    assert_eq!(seen, vec![]);
    assert_eq!(values(&mut strings), ["a"]);
}

#[test]
fn a_strict_table_reads_by_its_strict_relationship() {
    let main = rel(
        "rId1",
        "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument",
        "xl/workbook.xml",
    );
    let related = rel(
        "rId9",
        "http://purl.oclc.org/ooxml/officeDocument/relationships/sharedStrings",
        "strings/s.xml",
    );
    let xml = format!(r#"<s:sst xmlns:s="{STRICT_NS}"><s:si><s:t>a</s:t></s:si></s:sst>"#);
    let part = Item::stored("xl/strings/s.xml", xml.as_bytes());
    let (mut strings, seen) = read(package_of(&main, &related, vec![part]));
    assert_eq!(seen, vec![]);
    assert_eq!(values(&mut strings), ["a"]);
}

#[test]
fn a_utf16_table_reads_as_its_utf8() {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?><sst xmlns="{NS}"><si><t>日本</t></si></sst>"#
    );
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let (mut strings, seen) = read(book_with(Item::stored("xl/sharedStrings.xml", &bytes)));
    assert_eq!(seen, vec![]);
    assert_eq!(values(&mut strings), ["日本"]);
}

#[test]
fn a_lookup_past_the_table_is_out_of_range_and_leaves_out_empty() {
    let (mut strings, _) = read(book("<si><t>a</t></si><si><t>bc</t></si>"));
    let SharedStrings::Table(table) = &strings else {
        panic!("{strings:?}");
    };
    assert_eq!(
        (table.len(), table.is_empty(), table.is_spilled()),
        (2, false, false)
    );
    let mut out = String::from("stale");
    assert_eq!(strings.get(1, &mut out).unwrap(), Lookup::Found);
    assert_eq!(out, "bc");
    for index in [2, u64::MAX] {
        assert_eq!(strings.get(index, &mut out).unwrap(), Lookup::OutOfRange);
        assert_eq!(out, "");
    }
    assert_eq!(strings.get(0, &mut out).unwrap(), Lookup::Found);
    assert_eq!(out, "a");
}

#[test]
fn an_empty_table_has_nothing_to_look_up() {
    let (mut strings, seen) = read(book(""));
    assert_eq!(seen, vec![]);
    let SharedStrings::Table(table) = &strings else {
        panic!("{strings:?}");
    };
    assert!(table.is_empty());
    let mut out = String::new();
    assert_eq!(strings.get(0, &mut out).unwrap(), Lookup::OutOfRange);
}

#[test]
fn with_no_relationship_the_table_is_absent_and_every_index_out_of_range() {
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let (mut strings, seen) = read(package_of(&main, "", vec![]));
    assert_eq!(seen, vec![]);
    assert!(matches!(strings, SharedStrings::Absent));
    let mut out = String::from("stale");
    assert_eq!(strings.get(0, &mut out).unwrap(), Lookup::OutOfRange);
    assert_eq!(out, "");
}

#[test]
fn an_unknown_table_answers_unknown() {
    let mut strings = SharedStrings::Unknown;
    let mut out = String::from("stale");
    assert_eq!(strings.get(0, &mut out).unwrap(), Lookup::Unknown);
    assert_eq!(out, "");
}
