//! The tables: two of one sheet that overlap, a name another table's or a
//! defined name's, and an `id` two tables share. Each is reported at the
//! later table's part.

use super::{book, check, name, table, table_at, workbook};
use xlsx_graft_diagnostics::{Diagnostic, Echo, Location};

/// `table_range_overlap` of `name` with `other`.
fn overlap(name: &str, other: &str) -> Diagnostic {
    Diagnostic::TableRangeOverlap {
        name: Echo::new(name),
        other: Echo::new(other),
    }
}

/// What checking the sound workbook with a second table, `T_Extra` of id
/// 2 on the sheet at `sheet` covering `range`, reports.
fn second(sheet: Option<u32>, range: &str) -> Vec<(Diagnostic, Location)> {
    let mut book = book();
    book.tables.push(table(2, "T_Extra", sheet, range));
    check(&book)
}

#[test]
fn a_table_overlapping_an_earlier_one_of_its_sheet_is_refused() {
    // Excel for Mac 16.114.3 shows its repair dialog for データ!A2:B3 over
    // T_Data's A1:S2, and drops the later table.
    let at = table_at("/xl/tables/table2.xml");
    assert_eq!(
        second(Some(0), "A2:B3"),
        [(overlap("T_Extra", "T_Data"), at.clone())]
    );
    assert_eq!(
        second(Some(0), "S2"),
        [(overlap("T_Extra", "T_Data"), at.clone())]
    );
    assert_eq!(
        second(Some(0), "B3:A2"),
        [(overlap("T_Extra", "T_Data"), at)]
    );
}

#[test]
fn tables_that_only_touch_or_are_on_other_sheets_do_not_overlap() {
    // Below T_Data's last row, right of its last column, at its corner,
    // and on another sheet.
    for (sheet, range) in [
        (Some(0), "A3:B4"),
        (Some(0), "T1:U2"),
        (Some(0), "T3:U4"),
        (Some(2), "A1:B2"),
    ] {
        assert_eq!(second(sheet, range), [], "{range}");
    }
}

#[test]
fn a_table_with_no_sheet_or_a_ref_not_read_as_cells_overlaps_nothing() {
    for (sheet, range) in [
        (None, "A1:B2"),
        (Some(0), "A0:B2"),
        (Some(0), "A1:XFE2"),
        (Some(0), "$A$1:$B$2"),
        (Some(0), ""),
    ] {
        assert_eq!(second(sheet, range), [], "{range}");
    }
}

#[test]
fn the_later_table_of_two_that_overlap_is_reported_whatever_rows_come_first() {
    // T_Low comes before T_Data in the package but starts lower down, and
    // T_Late overlaps both: it names the first.
    let mut book = book();
    book.tables.insert(0, table(3, "T_Low", Some(0), "B2:C9"));
    book.tables.push(table(2, "T_Late", Some(0), "C2:C2"));
    assert_eq!(
        check(&book),
        [
            (
                overlap("T_Data", "T_Low"),
                table_at("/xl/tables/table1.xml")
            ),
            (
                overlap("T_Late", "T_Low"),
                table_at("/xl/tables/table2.xml")
            ),
        ]
    );
}

#[test]
fn a_table_named_as_an_earlier_table_or_a_defined_name_is_refused() {
    // Excel shows its repair dialog for T_Data, t_data and Base_Name, and
    // renames the later table's displayName.
    let duplicate = |name: &str, other: &str, defined_name| Diagnostic::TableNameDuplicate {
        name: Echo::new(name),
        other: Echo::new(other),
        defined_name,
    };
    let mut book = book();
    book.defined_names.push(name("Local", Some(1), "集計!$A$1"));
    // Excel drops these two first: no table is named as either.
    book.defined_names.push(name("Gone", Some(9), "1"));
    book.defined_names.push(name("Bad Name", None, "1"));
    book.tables.push(table(5, "gone", Some(2), "A1:B2"));
    book.tables.push(table(6, "bad name", Some(2), "D1:E2"));
    book.tables.push(table(2, "t_data", Some(1), "A1:B2"));
    book.tables.push(table(3, "BASE_NAME", Some(1), "A5:B6"));
    book.tables.push(table(4, "local", Some(1), "A8:B9"));
    let dropped = [
        Diagnostic::DefinedNameSheetIndexInvalid {
            name: Echo::new("Gone"),
            local_sheet_id: 9,
            sheets: 3,
        },
        Diagnostic::DefinedNameInvalid {
            name: Echo::new("Bad Name"),
        },
    ];
    assert_eq!(
        check(&book),
        [
            (dropped[0].clone(), workbook()),
            (dropped[1].clone(), workbook()),
            (
                duplicate("t_data", "T_Data", false),
                table_at("/xl/tables/table2.xml")
            ),
            (
                duplicate("BASE_NAME", "Base_Name", true),
                table_at("/xl/tables/table3.xml")
            ),
            (
                duplicate("local", "Local", true),
                table_at("/xl/tables/table4.xml")
            ),
        ]
    );
}

#[test]
fn a_shared_id_is_reported_at_the_later_table() {
    // Excel shows its repair dialog and drops the later table.
    let mut book = book();
    book.tables.push(table(2, "T_Extra", Some(2), "A1:B2"));
    book.tables[1].id = 1;
    assert_eq!(
        check(&book),
        [(
            Diagnostic::TableIdDuplicate {
                name: Echo::new("T_Extra"),
                other: Echo::new("T_Data"),
                id: 1,
            },
            table_at("/xl/tables/table2.xml")
        )]
    );
}
