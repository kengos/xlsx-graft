//! Unit tests for the arguments of a sheet request: the range, `maxRows`
//! and `maxColumns`, and the sheet asked for.

use super::{Selection, SheetRequest, SheetRequestError};
use xlsx_graft_read::{CellRange, SheetChoice};

/// The request with only `range` given.
fn ranged(range: &str) -> Result<SheetRequest, SheetRequestError> {
    SheetRequest::new(None, Some(range), None, None)
}

/// The range of `text`, which the test knows is one.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap()
}

#[test]
fn a_request_without_arguments_asks_for_the_first_sheet_from_a1_with_the_defaults() {
    let request = SheetRequest::new(None, None, None, None).unwrap();
    assert_eq!(request.sheet, Selection::First);
    assert_eq!(request.range, None);
    assert_eq!(request.max_rows, SheetRequest::DEFAULT_MAX_ROWS);
    assert_eq!(request.max_columns, SheetRequest::DEFAULT_MAX_COLUMNS);
    assert_eq!((request.max_rows, request.max_columns), (100, 30));
}

#[test]
fn the_sheet_is_asked_for_by_name_or_by_index() {
    let by_name = SheetRequest::new(Some(SheetChoice::Name("Data")), None, None, None).unwrap();
    assert_eq!(by_name.sheet, Selection::Name("Data".to_owned()));
    let by_index = SheetRequest::new(Some(SheetChoice::Index(2)), None, None, None).unwrap();
    assert_eq!(by_index.sheet, Selection::Index(2));
}

#[test]
fn a_range_is_two_corners_or_one_cell_in_either_order_and_case() {
    assert_eq!(ranged("A1:H50").unwrap().range, Some(range("A1:H50")));
    assert_eq!(ranged("a1").unwrap().range, Some(range("A1:A1")));
    let reversed = ranged("H50:A1").unwrap().range.unwrap();
    assert_eq!(reversed.to_string(), "A1:H50");
    let whole = ranged("A1:XFD1048576").unwrap().range.unwrap();
    assert_eq!(whole.to_string(), "A1:XFD1048576");
}

#[test]
fn what_is_not_a_range_of_the_sheet_is_refused_with_the_text_it_was_given() {
    for text in [
        "A0",
        "XFE1",
        "A1:",
        "",
        "$A$1",
        "A1:B2:C3",
        "A1:A1048577",
        "1:3",
        "A:C",
    ] {
        assert_eq!(
            ranged(text),
            Err(SheetRequestError::Range(text.to_owned())),
            "{text:?}"
        );
    }
    assert_eq!(
        ranged("A0").unwrap_err().to_string(),
        "range \"A0\" is not a range of cells such as A1:H50 (columns A to XFD, rows 1 to 1048576)"
    );
}

#[test]
fn the_range_in_the_error_is_written_as_an_echo_does() {
    let hostile = format!("A1\u{1b}[2J\n:{}", "B".repeat(300));
    let SheetRequestError::Range(text) = ranged(&hostile).unwrap_err() else {
        panic!("not a range error");
    };
    assert!(!text.contains('\u{1b}') && !text.contains('\n'));
    assert!(text.starts_with("A1[2J:BBB") && text.ends_with('…'));
    assert_eq!(text.chars().count(), 201);
}

#[test]
fn max_rows_is_from_one_to_the_hard_max() {
    for (given, kept) in [(1, 1), (100, 100), (10_000, 10_000)] {
        let request = SheetRequest::new(None, None, Some(given), None).unwrap();
        assert_eq!(u64::from(request.max_rows), kept);
    }
    for given in [0, 10_001, u64::from(u32::MAX) + 1, u64::MAX] {
        assert_eq!(
            SheetRequest::new(None, None, Some(given), None),
            Err(SheetRequestError::MaxRows(given))
        );
    }
    assert_eq!(
        SheetRequestError::MaxRows(0).to_string(),
        "maxRows 0 is not between 1 and 10000"
    );
}

#[test]
fn max_columns_is_from_one_to_the_hard_max() {
    for (given, kept) in [(1, 1), (30, 30), (1_000, 1_000)] {
        let request = SheetRequest::new(None, None, None, Some(given)).unwrap();
        assert_eq!(u64::from(request.max_columns), kept);
    }
    for given in [0, 1_001, u64::from(u16::MAX) + 1, u64::MAX] {
        assert_eq!(
            SheetRequest::new(None, None, None, Some(given)),
            Err(SheetRequestError::MaxColumns(given))
        );
    }
    assert_eq!(
        SheetRequestError::MaxColumns(0).to_string(),
        "maxColumns 0 is not between 1 and 1000"
    );
}

#[test]
fn the_range_is_refused_before_the_counts() {
    assert_eq!(
        SheetRequest::new(None, Some("A0"), Some(0), Some(0)),
        Err(SheetRequestError::Range("A0".to_owned()))
    );
    assert_eq!(
        SheetRequest::new(None, Some("A1"), Some(0), Some(0)),
        Err(SheetRequestError::MaxRows(0))
    );
}
