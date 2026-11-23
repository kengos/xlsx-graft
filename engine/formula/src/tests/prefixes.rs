//! The sheet prefixes a formula states: where each is, and the book, sheet
//! or two sheets it names; and the function prefixes of a word.

use crate::{analyze, function_prefix};

/// A prefix as its text, book, first sheet and last sheet.
type Seen = (String, Option<String>, String, Option<String>);

/// A case: a formula, and the text, book, first and last sheet of the one
/// prefix in it.
type Case = (
    &'static str,
    &'static str,
    Option<&'static str>,
    &'static str,
    Option<&'static str>,
);

/// The prefixes in `formula`.
fn prefixes(formula: &str) -> Vec<Seen> {
    let read = analyze(formula);
    (read.facts.sheet_prefixes.iter())
        .map(|prefix| {
            (
                read.slice(prefix.span).to_owned(),
                prefix.book.clone(),
                prefix.first.clone(),
                prefix.last.clone(),
            )
        })
        .collect()
}

#[test]
fn a_prefix_that_names_a_sheet_is_read_with_its_span_book_and_sheets() {
    let cases: [Case; 13] = [
        ("Sheet1!A1", "Sheet1!", None, "Sheet1", None),
        ("'A b'!A1", "'A b'!", None, "A b", None),
        ("'it''s'!A1", "'it''s'!", None, "it's", None),
        ("S1:S3!A1", "S1:S3!", None, "S1", Some("S3")),
        ("'S 1:S 3'!A1", "'S 1:S 3'!", None, "S 1", Some("S 3")),
        ("[1]S!A1", "[1]S!", Some("1"), "S", None),
        ("1+[1]S!A1", "[1]S!", Some("1"), "S", None),
        ("[1]S1:S3!A1", "[1]S1:S3!", Some("1"), "S1", Some("S3")),
        ("[1]'a b'!A1", "[1]'a b'!", Some("1"), "a b", None),
        (
            r"'C:\d\[b.xlsx]S'!A1",
            r"'C:\d\[b.xlsx]S'!",
            Some(r"C:\d\b.xlsx"),
            "S",
            None,
        ),
        ("'[b.xlsx]S'!A1", "'[b.xlsx]S'!", Some("b.xlsx"), "S", None),
        (
            "[1]'[b.xlsx]S'!A1",
            "'[b.xlsx]S'!",
            Some("b.xlsx"),
            "S",
            None,
        ),
        (
            "'[b.xlsx]S1:S3'!A1",
            "'[b.xlsx]S1:S3'!",
            Some("b.xlsx"),
            "S1",
            Some("S3"),
        ),
    ];
    for (formula, text, book, first, last) in cases {
        let seen = (
            text.to_owned(),
            book.map(str::to_owned),
            first.to_owned(),
            last.map(str::to_owned),
        );
        assert_eq!(prefixes(formula), [seen], "{formula}");
    }
}

#[test]
fn a_quoted_name_is_split_at_its_first_colon() {
    let seen = (
        "'a:b:c'!".to_owned(),
        None,
        "a".to_owned(),
        Some("b:c".to_owned()),
    );
    assert_eq!(prefixes("'a:b:c'!A1"), [seen]);
}

#[test]
fn a_prefix_that_names_no_sheet_is_not_read() {
    for formula in [
        "[1]!x",
        r"'C:\d\b.xlsx'!x",
        "'[b.xlsx]'!x",
        r"'C:\d\[b.xlsx]'!x",
        "a|b!c",
        "'a b'",
        "A1",
        "a:b",
    ] {
        assert_eq!(prefixes(formula), [], "{formula}");
    }
}

#[test]
fn a_space_before_the_bang_is_read_as_the_walker_reads_it() {
    let seen = ("Sheet1 !".to_owned(), None, "Sheet1".to_owned(), None);
    assert_eq!(prefixes("Sheet1 !A1"), [seen]);
}

#[test]
fn each_prefix_of_a_formula_is_read_and_a_range_has_one() {
    let seen = |text: &str, first: &str| (text.to_owned(), None, first.to_owned(), None);
    assert_eq!(
        prefixes("Sheet1!A1+'B c'!B2:B3"),
        [seen("Sheet1!", "Sheet1"), seen("'B c'!", "B c")]
    );
    assert_eq!(prefixes("Sheet1!A1:B2"), [seen("Sheet1!", "Sheet1")]);
}

#[test]
fn a_book_alone_before_a_sheet_does_not_start_that_sheets_prefix() {
    let seen = |first: &str| {
        (
            format!("{first}!"),
            Some("1".to_owned()),
            first.to_owned(),
            None,
        )
    };
    assert_eq!(prefixes("[1]!S!A1"), [seen("S")]);
    let plain = ("S!".to_owned(), None, "S".to_owned(), None);
    assert_eq!(prefixes("[1]!x+S!A1"), [plain]);
}

#[test]
fn an_empty_quoted_name_is_a_prefix_of_an_empty_sheet() {
    let seen = ("''!".to_owned(), None, String::new(), None);
    assert_eq!(prefixes("''!A1"), [seen]);
}

#[test]
fn a_book_prefix_does_not_start_the_next_prefix() {
    let book = (
        "[1]S!".to_owned(),
        Some("1".to_owned()),
        "S".to_owned(),
        None,
    );
    let plain = ("T!".to_owned(), None, "T".to_owned(), None);
    assert_eq!(prefixes("[1]S!A1+T!B1"), [book, plain]);
}

#[test]
fn a_function_prefix_is_what_a_word_starts_with_as_written() {
    let cases = [
        ("_xlfn.SUM", "_xlfn."),
        ("_xlws.SORT", "_xlws."),
        ("_xludf.F", "_xludf."),
        ("_xleta.SUM", "_xleta."),
        ("_xlpm.x", "_xlpm."),
        ("_XLFN.SUM", "_XLFN."),
        ("_xLpM.x", "_xLpM."),
        ("_xlfn._xlws.FILTER", "_xlfn._xlws."),
        ("SUM", ""),
        ("", ""),
        ("_xlfnx", ""),
        ("xlfn.a", ""),
        ("_xlpm", ""),
    ];
    for (word, prefix) in cases {
        assert_eq!(function_prefix(word), prefix, "{word}");
    }
}
