//! Tests for `CellRange` and for `CellRef`'s construction and display.

use crate::{CellRange, CellRef, ValueError};

fn cell(text: &str) -> CellRef {
    CellRef::parse(text).expect("a cell of the sheet")
}

fn range(text: &str) -> CellRange {
    CellRange::parse(text).expect("a range of the sheet")
}

#[test]
fn a_cell_is_made_only_inside_the_sheet() {
    assert_eq!(CellRef::new(1, 1), Some(cell("A1")));
    assert_eq!(CellRef::new(16_384, 1_048_576), Some(cell("XFD1048576")));
    assert_eq!(CellRef::new(0, 1), None);
    assert_eq!(CellRef::new(16_385, 1), None);
    assert_eq!(CellRef::new(1, 0), None);
    assert_eq!(CellRef::new(1, 1_048_577), None);
}

#[test]
fn a_cell_is_written_as_its_column_letters_and_row() {
    let shown = |column, row| CellRef::new(column, row).map(|cell| cell.to_string());
    assert_eq!(shown(1, 1).as_deref(), Some("A1"));
    assert_eq!(shown(26, 7).as_deref(), Some("Z7"));
    assert_eq!(shown(27, 10).as_deref(), Some("AA10"));
    assert_eq!(shown(52, 1).as_deref(), Some("AZ1"));
    assert_eq!(shown(703, 2).as_deref(), Some("AAA2"));
    assert_eq!(shown(16_384, 1_048_576).as_deref(), Some("XFD1048576"));
}

#[test]
fn a_range_is_read_from_one_cell_or_two_in_either_order() {
    assert_eq!(range("A1:B2").first(), cell("A1"));
    assert_eq!(range("A1:B2").last(), cell("B2"));
    assert_eq!(range("B2:A1"), range("A1:B2"));
    assert_eq!(range("A2:B1"), range("A1:B2"));
    assert_eq!(range("c3"), CellRange::new(cell("C3"), cell("C3")));
}

#[test]
fn a_text_that_is_not_a_range_is_refused() {
    for text in [
        "", "A", "1", "A1:", ":A1", "A1:B2:C3", "$A$1", "A1:$B$2", "A1 B2",
    ] {
        assert_eq!(CellRange::parse(text), Err(ValueError::Form), "{text:?}");
    }
    for text in ["A0", "XFE1", "A1:XFE1", "A1:A1048577"] {
        assert_eq!(CellRange::parse(text), Err(ValueError::Range), "{text:?}");
    }
}

#[test]
fn a_range_is_written_with_both_corners() {
    assert_eq!(range("B2:A1").to_string(), "A1:B2");
    assert_eq!(range("C3").to_string(), "C3:C3");
}

#[test]
fn ranges_overlap_where_they_share_a_cell() {
    let a = range("A1:C3");
    assert_eq!(a.intersect(range("B2:D4")), Some(range("B2:C3")));
    assert_eq!(a.intersect(range("C3")), Some(range("C3")));
    assert_eq!(a.intersect(range("A1:C3")), Some(a));
    assert!(a.overlaps(range("C3:D4")));
    // Apart in columns, apart in rows, and apart in both.
    assert_eq!(a.intersect(range("D1:E3")), None);
    assert_eq!(a.intersect(range("A4:C5")), None);
    assert!(!a.overlaps(range("D4:E5")));
}
