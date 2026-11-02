//! References, names, DDE links and external books, with the sheet and
//! book before them.

use crate::{DdeLink, ExternalBook, Location, Reference, ReferenceKind, Span, analyze};

/// A reference as its kind, book, sheet and target.
type Seen = (ReferenceKind, Option<String>, Option<String>, String);

/// A case: a formula, and the kind, book, sheet and target of the one
/// reference in it.
type Case = (
    &'static str,
    ReferenceKind,
    Option<&'static str>,
    Option<&'static str>,
    &'static str,
);

/// The references in `formula`.
fn references(formula: &str) -> Vec<Seen> {
    analyze(formula)
        .facts
        .references
        .into_iter()
        .map(
            |Reference {
                 kind,
                 location,
                 target,
                 ..
             }| (kind, location.book, location.sheet, target),
        )
        .collect()
}

fn at(book: Option<&str>, sheet: Option<&str>) -> (Option<String>, Option<String>) {
    (book.map(str::to_owned), sheet.map(str::to_owned))
}

#[test]
fn references_carry_the_sheet_and_book_before_them() {
    use ReferenceKind::{Cell, Columns, Rows, Table};
    let cases: [Case; 15] = [
        ("$A$1", Cell, None, None, "$A$1"),
        ("Sheet1!A1", Cell, None, Some("Sheet1"), "A1"),
        ("'a b'!B$2", Cell, None, Some("a b"), "B$2"),
        ("'it''s'!C3", Cell, None, Some("it's"), "C3"),
        ("Sheet1:Sheet3!D4", Cell, None, Some("Sheet1:Sheet3"), "D4"),
        ("[1]Sheet1!E5", Cell, Some("1"), Some("Sheet1"), "E5"),
        (
            r"'C:\d\[b.xlsx]S'!F6",
            Cell,
            Some(r"C:\d\b.xlsx"),
            Some("S"),
            "F6",
        ),
        // A `]` with no `[` before it still ends a book.
        ("'x]S'!G7", Cell, Some("x"), Some("S"), "G7"),
        // A character no sheet name holds makes the whole a book.
        (r"'C:\d\b.xlsx'!H8", Cell, Some(r"C:\d\b.xlsx"), None, "H8"),
        (
            "[1]Sheet1:Sheet2!I9",
            Cell,
            Some("1"),
            Some("Sheet1:Sheet2"),
            "I9",
        ),
        ("A:$C", Columns, None, None, "A:$C"),
        ("Sheet1!1:$3", Rows, None, Some("Sheet1"), "1:$3"),
        ("$1:2", Rows, None, None, "$1:2"),
        ("T[[#This Row],[a]]", Table, None, None, "T"),
        ("[@a]", Table, None, None, ""),
    ];
    for (formula, kind, book, sheet, target) in cases {
        let (book, sheet) = at(book, sheet);
        assert_eq!(
            references(formula),
            [(kind, book, sheet, target.to_owned())],
            "{formula}"
        );
    }
}

#[test]
fn a_prefix_holds_through_a_range_and_no_further() {
    let (book, sheet) = at(None, Some("S"));
    assert_eq!(
        references("S!A1:B2+C3"),
        [
            (
                ReferenceKind::Cell,
                book.clone(),
                sheet.clone(),
                "A1".to_owned()
            ),
            (ReferenceKind::Cell, book, sheet, "B2".to_owned()),
            (ReferenceKind::Cell, None, None, "C3".to_owned()),
        ]
    );
    // Only the cell right after the prefix hands it on.
    let sheet = Some("S".to_owned());
    assert_eq!(
        references("S!A1:A2:A3"),
        [
            (ReferenceKind::Cell, None, sheet.clone(), "A1".to_owned()),
            (ReferenceKind::Cell, None, sheet, "A2".to_owned()),
            (ReferenceKind::Cell, None, None, "A3".to_owned()),
        ]
    );
    // A prefix before something that is no reference is dropped.
    assert_eq!(
        references("S!#REF!+A1"),
        [(ReferenceKind::Cell, None, None, "A1".to_owned())]
    );
}

#[test]
fn names_carry_the_sheet_and_book_before_them() {
    let read = analyze("[1]!Rate+Sheet1!Tax+Total");
    let names: Vec<_> = read
        .facts
        .names
        .iter()
        .map(|name| (name.name.as_str(), name.location.clone()))
        .collect();
    let location = |book: Option<&str>, sheet: Option<&str>| {
        let (book, sheet) = at(book, sheet);
        Location { book, sheet }
    };
    assert_eq!(
        names,
        [
            ("RATE", location(Some("1"), None)),
            ("TAX", location(None, Some("Sheet1"))),
            ("TOTAL", location(None, None)),
        ]
    );
    assert_eq!(
        read.facts.external_books,
        [ExternalBook {
            book: "1".to_owned(),
            span: Span { start: 0, end: 3 },
        }]
    );
}

#[test]
fn a_dde_link_has_its_app_topic_and_item() {
    let read = analyze("cmd|' /C calc'!A0+1");
    assert_eq!(
        read.facts.dde_links,
        [DdeLink {
            app: "cmd".to_owned(),
            topic: " /C calc".to_owned(),
            item: Some("A0".to_owned()),
            span: Span { start: 0, end: 17 },
        }]
    );
    let read = analyze("'my app'|topic!'it''em'");
    let link = read.facts.dde_links.first().cloned();
    assert_eq!(
        link.map(|l| (l.app, l.topic, l.item)),
        Some((
            "my app".to_owned(),
            "topic".to_owned(),
            Some("it'em".to_owned())
        ))
    );
}

#[test]
fn a_pipe_alone_is_still_a_dde_link() {
    let links = |formula: &str| -> Vec<(String, String, Option<String>)> {
        analyze(formula)
            .facts
            .dde_links
            .into_iter()
            .map(|l| (l.app, l.topic, l.item))
            .collect()
    };
    assert_eq!(links("(1)|2"), [(String::new(), "2".to_owned(), None)]);
    assert_eq!(links("x|"), [("x".to_owned(), String::new(), None)]);
    assert_eq!(links("x|y!"), [("x".to_owned(), "y".to_owned(), None)]);
    let read = analyze("|");
    assert_eq!(
        read.facts.dde_links.first().map(|l| l.span),
        Some(Span { start: 0, end: 1 })
    );
}
