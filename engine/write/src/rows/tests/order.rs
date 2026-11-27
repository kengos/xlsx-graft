//! Which error `push` returns when a row breaks two rules (the order `push`
//! documents), and the date system a date cell is written in.

use super::{num, render};
use crate::{Row, RowCell, RowsWriter, WriteError};
use xlsx_graft_cell::{CellValue, Date, DateSystem};
use xlsx_graft_spool::MemorySpools;

#[test]
fn a_date_is_written_as_a_serial_of_the_date_system_the_writer_was_made_with() {
    let date = Date::from_iso("1904-01-02").unwrap();
    let mut out = Vec::new();
    for system in [DateSystem::Base1904, DateSystem::Base1900] {
        let mut spools = MemorySpools;
        let mut writer = RowsWriter::new(&mut spools, u64::MAX, system);
        let cells = [RowCell::new(1, None, CellValue::Date(date))];
        writer.push(&Row::new(1, &cells)).unwrap();
        out.push(render(&mut writer.finish().unwrap()));
    }
    assert_eq!(out[0], r#"<row r="1"><c r="A1"><v>1</v></c></row>"#);
    assert_eq!(out[1], r#"<row r="1"><c r="A1"><v>1463</v></c></row>"#);
}

#[test]
fn a_row_out_of_order_is_reported_before_the_column_of_its_cells_is_checked() {
    let mut spools = MemorySpools;
    let mut writer = RowsWriter::new(&mut spools, u64::MAX, DateSystem::Base1900);
    writer.push(&Row::new(5, &[num(1, 1.0)])).unwrap();
    let error = writer.push(&Row::new(5, &[num(0, 1.0)])).unwrap_err();
    assert!(
        matches!(error, WriteError::RowOutOfOrder { row: 5, last: 5 }),
        "{error:?}"
    );
}

#[test]
fn a_cell_out_of_order_is_reported_before_its_value_is_checked() {
    let mut spools = MemorySpools;
    let mut writer = RowsWriter::new(&mut spools, u64::MAX, DateSystem::Base1900);
    let cells = [num(3, 1.0), num(2, f64::NAN)];
    let error = writer.push(&Row::new(1, &cells)).unwrap_err();
    assert!(
        matches!(
            error,
            WriteError::CellOutOfOrder {
                row: 1,
                column: 2,
                last: 3
            }
        ),
        "{error:?}"
    );
}

#[test]
fn an_empty_row_not_past_the_last_row_is_refused_and_the_writer_is_unchanged() {
    let mut spools = MemorySpools;
    let mut writer = RowsWriter::new(&mut spools, u64::MAX, DateSystem::Base1900);
    writer.push(&Row::new(4, &[num(1, 1.0)])).unwrap();
    for number in [4, 3] {
        let error = writer.push(&Row::new(number, &[])).unwrap_err();
        assert!(
            matches!(error, WriteError::RowOutOfOrder { row, last: 4 } if row == number),
            "{error:?}"
        );
    }
    writer.push(&Row::new(5, &[num(2, 2.0)])).unwrap();
    assert_eq!(
        render(&mut writer.finish().unwrap()),
        concat!(
            r#"<row r="4"><c r="A4"><v>1</v></c></row>"#,
            r#"<row r="5"><c r="B5"><v>2</v></c></row>"#,
        )
    );
}
