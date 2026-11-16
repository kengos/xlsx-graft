//! Unit tests for `CellEcho`: what it keeps, what it strips, where it cuts.

use super::{CellEcho, MAX_CELL_ECHO};

#[test]
fn text_with_tab_line_feed_and_carriage_return_is_kept_as_is() {
    let echo = CellEcho::new("a\tb\nc\r\nd");
    assert_eq!(echo.as_str(), "a\tb\nc\r\nd");
    assert!(!echo.truncated());
}

#[test]
fn other_control_characters_are_stripped() {
    let echo = CellEcho::new("a\u{1b}[31mb\0c\u{7f}d\u{0b}e\u{85}f");
    assert_eq!(echo.as_str(), "a[31mbcdef");
}

#[test]
fn bidirectional_formatting_and_line_breaking_characters_are_stripped() {
    let hostile = "a\u{061c}b\u{200e}c\u{202e}d\u{2066}e\u{2028}f\u{2029}g\u{e0041}h";
    assert_eq!(CellEcho::new(hostile).as_str(), "abcdefgh");
}

#[test]
fn text_of_exactly_the_cap_is_not_truncated() {
    let text = "x".repeat(MAX_CELL_ECHO);
    let echo = CellEcho::new(&text);
    assert_eq!((echo.as_str(), echo.truncated()), (text.as_str(), false));
}

#[test]
fn text_past_the_cap_is_cut_without_an_ellipsis() {
    let echo = CellEcho::new(&"x".repeat(MAX_CELL_ECHO + 1));
    assert_eq!(echo.as_str(), "x".repeat(MAX_CELL_ECHO));
    assert!(echo.truncated());
}

#[test]
fn the_cap_counts_the_characters_kept_not_the_ones_stripped() {
    // The stripped characters before and between do not use up the cap.
    let text = format!("\u{1b}{}\u{202e}{}\n", "x".repeat(MAX_CELL_ECHO - 1), "y");
    let echo = CellEcho::new(&text);
    assert_eq!(echo.as_str(), format!("{}y", "x".repeat(MAX_CELL_ECHO - 1)));
    assert!(echo.truncated());
    // Stripped characters after the cap leave nothing to cut.
    let text = format!("{}\u{1b}\u{202e}", "x".repeat(MAX_CELL_ECHO));
    assert!(!CellEcho::new(&text).truncated());
}

#[test]
fn a_multi_byte_character_is_never_cut() {
    let echo = CellEcho::new(&"あ".repeat(MAX_CELL_ECHO + 1));
    assert_eq!(echo.as_str().chars().count(), MAX_CELL_ECHO);
    assert!(echo.truncated());
}
