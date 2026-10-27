//! Unit tests for `Echo`: what it strips, where it clips, and how it prints.

use super::{Echo, MAX_ECHO};

#[test]
fn short_text_is_kept_as_is() {
    let echo = Echo::new("/xl/workbook.xml");
    assert_eq!(echo.as_str(), "/xl/workbook.xml");
    assert_eq!(echo.to_string(), "/xl/workbook.xml");
}

#[test]
fn control_characters_are_stripped() {
    assert_eq!(Echo::new("a\u{1b}[31mb\nc\0d\u{7f}e").as_str(), "a[31mbcde");
}

#[test]
fn bidirectional_formatting_characters_are_stripped() {
    let hostile = "a\u{061c}b\u{200e}c\u{200f}d\u{202a}e\u{202e}f\u{2066}g\u{2069}h";
    assert_eq!(Echo::new(hostile).as_str(), "abcdefgh");
}

#[test]
fn separators_and_tag_characters_are_stripped() {
    let hostile = "a\u{2028}b\u{2029}c\u{e0000}d\u{e0041}e\u{e007f}f";
    assert_eq!(Echo::new(hostile).as_str(), "abcdef");
}

#[test]
fn zero_width_joiners_are_kept() {
    assert_eq!(
        Echo::new("a\u{200c}b\u{200d}c").as_str(),
        "a\u{200c}b\u{200d}c"
    );
}

#[test]
fn text_of_exactly_the_cap_is_not_marked() {
    let text = "x".repeat(MAX_ECHO);
    assert_eq!(Echo::new(&text).as_str(), text);
}

#[test]
fn text_one_past_the_cap_is_clipped_and_marked() {
    let text = "x".repeat(MAX_ECHO + 1);
    assert_eq!(
        Echo::new(&text).as_str(),
        format!("{}…", "x".repeat(MAX_ECHO))
    );
}

#[test]
fn the_cap_counts_characters_not_bytes() {
    let text = "表".repeat(MAX_ECHO + 1);
    let echo = Echo::new(&text);
    assert_eq!(echo.as_str().chars().count(), MAX_ECHO + 1);
    assert!(echo.as_str().ends_with("表…"));
}

#[test]
fn stripped_characters_do_not_count_toward_the_cap() {
    let text = format!("{}{}", "\n".repeat(MAX_ECHO), "x".repeat(MAX_ECHO));
    assert_eq!(Echo::new(&text).as_str(), "x".repeat(MAX_ECHO));
}

#[test]
fn an_echo_serializes_as_a_string() {
    assert_eq!(
        serde_json::to_string(&Echo::new("a\"b")).unwrap(),
        r#""a\"b""#
    );
}
