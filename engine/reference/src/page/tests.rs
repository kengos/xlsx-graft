//! Tests for `parse`: the front matter it reads into a [`Page`], and each
//! form of page source it refuses.

use super::*;

/// The front matter and body of a well-formed page, for the cases that
/// change one line of it.
const WELL_FORMED: &str = "---\ntitle: T\nsummary: S\ntools: [a, b_2]\n---\n\n# T\n\nBody.\n";

#[test]
fn parse_reads_the_three_keys_and_the_body() {
    let page = parse("name", WELL_FORMED).expect("a well-formed page");
    assert_eq!(page.name(), "name");
    assert_eq!(page.uri(), "xlsx-graft://reference/name");
    assert_eq!(page.title(), "T");
    assert_eq!(page.summary(), "S");
    assert_eq!(page.tools(), ["a", "b_2"]);
    assert_eq!(page.body(), "# T\n\nBody.\n");
}

#[test]
fn parse_reads_crlf_line_endings_keys_in_any_order_and_an_empty_tool_list() {
    let source =
        "---\r\ntools: []\r\nsummary: S: with a colon\r\ntitle:  T  \r\n---\r\n\r\n# T\r\n";
    let page = parse("name", source).expect("a page with CRLF line endings");
    assert_eq!(page.title(), "T");
    assert_eq!(page.summary(), "S: with a colon");
    assert!(page.tools().is_empty());
    assert_eq!(page.body(), "# T\r\n");
}

#[test]
fn parse_reads_a_page_with_no_body_after_the_closing_line() {
    let page = parse("name", "---\ntitle: T\nsummary: S\ntools: []\n---").expect("a page");
    assert_eq!(page.body(), "");
}

#[test]
fn parse_refuses_a_page_without_an_opening_or_a_closing_line() {
    for source in [
        "",
        "# T\n",
        "title: T\nsummary: S\ntools: []\n---\n",
        " ---\ntitle: T\nsummary: S\ntools: []\n---\n",
        "---\ntitle: T\nsummary: S\ntools: []\n",
        "---\ntitle: T\nsummary: S\ntools: []\n--- \n",
    ] {
        assert_eq!(parse("name", source), None, "{source:?}");
    }
}

#[test]
fn parse_refuses_a_line_that_is_not_one_of_the_three_keys() {
    for line in ["order: 1", "Title: T", "title T", ""] {
        let source = WELL_FORMED.replacen("summary: S\n", &format!("summary: S\n{line}\n"), 1);
        assert_eq!(parse("name", source.leak()), None, "{line:?}");
    }
}

#[test]
fn parse_refuses_a_repeated_missing_or_empty_key() {
    for (from, to) in [
        ("title: T\n", "title: T\ntitle: U\n"),
        ("summary: S\n", "summary: S\nsummary: S\n"),
        ("tools: [a, b_2]\n", "tools: []\ntools: []\n"),
        ("title: T\n", ""),
        ("summary: S\n", ""),
        ("tools: [a, b_2]\n", ""),
        ("title: T\n", "title:\n"),
        ("summary: S\n", "summary:   \n"),
    ] {
        let source = WELL_FORMED.replacen(from, to, 1);
        assert_eq!(parse("name", source.leak()), None, "{from:?} -> {to:?}");
    }
}

#[test]
fn parse_refuses_a_tool_list_without_brackets_or_with_a_bad_or_repeated_tool_name() {
    for tools in [
        "a", "[a", "a]", "[a,]", "[,a]", "[A]", "[a-b]", "[a b]", "[a, , b]", "[a, a]",
    ] {
        let source = WELL_FORMED.replacen("[a, b_2]", tools, 1);
        assert_eq!(parse("name", source.leak()), None, "{tools:?}");
    }
}

#[test]
fn parse_reads_a_page_at_the_size_limit_and_refuses_one_byte_over_it() {
    let padding = MAX_PAGE_BYTES - WELL_FORMED.len();
    let at_limit = format!("{WELL_FORMED}{}", "x".repeat(padding));
    assert_eq!(at_limit.len(), MAX_PAGE_BYTES);
    assert!(parse("name", at_limit.clone().leak()).is_some());
    let over = format!("{at_limit}x");
    assert_eq!(parse("name", over.leak()), None);
}
