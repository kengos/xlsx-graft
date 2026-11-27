//! Tests for `RowsWriter` and `SpooledRows`, driven through their public
//! methods: the XML written for each row, what is refused and that a refused
//! row leaves the writer as it was, and the range and the count of cut
//! strings. `order` covers which error `push` returns when a row breaks two
//! rules (the order `push` documents), and the date system a date cell is
//! written in;
//! `storage` covers the deflated bytes held (spilled or not, a failing spool,
//! a corrupt stream).

mod order;
mod storage;

use crate::{Row, RowCell, RowsWriter, SpooledRows, WriteError};
use xlsx_graft_cell::{CellError, CellRef, CellValue, DateSystem};
use xlsx_graft_spool::MemorySpools;

/// A cell of column `column` holding the number `value`.
pub(super) fn num(column: u16, value: f64) -> RowCell<'static> {
    RowCell::new(column, None, CellValue::Number(value))
}

/// The writer every test starts from: in memory, the 1900 date system.
fn writer(spools: &mut MemorySpools) -> RowsWriter<'_> {
    RowsWriter::new(spools, u64::MAX, DateSystem::Base1900)
}

/// What `rows` holds, as the text `write_sheet_data` writes.
pub(super) fn render(rows: &mut SpooledRows) -> String {
    // A sink that fails past 4 MiB, so a loop that never ends fails the test.
    let mut out = crate::tests::Sink::failing_after(1 << 22);
    rows.write_sheet_data(&mut out).unwrap();
    String::from_utf8(out.bytes).unwrap()
}

/// Pushes each of `rows` (a row number and its cells) and finishes.
fn finished(rows: &[(u32, &[RowCell<'_>])]) -> SpooledRows {
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    for (number, cells) in rows {
        writer.push(&Row::new(*number, cells)).unwrap();
    }
    writer.finish().unwrap()
}

fn at(column: u16, row: u32) -> CellRef {
    CellRef::new(column, row).unwrap()
}

#[test]
fn rows_in_ascending_order_are_written_as_row_elements() {
    let mut rows = finished(&[
        (1, &[num(1, 1.5), num(2, 2.0)]),
        (3, &[RowCell::new(4, Some(7), CellValue::Integer(-3))]),
    ]);
    assert_eq!(
        render(&mut rows),
        concat!(
            r#"<row r="1"><c r="A1"><v>1.5</v></c><c r="B1"><v>2</v></c></row>"#,
            r#"<row r="3"><c r="D3" s="7"><v>-3</v></c></row>"#,
        )
    );
    assert_eq!(rows.cells(), Some((at(1, 1), at(4, 3))));
    assert_eq!(rows.truncated(), 0);
}

#[test]
fn a_row_number_not_past_the_last_one_is_refused_and_the_writer_is_unchanged() {
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    writer.push(&Row::new(5, &[num(2, 1.0)])).unwrap();
    for (number, last) in [(5, 5), (4, 5), (1, 5)] {
        let error = writer.push(&Row::new(number, &[num(9, 1.0)])).unwrap_err();
        assert!(
            matches!(error, WriteError::RowOutOfOrder { row, last: l } if row == number && l == last),
            "{error:?}"
        );
    }
    writer.push(&Row::new(6, &[num(3, 2.0)])).unwrap();
    let mut rows = writer.finish().unwrap();
    assert_eq!(
        render(&mut rows),
        concat!(
            r#"<row r="5"><c r="B5"><v>1</v></c></row>"#,
            r#"<row r="6"><c r="C6"><v>2</v></c></row>"#,
        )
    );
    assert_eq!(rows.cells(), Some((at(2, 5), at(3, 6))));
}

#[test]
fn a_row_number_outside_the_sheet_is_refused_before_the_order_is_checked() {
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    for number in [0, 1_048_577, u32::MAX] {
        let error = writer.push(&Row::new(number, &[num(1, 1.0)])).unwrap_err();
        assert!(
            matches!(error, WriteError::RowOutOfRange { row } if row == number),
            "{error:?}"
        );
    }
    writer.push(&Row::new(1_048_576, &[num(1, 1.0)])).unwrap();
    // After row 1,048,576, the rows 0 and 1,048,577 are refused as out of
    // range, not as out of order.
    for number in [0, 1_048_577] {
        let error = writer.push(&Row::new(number, &[])).unwrap_err();
        assert!(
            matches!(error, WriteError::RowOutOfRange { row } if row == number),
            "{error:?}"
        );
    }
    let mut rows = writer.finish().unwrap();
    assert_eq!(
        render(&mut rows),
        r#"<row r="1048576"><c r="A1048576"><v>1</v></c></row>"#
    );
}

#[test]
fn a_column_outside_the_sheet_is_refused_and_no_cell_of_the_row_is_written() {
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    for column in [0, 16_385, u16::MAX] {
        let cells = [num(1, 1.0), num(column, 1.0)];
        let row = Row::new(2, &cells);
        let error = writer.push(&row).unwrap_err();
        assert!(
            matches!(error, WriteError::ColumnOutOfRange { row: 2, column: c } if c == column),
            "{error:?}"
        );
    }
    // Column 0 after column 3 is out of range, not out of order.
    let error = writer
        .push(&Row::new(2, &[num(3, 1.0), num(0, 1.0)]))
        .unwrap_err();
    assert!(matches!(error, WriteError::ColumnOutOfRange { .. }));
    // Row 2 was never pushed, so it can be pushed now.
    writer.push(&Row::new(2, &[num(16_384, 1.0)])).unwrap();
    let mut rows = writer.finish().unwrap();
    assert_eq!(
        render(&mut rows),
        r#"<row r="2"><c r="XFD2"><v>1</v></c></row>"#
    );
    assert_eq!(rows.cells(), Some((at(16_384, 2), at(16_384, 2))));
}

#[test]
fn a_column_not_past_the_previous_cell_is_refused_and_the_row_is_not_written() {
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    for column in [3, 2, 1] {
        // The first two cells are in order: the third is the one refused.
        let cells = [num(1, 1.0), num(3, 1.0), num(column, 1.0)];
        let row = Row::new(4, &cells);
        let error = writer.push(&row).unwrap_err();
        assert!(
            matches!(error, WriteError::CellOutOfOrder { row: 4, column: c, last: 3 } if c == column),
            "{error:?}"
        );
    }
    writer.push(&Row::new(4, &[num(1, 5.0)])).unwrap();
    let mut rows = writer.finish().unwrap();
    assert_eq!(
        render(&mut rows),
        r#"<row r="4"><c r="A4"><v>5</v></c></row>"#
    );
}

#[test]
fn a_value_the_cell_writer_refuses_is_reported_at_its_cell_and_nothing_is_written() {
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    let long = "x".repeat(32_768);
    let cells = [
        num(1, 1.0),
        RowCell::new(2, None, CellValue::Number(f64::NAN)),
    ];
    let row = Row::new(3, &cells);
    let error = writer.push(&row).unwrap_err();
    assert!(
        matches!(&error, WriteError::CellRefused { at: a, source: CellError::NotFinite } if *a == at(2, 3)),
        "{error:?}"
    );
    assert!(std::error::Error::source(&error).is_some());
    let cells = [
        RowCell::new(1, None, CellValue::String(&long)),
        RowCell::new(2, None, CellValue::Integer(i64::MAX)),
    ];
    let row = Row::new(3, &cells);
    let error = writer.push(&row).unwrap_err();
    assert!(matches!(
        error,
        WriteError::CellRefused {
            source: CellError::InexactInteger,
            ..
        }
    ));
    // Neither refused row changed the last row number, the range or the count
    // of cut strings.
    writer.push(&Row::new(3, &[num(5, 9.0)])).unwrap();
    let mut rows = writer.finish().unwrap();
    assert_eq!(
        render(&mut rows),
        r#"<row r="3"><c r="E3"><v>9</v></c></row>"#
    );
    assert_eq!(rows.cells(), Some((at(5, 3), at(5, 3))));
    assert_eq!(rows.truncated(), 0);
}

#[test]
fn a_row_without_cells_writes_nothing_and_still_counts_for_the_order() {
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    writer.push(&Row::new(4, &[])).unwrap();
    for number in [4, 3] {
        let error = writer.push(&Row::new(number, &[num(1, 1.0)])).unwrap_err();
        assert!(matches!(error, WriteError::RowOutOfOrder { last: 4, .. }));
    }
    let mut rows = writer.finish().unwrap();
    assert_eq!(render(&mut rows), "");
    assert_eq!(rows.cells(), None);
    assert_eq!(rows.truncated(), 0);

    let mut spools = MemorySpools;
    let rows = RowsWriter::new(&mut spools, 0, DateSystem::Base1900);
    assert_eq!(rows.finish().unwrap().cells(), None);
    assert_eq!(render(&mut finished(&[])), "");
}

#[test]
fn an_empty_cell_is_written_with_its_format_and_is_in_the_range() {
    let mut rows = finished(&[(2, &[RowCell::new(2, Some(3), CellValue::Empty)])]);
    assert_eq!(render(&mut rows), r#"<row r="2"><c r="B2" s="3"/></row>"#);
    assert_eq!(rows.cells(), Some((at(2, 2), at(2, 2))));
}

#[test]
fn the_range_spans_the_cells_of_every_row() {
    let rows = finished(&[
        (2, &[num(3, 1.0), num(4, 1.0)]),
        (4, &[]),
        (5, &[num(1, 1.0)]),
        (9, &[num(6, 1.0)]),
        (12, &[]),
    ]);
    assert_eq!(rows.cells(), Some((at(1, 2), at(6, 9))));
    // A narrower row after a wider one leaves the columns as they were.
    let rows = finished(&[
        (1, &[num(2, 1.0), num(8, 1.0)]),
        (2, &[num(4, 1.0), num(5, 1.0)]),
    ]);
    assert_eq!(rows.cells(), Some((at(2, 1), at(8, 2))));
}

#[test]
fn the_first_and_last_row_and_column_of_the_sheet_are_accepted() {
    let mut rows = finished(&[
        (1, &[num(1, 1.0), num(16_384, 1.0)]),
        (1_048_576, &[num(1, 1.0), num(16_384, 1.0)]),
    ]);
    assert_eq!(rows.cells(), Some((at(1, 1), at(16_384, 1_048_576))));
    assert!(render(&mut rows).starts_with(r#"<row r="1"><c r="A1">"#));
}

#[test]
fn a_string_cut_to_32767_code_units_is_counted_once_per_cell() {
    let exact = "a".repeat(32_767);
    let long = "a".repeat(32_768);
    let mut spools = MemorySpools;
    let mut writer = writer(&mut spools);
    let text = |column, value| RowCell::new(column, None, CellValue::String(value));
    writer
        .push(&Row::new(
            1,
            &[text(1, &exact), text(2, &long), text(3, &long)],
        ))
        .unwrap();
    writer.push(&Row::new(2, &[text(1, &long)])).unwrap();
    let rows = writer.finish().unwrap();
    assert_eq!(rows.truncated(), 3);

    let rows = finished(&[(1, &[text(1, &exact)])]);
    assert_eq!(rows.truncated(), 0);
}

#[test]
fn strings_are_inline_and_a_shared_string_is_written_as_given() {
    let mut rows = finished(&[(
        1,
        &[
            RowCell::new(1, None, CellValue::String("a<b")),
            RowCell::new(2, None, CellValue::SharedString(12)),
        ],
    )]);
    assert_eq!(
        render(&mut rows),
        concat!(
            r#"<row r="1"><c r="A1" t="inlineStr"><is><t>a&lt;b</t></is></c>"#,
            r#"<c r="B1" t="s"><v>12</v></c></row>"#,
        )
    );
}

#[test]
fn write_sheet_data_gives_the_same_bytes_every_time() {
    let mut rows = finished(&[(1, &[num(1, 1.0)]), (2, &[num(1, 2.0)])]);
    let first = render(&mut rows);
    assert_eq!(render(&mut rows), first);
    assert_eq!(render(&mut rows), first);
    assert!(!first.contains("<sheetData"));
}

#[test]
fn the_error_messages_name_the_row_and_column() {
    let messages = [
        (
            WriteError::RowOutOfOrder { row: 3, last: 5 },
            "row 3 is not after row 5, the last row pushed",
        ),
        (
            WriteError::RowOutOfRange { row: 0 },
            "row 0 is outside the sheet (1 to 1048576)",
        ),
        (
            WriteError::ColumnOutOfRange { row: 2, column: 0 },
            "column 0 of row 2 is outside the sheet (1 to 16384)",
        ),
        (
            WriteError::CellOutOfOrder {
                row: 2,
                column: 1,
                last: 4,
            },
            "column 1 of row 2 is not after column 4, the column of the cell before it",
        ),
        (
            WriteError::CellRefused {
                at: at(2, 3),
                source: CellError::NotFinite,
            },
            "the cell B3 is refused: the number is not finite",
        ),
    ];
    for (error, message) in messages {
        assert_eq!(error.to_string(), message);
    }
}
