//! Where the rows and cells of a sheet stand: inside the sheet, each past
//! the one before, and what comes after one refused.

use super::{check, invalid, order, rows};

#[test]
fn a_row_that_goes_back_or_repeats_one_is_refused_and_not_kept() {
    // Excel for Mac 16.114.3 offers to repair both, dropping the row.
    assert_eq!(
        check(&rows(r#"<row r="2"/><row r="1"/>"#)),
        [order("1", "2")]
    );
    assert_eq!(
        check(&rows(r#"<row r="1"/><row r="1"/>"#)),
        [order("1", "1")]
    );
    // The row refused is not kept: the next is placed after row 3.
    let xml = rows(r#"<row r="3"/><row r="2"/><row/><row r="4"/>"#);
    assert_eq!(check(&xml), [order("2", "3"), order("4", "4")]);
}

#[test]
fn a_row_number_is_from_1_to_1048576() {
    // Excel for Mac 16.114.3 offers to repair row 1,048,577, dropping it.
    assert_eq!(check(&rows(r#"<row r="1048576"/>"#)), []);
    assert_eq!(check(&rows(r#"<row r="1048577"/>"#)), [invalid("1048577")]);
    assert_eq!(check(&rows(r#"<row r="0"/>"#)), [invalid("0")]);
    assert_eq!(check(&rows(r#"<row r="x"/>"#)), [invalid("x")]);
    assert_eq!(
        check(&rows(r#"<row r="99999999999"/>"#)),
        [invalid("99999999999")]
    );
    // Without `r`, a row after the last one would be past it.
    let xml = rows(r#"<row r="1048576"/><row/>"#);
    assert_eq!(check(&xml), [invalid("1048577")]);
}

#[test]
fn a_cell_that_goes_back_or_repeats_one_is_refused_and_not_kept() {
    // Excel for Mac 16.114.3 offers to repair both, dropping the cell.
    let xml = rows(r#"<row r="2"><c r="B2"/><c r="A2"/><c/></row>"#);
    assert_eq!(check(&xml), [order("A2", "B2")]);
    let xml = rows(r#"<row r="2"><c r="A2"/><c r="A2"/><c r="C2"/><c/></row>"#);
    assert_eq!(check(&xml), [order("A2", "A2")]);
    // A cell without `r` comes after the last one kept.
    let xml = rows(r#"<row r="2"><c r="C2"/><c r="A2"/><c/><c r="D2"/></row>"#);
    assert_eq!(check(&xml), [order("A2", "C2"), order("D2", "D2")]);
}

#[test]
fn a_cell_is_up_to_xfd_in_its_own_row() {
    // Excel for Mac 16.114.3 moves XFE2 to the next column without a word:
    // the value is no longer where the workbook puts it, so it is refused.
    let xml = rows(r#"<row r="2"><c r="B2"/><c r="XFE2"/><c r="XFD2"/></row>"#);
    assert_eq!(check(&xml), [invalid("XFE2")]);
    let xml = rows(r#"<row r="2"><c r="XFD2"/><c/></row>"#);
    assert_eq!(check(&xml), [invalid("XFE2")]);
    let xml = rows(r#"<row r="2"><c r="b03"/><c r="$C$2"/><c r=""/></row>"#);
    assert_eq!(check(&xml), [invalid("B3"), invalid("$C$2"), invalid("")]);
}

#[test]
fn the_cells_of_a_row_refused_are_not_placed() {
    let xml =
        rows(r#"<row r="2"/><row r="1"><c r="Z9"/><c r="A1"/></row><row r="3"><c r="A3"/></row>"#);
    assert_eq!(check(&xml), [order("1", "2")]);
}

#[test]
fn rows_and_cells_without_r_follow_the_ones_before() {
    let xml =
        rows(r#"<row><c/><c/></row><row><c r="A2"/><c/></row><row r="5"><c r="C5"/><c/></row>"#);
    assert_eq!(check(&xml), []);
}
