//! Tests for `truncate`, and for `write_t`: the cut at 32,767 UTF-16 code
//! units, `xml:space="preserve"`, and that both escapes are applied.

use super::{truncate, write_t};
use crate::encode::Written;

/// `text` written by `write_t`.
fn t(text: &str) -> String {
    let mut out = Vec::new();
    write_t(&mut out, text).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn text_up_to_32767_units_is_kept_and_the_rest_is_cut() {
    let kept = "a".repeat(32_767);
    assert_eq!(truncate(&kept), (kept.as_str(), Written::Whole));
    let long = "a".repeat(32_768);
    assert_eq!(truncate(&long), (kept.as_str(), Written::Truncated));
    assert_eq!(truncate(""), ("", Written::Whole));
}

#[test]
fn units_are_counted_in_utf16_and_a_surrogate_pair_is_not_split() {
    // あ is one unit and 😀 two: 32,766 units, then a pair that would end
    // at 32,768.
    let text = format!("{}😀b", "あ".repeat(32_766));
    let (cut, written) = truncate(&text);
    assert_eq!(
        (cut.encode_utf16().count(), written),
        (32_766, Written::Truncated)
    );
    // 32,765 units and a pair end at 32,767 exactly.
    let text = format!("{}😀b", "あ".repeat(32_765));
    let (cut, written) = truncate(&text);
    assert_eq!(
        (cut.encode_utf16().count(), written),
        (32_767, Written::Truncated)
    );
    let fits = format!("{}😀", "あ".repeat(32_765));
    assert_eq!(truncate(&fits), (fits.as_str(), Written::Whole));
}

#[test]
fn space_is_preserved_when_the_text_starts_or_ends_with_white_space() {
    for text in [" a", "a ", "\ta", "a\t", "\na", "a\n", "\ra", "a\r"] {
        assert!(
            t(text).starts_with(r#"<t xml:space="preserve">"#),
            "{text:?}"
        );
    }
    for text in ["a b", "a\tb", "", "a", "\u{a0}a"] {
        assert!(t(text).starts_with("<t>"), "{text:?}");
    }
}

#[test]
fn the_text_is_escaped_as_an_xstring_and_as_xml() {
    assert_eq!(t("a&b<c>d"), "<t>a&amp;b&lt;c&gt;d</t>");
    // U+0001 is written as an ST_Xstring escape, and the carriage return
    // as an XML reference.
    assert_eq!(
        t("a\u{1}\r\n"),
        "<t xml:space=\"preserve\">a_x0001_&#13;\n</t>"
    );
    // The `_` of `_x0041_` is written `_x005F_`, so that it is not read
    // as the escape of `A`.
    assert_eq!(t("_x0041_"), "<t>_x005F_x0041_</t>");
}
