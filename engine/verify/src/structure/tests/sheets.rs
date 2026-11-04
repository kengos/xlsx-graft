//! The sheets: a name Excel does not take, and a `sheetId` two sheets
//! share.

use super::{book, check, sheet, workbook};
use xlsx_graft_diagnostics::{Diagnostic, Echo, SheetNameReason};

/// `sheet_name_invalid` for `name`, for `reason`.
fn invalid(name: &str, reason: SheetNameReason) -> Diagnostic {
    Diagnostic::SheetNameInvalid {
        name: Echo::new(name),
        reason,
    }
}

/// What checking the sound workbook with its sheet Extra renamed `name`
/// reports, as the diagnostics alone.
fn renamed(name: &str) -> Vec<Diagnostic> {
    let mut book = book();
    book.sheets[2].name = name.to_owned();
    check(&book)
        .into_iter()
        .map(|(diagnostic, _)| diagnostic)
        .collect()
}

#[test]
fn a_name_of_more_than_31_utf16_units_is_refused() {
    // Excel for Mac 16.114.3 shows its repair dialog for 32 characters and
    // cuts the name to 31; it opens 31 of `a` or of `あ` as they are.
    assert_eq!(renamed(&"a".repeat(31)), []);
    assert_eq!(renamed(&"あ".repeat(31)), []);
    let long = "a".repeat(32);
    assert_eq!(
        check(&{
            let mut book = book();
            book.sheets[2].name = long.clone();
            book
        }),
        [(invalid(&long, SheetNameReason::TooLong), workbook())]
    );
    // A character past the BMP counts as two UTF-16 units.
    let wide = format!("{}a", "😀".repeat(15));
    assert_eq!(renamed(&wide), []);
    let wider = "😀".repeat(16);
    assert_eq!(renamed(&wider), [invalid(&wider, SheetNameReason::TooLong)]);
}

#[test]
fn an_empty_name_is_refused() {
    // Excel shows its repair dialog and names the sheet 回復済み_Sheet1.
    assert_eq!(renamed(""), [invalid("", SheetNameReason::Empty)]);
}

#[test]
fn a_name_holding_a_character_a_quoted_sheet_name_may_not_is_refused() {
    // Excel shows its repair dialog for each of the seven.
    for name in ["x[y", "x]y", "x:y", "x*y", "x?y", "x/y", "x\\y"] {
        assert_eq!(
            renamed(name),
            [invalid(name, SheetNameReason::Character)],
            "{name}"
        );
    }
}

#[test]
fn a_name_starting_or_ending_with_an_apostrophe_is_refused() {
    // Excel shows its repair dialog for the first two, and opens the third
    // as it is.
    for name in ["'abc", "abc'"] {
        assert_eq!(
            renamed(name),
            [invalid(name, SheetNameReason::Apostrophe)],
            "{name}"
        );
    }
    assert_eq!(renamed("a'bc"), []);
}

#[test]
fn history_is_taken_as_excel_opens_it() {
    // Excel's own screens refuse it, but Excel opens a file that has it.
    assert_eq!(renamed("History"), []);
    assert_eq!(renamed("history"), []);
}

#[test]
fn a_name_broken_more_ways_than_one_is_reported_once_for_each() {
    let name = format!("'{}/", "a".repeat(31));
    assert_eq!(
        renamed(&name),
        [
            invalid(&name, SheetNameReason::TooLong),
            invalid(&name, SheetNameReason::Character),
            invalid(&name, SheetNameReason::Apostrophe),
        ]
    );
}

#[test]
fn a_shared_sheet_id_is_reported_at_every_later_sheet_naming_the_first() {
    // Excel shows its repair dialog and gives the later sheet another.
    let mut book = book();
    book.sheets[1].sheet_id = 1;
    book.sheets.push(sheet("Last", 1));
    let duplicate = |name: &str| Diagnostic::SheetIdDuplicate {
        name: Echo::new(name),
        other: Echo::new("データ"),
        sheet_id: 1,
    };
    assert_eq!(
        check(&book),
        [
            (duplicate("集計"), workbook()),
            (duplicate("Last"), workbook()),
        ]
    );
}

#[test]
fn a_name_another_sheet_has_is_left_to_the_workbook_reader() {
    // `sheet_name_duplicate`, which the workbook reader reports.
    let mut book = book();
    book.sheets[2].name = "データ".to_owned();
    assert_eq!(check(&book), []);
}
