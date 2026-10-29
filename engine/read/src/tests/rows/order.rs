//! Where rows and cells stand: a row or cell out of order, a reference
//! that names no place in the sheet or another row, and a row or cell
//! without `r` past the last one. Each stops the reading there, after the
//! rows before it.

use super::{SHEET, Shown, row, rows};
use xlsx_graft_diagnostics::{Diagnostic, Echo};

/// What reading a sheet of `body` stops at: the rows before, and the one
/// diagnostic, which points to the sheet.
fn stops(body: &str) -> (Vec<Shown>, Diagnostic) {
    let (rows, mut seen) = rows(body);
    assert_eq!(seen.len(), 1, "{body}: {seen:?}");
    let (diagnostic, place) = seen.remove(0);
    assert_eq!(place, SHEET);
    (rows, diagnostic)
}

/// `cell_order_invalid` of `reference` after `previous`.
fn order(reference: &str, previous: &str) -> Diagnostic {
    Diagnostic::CellOrderInvalid {
        reference: Echo::new(reference),
        previous: Echo::new(previous),
    }
}

/// `cell_reference_invalid` of `reference`.
fn reference(reference: &str) -> Diagnostic {
    Diagnostic::CellReferenceInvalid {
        reference: Echo::new(reference),
    }
}

#[test]
fn a_row_at_or_before_the_one_before_is_out_of_order() {
    let first = row(3, &["n:1"]);
    for (body, previous) in [
        (r#"<row r="3"><c><v>1</v></c></row><row r="2"/>"#, "2"),
        (r#"<row r="3"><c><v>1</v></c></row><row r="3"/>"#, "3"),
    ] {
        let (rows, diagnostic) = stops(body);
        assert_eq!(rows, vec![first.clone()]);
        assert_eq!(diagnostic, order(previous, "3"));
    }
}

#[test]
fn a_cell_at_or_before_the_one_before_in_its_row_is_out_of_order() {
    for (body, expected) in [
        (r#"<row><c r="C1"/><c r="B1"/></row>"#, order("B1", "C1")),
        (r#"<row><c r="B1"/><c r="b1"/></row>"#, order("B1", "B1")),
        (
            r#"<row r="2"><c r="AA2"/><c r="Z2"/></row>"#,
            order("Z2", "AA2"),
        ),
    ] {
        assert_eq!(stops(body), (vec![], expected), "{body}");
    }
}

#[test]
fn a_reference_that_names_no_cell_of_its_row_is_refused() {
    for (body, expected) in [
        (r#"<row><c r="A2"/></row>"#, "A2"),
        (r#"<row><c r="a02"/></row>"#, "A2"),
        (r#"<row r="2"><c r="A1"/></row>"#, "A1"),
        (r#"<row><c r="$A$1"/></row>"#, "$A$1"),
        (r#"<row><c r="A1:B2"/></row>"#, "A1:B2"),
        (r#"<row><c r="XFE1"/></row>"#, "XFE1"),
        (r#"<row><c r="A0"/></row>"#, "A0"),
        (r#"<row><c r=""/></row>"#, ""),
    ] {
        assert_eq!(stops(body), (vec![], reference(expected)), "{body}");
    }
}

#[test]
fn a_row_number_outside_the_sheet_is_refused() {
    for (body, expected) in [
        (r#"<row r="0"/>"#, "0"),
        (r#"<row r="1048577"/>"#, "1048577"),
        (r#"<row r=" 01048577 "/>"#, " 01048577 "),
    ] {
        assert_eq!(stops(body), (vec![], reference(expected)), "{body}");
    }
}

#[test]
fn a_row_or_cell_without_r_past_the_last_is_refused() {
    let (rows, diagnostic) = stops(r#"<row r="1048576"/><row/>"#);
    assert_eq!(
        (rows, diagnostic),
        (vec![row(1_048_576, &[])], reference("1048577"))
    );
    let (rows, diagnostic) = stops(r#"<row r="7"><c r="XFD7"/><c/></row>"#);
    assert_eq!((rows, diagnostic), (vec![], reference("XFE7")));
}

#[test]
fn the_rows_before_a_problem_come_and_none_after() {
    let body = concat!(
        r#"<row><c><v>1</v></c></row>"#,
        r#"<row><c><v>2</v></c></row>"#,
        r#"<row r="1"><c><v>3</v></c></row>"#,
        r#"<row r="9"><c><v>4</v></c></row>"#,
    );
    let (rows, diagnostic) = stops(body);
    assert_eq!(rows, vec![row(1, &["n:1"]), row(2, &["n:2"])]);
    assert_eq!(diagnostic, order("1", "2"));
}
