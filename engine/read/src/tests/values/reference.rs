//! A cell's `r`: its column and row, the last column and row and one
//! past each, letters in either case, and what is no cell's reference.

use crate::{CellRef, ValueError};

#[test]
fn columns_and_rows() {
    let cases = [
        ("A1", 1, 1),
        ("Z9", 26, 9),
        ("AA10", 27, 10),
        ("AZ1", 52, 1),
        ("BA1", 53, 1),
        ("ZZ1", 702, 1),
        ("AAA1", 703, 1),
        ("XFD1048576", 16_384, 1_048_576),
        ("xfd1", 16_384, 1),
        ("aB12", 28, 12),
        ("A01", 1, 1),
    ];
    for (text, column, row) in cases {
        let cell = CellRef::parse(text);
        assert_eq!(
            cell.map(|cell| (cell.column(), cell.row())),
            Ok((column, row)),
            "{text}"
        );
    }
}

#[test]
fn past_the_last_column_or_row() {
    for text in [
        "XFE1",
        "ZZZ1",
        "AAAA1",
        "ZZZZZZZZZZZZZZZZZZZZ1",
        "A1048577",
        "A0",
        "A00",
        "A99999999999999999999",
    ] {
        assert_eq!(CellRef::parse(text), Err(ValueError::Range), "{text}");
    }
}

#[test]
fn what_is_no_cell_reference() {
    for text in [
        "", "A", "1", "$A$1", "A$1", "$A1", "A1:B2", "1A", "A1A", " A1", "A1 ", "Å1", "A-1", "A+1",
    ] {
        assert_eq!(CellRef::parse(text), Err(ValueError::Form), "{text:?}");
    }
}
