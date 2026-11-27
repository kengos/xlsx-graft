//! The value types made by `new`: what each refuses at each boundary, and
//! the first rule it breaks.

use super::range;
use crate::sheet::{ColumnWidth, Columns, FrozenPanes, MergedCells, RowHeight, SheetError};

fn width(first: u16, last: u16) -> ColumnWidth {
    ColumnWidth::new(first, last, 10.0).unwrap()
}

#[test]
fn a_column_width_takes_columns_1_to_16384_in_order_and_a_width_to_255() {
    for (first, last, w) in [
        (1, 1, 0.0),
        (1, 16_384, 255.0),
        (16_384, 16_384, 8.43),
        (3, 4, -0.0),
    ] {
        let column = ColumnWidth::new(first, last, w).unwrap();
        assert_eq!(
            (column.first(), column.last(), column.width()),
            (first, last, w)
        );
    }
    for (first, last) in [(0, 1), (1, 16_385), (2, 1), (16_385, 16_385), (0, 0)] {
        let got = ColumnWidth::new(first, last, 10.0);
        assert_eq!(got, Err(SheetError::ColumnRange { first, last }));
    }
    // The columns are checked first.
    let got = ColumnWidth::new(0, 1, f64::NAN);
    assert_eq!(got, Err(SheetError::ColumnRange { first: 0, last: 1 }));
    for w in [
        255.000_000_1,
        256.0,
        -0.000_1,
        -1.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let got = ColumnWidth::new(1, 1, w);
        assert_eq!(got, Err(SheetError::ColumnWidth { width: w }), "{w}");
    }
    assert!(matches!(
        ColumnWidth::new(1, 1, f64::NAN),
        Err(SheetError::ColumnWidth { width }) if width.is_nan()
    ));
}

#[test]
fn a_row_height_takes_rows_1_to_1048576_and_a_height_to_409() {
    for (row, h) in [(1, 0.0), (1_048_576, 409.0), (7, 15.75), (2, -0.0)] {
        let height = RowHeight::new(row, h).unwrap();
        assert_eq!((height.row(), height.height()), (row, h));
    }
    for row in [0, 1_048_577, u32::MAX] {
        assert_eq!(RowHeight::new(row, 15.0), Err(SheetError::RowRange { row }));
    }
    assert_eq!(
        RowHeight::new(0, f64::NAN),
        Err(SheetError::RowRange { row: 0 })
    );
    for h in [
        409.000_000_1,
        410.0,
        -0.000_1,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let got = RowHeight::new(1, h);
        assert_eq!(got, Err(SheetError::RowHeight { height: h }), "{h}");
    }
    assert!(matches!(
        RowHeight::new(1, f64::NAN),
        Err(SheetError::RowHeight { height }) if height.is_nan()
    ));
}

#[test]
fn frozen_panes_take_up_to_the_last_row_and_column_less_one() {
    for (rows, columns) in [(0, 0), (1, 0), (0, 1), (1_048_575, 16_383), (3, 2)] {
        let panes = FrozenPanes::new(rows, columns).unwrap();
        assert_eq!((panes.rows(), panes.columns()), (rows, columns));
    }
    for (rows, columns) in [
        (1_048_576, 0),
        (0, 16_384),
        (1_048_576, 16_384),
        (u32::MAX, 1),
    ] {
        let got = FrozenPanes::new(rows, columns);
        assert_eq!(got, Err(SheetError::FreezeRange { rows, columns }));
    }
}

#[test]
fn merged_cells_keep_the_order_and_refuse_a_single_cell_or_an_overlap() {
    let ranges = vec![
        range("D4:E5"),
        range("A1:B2"),
        range("C1:D3"),
        range("A3:B4"),
    ];
    let merged = MergedCells::new(ranges.clone()).unwrap();
    assert_eq!(merged.ranges(), ranges);
    assert_eq!(MergedCells::new(vec![]).unwrap().ranges(), []);
    for single in ["A1", "A1:A1"] {
        let got = MergedCells::new(vec![range("B2:C3"), range(single)]);
        assert_eq!(
            got,
            Err(SheetError::MergeSingleCell {
                range: range(single)
            })
        );
    }
    // The later range is `range`, whatever the order of the corners.
    for (a, b) in [
        ("A1:B2", "B2:C3"),
        ("B2:C3", "A1:B2"),
        ("A1:D4", "B2:C3"),
        ("B2:C3", "B2:C3"),
    ] {
        let got = MergedCells::new(vec![range("F1:G2"), range(a), range(b)]);
        let error = SheetError::MergeOverlap {
            range: range(b),
            other: range(a),
        };
        assert_eq!(got, Err(error), "{a} {b}");
    }
    // A single cell is refused before an overlap is looked for.
    let got = MergedCells::new(vec![range("A1:B2"), range("A1:B2"), range("C3")]);
    assert_eq!(got, Err(SheetError::MergeSingleCell { range: range("C3") }));
}

#[test]
fn merged_cells_take_262144_ranges_and_refuse_one_more() {
    let row = |n: u32| range(&format!("A{n}:B{n}"));
    let ranges: Vec<_> = (1..=262_144).map(row).collect();
    assert_eq!(
        MergedCells::new(ranges.clone()).unwrap().ranges().len(),
        262_144
    );
    let mut more = ranges;
    more.push(row(262_145));
    assert_eq!(
        MergedCells::new(more),
        Err(SheetError::TooManyMerges {
            count: 262_145,
            limit: 262_144
        })
    );
}

#[test]
fn columns_are_in_ascending_order_and_do_not_overlap() {
    assert_eq!(Columns::new(vec![]).unwrap().widths(), []);
    let widths = vec![width(1, 2), width(3, 3), width(5, 16_384)];
    assert_eq!(Columns::new(widths.clone()).unwrap().widths(), widths);
    for (a, b) in [
        ((1, 3), (3, 4)),
        ((3, 4), (1, 2)),
        ((2, 2), (2, 2)),
        ((1, 9), (5, 6)),
    ] {
        let got = Columns::new(vec![width(a.0, a.1), width(b.0, b.1)]);
        assert_eq!(
            got,
            Err(SheetError::ColumnsOverlap {
                first: b.0,
                last: b.1
            }),
            "{a:?} {b:?}"
        );
    }
}

#[test]
fn every_error_says_what_it_refused() {
    let errors = [
        SheetError::ColumnRange { first: 0, last: 1 },
        SheetError::ColumnWidth { width: 256.0 },
        SheetError::ColumnsOverlap { first: 3, last: 4 },
        SheetError::RowRange { row: 0 },
        SheetError::RowHeight { height: 410.0 },
        SheetError::FreezeRange {
            rows: 1_048_576,
            columns: 0,
        },
        SheetError::MergeSingleCell { range: range("A1") },
        SheetError::MergeOverlap {
            range: range("B2:C3"),
            other: range("A1:B2"),
        },
        SheetError::TooManyMerges {
            count: 262_145,
            limit: 262_144,
        },
        SheetError::PasswordEmpty,
        SheetError::PasswordTooLong { length: 256 },
        SheetError::SpinCount { spin_count: 0 },
    ];
    for error in &errors {
        assert!(!error.to_string().is_empty());
    }
    let messages: Vec<_> = errors.iter().map(ToString::to_string).collect();
    assert!(messages[6].contains("A1:A1"), "{}", messages[6]);
    assert!(messages[7].contains("B2:C3") && messages[7].contains("A1:B2"));
    assert!(messages[8].contains("262145") || messages[8].contains("262,145"));
}
