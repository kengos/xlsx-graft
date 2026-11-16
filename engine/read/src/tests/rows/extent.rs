//! What a sheet says of its extent beside its rows: the `ref` of its
//! `dimension`, the ranges of its `mergeCells`, the limit on them, and
//! whether the part was read to its end.

use super::super::NS;
use super::super::zip::Item;
use super::{SHEET, book_of, briefs, open_with, seen};
use crate::{ReadLimits, Row};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit};

/// What reading a sheet to its end found of its extent.
#[derive(Debug, PartialEq, Eq)]
struct Found {
    /// The `dimension` once the first row has come.
    early: Option<String>,
    /// The `dimension` at the end.
    dimension: Option<String>,
    /// The merged ranges kept, written.
    merges: Vec<String>,
    /// Whether the sheet says it was read to its end.
    complete: bool,
    /// What was found: each diagnostic's code and where it points.
    codes: Vec<(String, String)>,
}

/// A worksheet of two rows between `head` and `tail`.
fn sheet(head: &str, tail: &str) -> String {
    let rows = r#"<row r="1"><c><v>1</v></c></row><row r="2"><c><v>2</v></c></row>"#;
    format!(r#"<worksheet xmlns="{NS}">{head}<sheetData>{rows}</sheetData>{tail}</worksheet>"#)
}

/// The extent of the first sheet of `items`, read to its end with at most
/// `max_merged` merged ranges kept, and what was found; `push` says whether
/// the caller then calls `push_merge_limit`.
fn found_in(items: Vec<Item>, max_merged: u64, push: bool) -> Found {
    let limits = ReadLimits {
        max_merged_cells: Limit::default_of(max_merged),
        ..ReadLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let book = open_with(items, &limits, Rc::default(), &mut diagnostics).unwrap();
    let mut rows = book.rows(0).unwrap();
    let mut row = Row::new();
    assert!(!rows.complete());
    let first = rows.next_row(&mut row, &mut diagnostics).unwrap();
    let early = rows.dimension().map(str::to_owned);
    assert!(first && !rows.complete());
    while rows.next_row(&mut row, &mut diagnostics).unwrap() {}
    if push {
        rows.push_merge_limit(&mut diagnostics);
    }
    Found {
        early,
        dimension: rows.dimension().map(str::to_owned),
        merges: rows.merges().iter().map(ToString::to_string).collect(),
        complete: rows.complete(),
        codes: briefs(&seen(diagnostics)),
    }
}

/// [`found_in`] a sheet of `head` and `tail`, the limit the default.
fn found(head: &str, tail: &str) -> Found {
    found_in(book_of(&sheet(head, tail)), 262_144, true)
}

/// The ranges kept for `tail`, which must read to its end clean.
fn merges(tail: &str) -> Vec<String> {
    let found = found("", tail);
    assert_eq!((found.complete, found.codes), (true, vec![]), "{tail}");
    found.merges
}

#[test]
fn the_dimension_gives_its_ref_as_written() {
    for (head, expected) in [
        (r#"<dimension ref="A1:C3"/>"#, Some("A1:C3")),
        (r#"<dimension ref="B2"/>"#, Some("B2")),
        // Not checked to be a range, and decoded as XML 1.0 has it.
        (r#"<dimension ref="a&amp;b"/>"#, Some("a&b")),
        (r#"<dimension ref=""/>"#, Some("")),
        ("<dimension/>", None),
        (r#"<dimension spans="A1"/>"#, None),
        (r#"<dimension xmlns:x="urn:x" x:ref="A1"/>"#, None),
        ("", None),
    ] {
        let found = found(head, "");
        let expected = expected.map(str::to_owned);
        assert_eq!((&found.early, &found.dimension), (&expected, &expected));
        assert_eq!((found.complete, found.codes), (true, vec![]), "{head}");
    }
    // An element that holds something is passed over as well.
    let head = r#"<dimension ref="A1:Z9"><x ref="Q1"/></dimension>"#;
    assert_eq!(found(head, "").dimension.as_deref(), Some("A1:Z9"));
}

#[test]
fn a_range_is_kept_for_each_merge_cell_that_names_one() {
    let tail = concat!(
        r#"<mergeCells count="9">"#,
        // Corners in either order, and one cell, which is a range alone.
        r#"<mergeCell ref="A1:C1"/><mergeCell ref="B3:A2"/><mergeCell ref="D4"/>"#,
        // A `ref` that names no range, and a `mergeCell` without a `ref`.
        r#"<mergeCell ref="A1:"/><mergeCell ref="$A$1:$B$2"/><mergeCell ref="XFE1:XFE2"/>"#,
        r#"<mergeCell/><mergeCell ref=""/>"#,
        // Taken for its `ref`; what it holds is passed over.
        r#"<mergeCell ref="E5:F6"><extLst/></mergeCell>"#,
        "</mergeCells>",
    );
    assert_eq!(merges(tail), ["A1:C1", "A2:B3", "D4:D4", "E5:F6"]);
}

#[test]
fn only_a_merge_cell_of_the_sheet_inside_merge_cells_is_taken() {
    let tail = concat!(
        r#"<mergeCells xmlns:m="urn:x">"#,
        r#"<m:mergeCell ref="A1:B1"/><other ref="A2:B2"/><mergeCell ref="A3:B3"/>"#,
        r#"<wrapper><mergeCell ref="A4:B4"/></wrapper>"#,
        r#"<mergeCell ref="A5:B5"/>"#,
        "</mergeCells>",
        r#"<conditionalFormatting><mergeCell ref="A6:B6"/></conditionalFormatting>"#,
    );
    assert_eq!(merges(tail), ["A3:B3", "A5:B5"]);
}

#[test]
fn a_sheet_without_merge_cells_keeps_none() {
    let none = Vec::<String>::new();
    for tail in [
        "",
        "<mergeCells/>",
        "<mergeCells></mergeCells>",
        "<mergeCells> </mergeCells>",
    ] {
        assert_eq!(merges(tail), none, "{tail}");
    }
    // An element after an empty `mergeCells` is not one of its children.
    assert_eq!(merges(r#"<mergeCells/><phoneticPr ref="A1"/>"#), none);
}

#[test]
fn merge_cells_past_the_limit_are_counted_and_not_kept() {
    let cells = |count: u32| -> Vec<Item> {
        let cells: String = (1..=count)
            .map(|n| format!(r#"<mergeCell ref="A{n}:B{n}"/>"#))
            .collect();
        book_of(&sheet("", &format!("<mergeCells>{cells}</mergeCells>")))
    };
    // At the limit all are kept, and nothing is reported.
    let at = found_in(cells(3), 3, true);
    assert_eq!((at.merges.len(), at.codes), (3, vec![]));
    // Past it the first are kept, and the sheet reports how many there are.
    let past = found_in(cells(5), 3, true);
    assert_eq!(past.merges, ["A1:B1", "A2:B2", "A3:B3"]);
    assert_eq!(
        past.codes,
        [("merged_cells_too_many".to_owned(), SHEET.to_owned())]
    );
    assert!(past.complete);
    // Reading the rows alone pushes nothing, and keeps and stops no less.
    let quiet = found_in(cells(5), 3, false);
    assert_eq!(
        (quiet.merges, quiet.codes, quiet.complete),
        (past.merges, vec![], true)
    );
    // A `mergeCell` that names no range counts as one.
    let bad = sheet(
        "",
        r#"<mergeCells><mergeCell ref="x"/><mergeCell ref="A1"/></mergeCells>"#,
    );
    let bad = found_in(book_of(&bad), 1, true);
    assert_eq!((bad.merges, bad.codes.len()), (vec![], 1));
}

#[test]
fn the_diagnostic_of_too_many_merge_cells_has_the_count_and_the_limit() {
    let cells = r#"<mergeCell ref="A1"/><mergeCell ref="A2"/>"#;
    let items = book_of(&sheet("", &format!("<mergeCells>{cells}</mergeCells>")));
    let limits = ReadLimits {
        max_merged_cells: Limit::default_of(1),
        ..ReadLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let book = open_with(items, &limits, Rc::default(), &mut diagnostics).unwrap();
    let mut rows = book.rows(0).unwrap();
    let mut row = Row::new();
    while rows.next_row(&mut row, &mut diagnostics).unwrap() {}
    rows.push_merge_limit(&mut diagnostics);
    let expected = Diagnostic::MergedCellsTooMany {
        count: 2,
        limit: Limit::default_of(1),
    };
    let seen = seen(diagnostics);
    assert!(matches!(&seen[..], [(diagnostic, _)] if *diagnostic == expected));
}

#[test]
fn a_sheet_is_complete_only_when_read_to_its_end_with_nothing_stopping_it() {
    assert!(found("", "").complete);
    // A problem in the part stops the reading: the sheet is not complete.
    let tail = r#"<mergeCells><mergeCell ref="A1:B1"/></mergeCells><bad/>"#;
    let stopped = found("", tail);
    assert_eq!((stopped.complete, stopped.codes.len()), (false, 1));
    // The count of a part that stopped is not the sheet's: no diagnostic of
    // the limit comes, though more ranges than it came before the problem.
    let cells = r#"<mergeCell ref="A1"/><mergeCell ref="A2"/><mergeCell ref="A3"/>"#;
    let tail = format!("<mergeCells>{cells}</mergeCells><bad/>");
    let stopped = found_in(book_of(&sheet("", &tail)), 2, true);
    assert_eq!(stopped.codes.len(), 1);
    assert!(
        stopped.codes[0].0.starts_with("schema:"),
        "{:?}",
        stopped.codes
    );
    assert!(!stopped.complete);
    // A damaged entry reports itself alone, and the sheet is not complete,
    // whatever was found in its bytes, a count past the limit included.
    let cells = r#"<mergeCell ref="A1"/><mergeCell ref="A2"/>"#;
    let mut items = book_of(&sheet("", &format!("<mergeCells>{cells}</mergeCells>")));
    let at = items.len() - 2;
    items[at] = items[at].clone().damaged(|data| {
        let from = data.windows(3).position(|w| w == b">2<").unwrap();
        data[from + 1] = b'3';
    });
    let damaged = found_in(items, 1, true);
    let crc = (
        "zip_crc_mismatch".to_owned(),
        "xl/worksheets/sheet1.xml".to_owned(),
    );
    assert_eq!((damaged.complete, damaged.codes), (false, vec![crc]));
}

#[test]
fn a_dimension_is_kept_to_its_first_thousand_characters_cut_between_characters() {
    let kept = |text: String| {
        let head = format!(r#"<dimension ref="{text}"/>"#);
        found(&head, "").dimension.unwrap()
    };
    assert_eq!(kept("A".repeat(1_000)), "A".repeat(1_000));
    assert_eq!(kept("A".repeat(1_001)), "A".repeat(1_000));
    assert_eq!(kept("あ".repeat(1_500)), "あ".repeat(1_000));
    assert_eq!(kept("A1:B2".to_owned()), "A1:B2");
}

#[test]
fn the_merges_are_cut_when_more_mergecells_came_than_the_limit() {
    let cells = |count: u32| -> Vec<Item> {
        let cells: String = (1..=count)
            .map(|n| format!(r#"<mergeCell ref="A{n}:B{n}"/>"#))
            .collect();
        book_of(&sheet("", &format!("<mergeCells>{cells}</mergeCells>")))
    };
    let cut = |count: u32, bad: bool| {
        let limits = ReadLimits {
            max_merged_cells: Limit::default_of(3),
            ..ReadLimits::default()
        };
        let mut diagnostics = Diagnostics::new();
        let mut items = cells(count);
        if bad {
            let xml = sheet(
                "",
                r#"<mergeCells><mergeCell ref="x"/><mergeCell ref="y"/></mergeCells>"#,
            );
            items = book_of(&xml);
        }
        let book = open_with(items, &limits, Rc::default(), &mut diagnostics).unwrap();
        let mut rows = book.rows(0).unwrap();
        let mut row = Row::new();
        while rows.next_row(&mut row, &mut diagnostics).unwrap() {}
        rows.merges_cut()
    };
    assert!(!cut(3, false));
    assert!(cut(4, false));
    // Ranges that are no range are not cut by the limit.
    assert!(!cut(0, true));
}
