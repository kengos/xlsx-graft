//! Unit tests for the text of a grid: the whole text of one, each way a
//! cell is written, and the lines that are there only sometimes.

use super::GridText;
use crate::policy::limits::HostLimits;
use crate::sheet::tests::{
    Unreadable, asking, book, rel_type, rows_only, single, view, view_with, workbook, worksheet,
};
use crate::sheet::{Grid, SheetRequest};
use crate::view::View;
use xlsx_graft_read::SheetChoice;
use xlsx_graft_spool::MemorySpools;

/// The text of the view of the workbook `bytes` for `request`.
fn text(bytes: &[u8], request: &SheetRequest) -> String {
    GridText(&view(bytes, request)).to_string()
}

/// The text of a view.
fn written(view: &View<Grid>) -> String {
    GridText(view).to_string()
}

#[test]
fn a_grid_is_a_heading_a_table_what_was_left_out_and_a_last_line() {
    let rows = concat!(
        r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Name</t></is></c><c r="B1" t="s"><v>1</v></c>"#,
        r#"<c r="C1" s="1"><v>45306</v></c><c r="E1"><v>5</v></c></row>"#,
        r#"<row r="2"><c r="A2" t="s"><v>2</v></c><c r="C2"><v>3</v></c></row>"#,
        "<row r=\"3\"><c r=\"A3\" t=\"inlineStr\"><is><t>x\ty\\z&#13;</t></is></c></row>",
        r#"<row r="5"><c r="A5"><v>9</v></c></row>"#,
    );
    let merges =
        r#"<mergeCells count="2"><mergeCell ref="A1:B1"/><mergeCell ref="A5:B5"/></mergeCells>"#;
    let bytes = single(&worksheet(r#"<dimension ref="A1:E5"/>"#, rows, merges));
    let expected = concat!(
        "Sheet1 (index 0, worksheet, visible) — used A1:E5, dimension A1:E5, shown A1:C3\n",
        "|  | A | B | C |\n",
        "| --- | --- | --- | --- |\n",
        "| 1 | Name | a\\|b | 2024-01-15 |\n",
        "| 2 | line\\nbreak |  | 3 |\n",
        "| 3 | x\\ty\\\\z\\r |  |  |\n",
        "Merged: A1:B1\n",
        "… 1 more row with values below row 3\n",
        "… more columns with values right of column C\n",
        "6 cells, 0 errors, 0 warnings, 0 notices — ok\n",
    );
    assert_eq!(text(&bytes, &asking(None, Some(3), Some(3))), expected);
}

#[test]
fn each_value_is_written_as_read_writes_it() {
    let cells = concat!(
        r#"<c r="A1"><v>0</v></c><c r="B1"><v>0.00001</v></c><c r="C1"><v>0.000001</v></c>"#,
        r#"<c r="D1"><v>1234567890123456</v></c><c r="E1"><v>10000000000000000</v></c>"#,
        r#"<c r="F1"><v>-2.5</v></c><c r="G1" t="b"><v>1</v></c><c r="H1" t="b"><v>0</v></c>"#,
        r#"<c r="I1" s="1"><v>45306</v></c><c r="J1" s="2"><v>0.5</v></c>"#,
        r#"<c r="K1" s="3"><v>45306.354166666664</v></c><c r="L1" s="4"><v>1.0854166666666667</v></c>"#,
        r#"<c r="M1" t="e"><v>#N/A</v></c>"#,
    );
    let bytes = single(&rows_only(&format!("<row>{cells}</row>")));
    let written = text(&bytes, &asking(None, None, None));
    let line = written.lines().nth(3).unwrap();
    assert_eq!(
        line,
        concat!(
            "| 1 | 0 | 0.00001 | 1e-6 | 1234567890123456 | 1e16 | -2.5 | TRUE | FALSE ",
            "| 2024-01-15 | 12:00:00.000 | 2024-01-15T08:30:00.000 | 26:03:00.000 | #N/A |"
        )
    );
}

#[test]
fn a_cut_string_ends_in_an_ellipsis_after_its_escapes() {
    let long = format!("a|{}", "b".repeat(1_000));
    let cell = format!(r#"<row><c r="A1" t="inlineStr"><is><t>{long}</t></is></c></row>"#);
    let written = text(&single(&rows_only(&cell)), &asking(None, None, None));
    let line = written.lines().nth(3).unwrap();
    assert!(line.starts_with("| 1 | a\\|bbb"), "{line}");
    assert!(line.ends_with("bbb… |"), "{line}");
    // The cap counts the 1,000 characters of the text (`a`, `|` and 998
    // `b`), not the escape's backslash.
    assert_eq!(line.matches('b').count(), 998);
}

#[test]
fn a_sheet_with_nothing_to_show_says_so() {
    let written = text(&single(&rows_only("")), &asking(None, None, None));
    assert_eq!(
        written,
        concat!(
            "Sheet1 (index 0, worksheet, visible) — used none, dimension none, shown none\n",
            "nothing to show\n",
            "0 cells, 0 errors, 0 warnings, 0 notices — ok\n",
        )
    );
}

#[test]
fn a_range_with_no_value_shows_nothing_beside_the_used_range() {
    let rows = r#"<row r="1"><c r="A1"><v>1</v></c></row>"#;
    let written = text(
        &single(&rows_only(rows)),
        &asking(Some("C3:D4"), None, None),
    );
    assert!(written.starts_with(concat!(
        "Sheet1 (index 0, worksheet, visible) — used A1:A1, dimension none, shown none\n",
        "nothing to show\n",
    )));
}

#[test]
fn the_columns_are_lettered_and_a_row_may_be_empty_between_two() {
    let rows = concat!(
        r#"<row r="2"><c r="Z2"><v>1</v></c><c r="AB2"><v>2</v></c></row>"#,
        r#"<row r="4"><c r="XFD4"><v>3</v></c></row>"#,
    );
    let bytes = single(&rows_only(rows));
    let written = text(&bytes, &asking(Some("Z2:AB4"), None, None));
    assert_eq!(
        written,
        concat!(
            "Sheet1 (index 0, worksheet, visible) — used Z2:XFD4, dimension none, shown Z2:AB4\n",
            "|  | Z | AA | AB |\n",
            "| --- | --- | --- | --- |\n",
            "| 2 | 1 |  | 2 |\n",
            "| 3 |  |  |  |\n",
            "| 4 |  |  |  |\n",
            "2 cells, 0 errors, 0 warnings, 0 notices — ok\n",
        )
    );
    let last = text(&bytes, &asking(Some("XFD4"), None, None));
    assert!(
        last.contains("|  | XFD |\n| --- | --- |\n| 4 | 3 |\n"),
        "{last}"
    );
}

#[test]
fn rows_below_are_counted_in_the_plural_and_the_one_cell_is_singular() {
    let rows = concat!(
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        r#"<row r="2"><c r="A2"><v>2</v></c></row>"#,
        r#"<row r="3"><c r="A3"><v>3</v></c></row>"#,
    );
    let written = text(&single(&rows_only(rows)), &asking(None, Some(1), None));
    assert!(
        written.contains("… 2 more rows with values below row 1\n"),
        "{written}"
    );
    assert!(written.ends_with("1 cell, 0 errors, 0 warnings, 0 notices — ok\n"));
}

#[test]
fn a_sheet_that_stops_at_a_problem_has_its_rows_then_its_diagnostics_and_is_not_ok() {
    let rows = concat!(
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        r#"<row r="2"><c r="B2" t="e"><v></v></c></row>"#,
    );
    let written = text(&single(&rows_only(rows)), &asking(None, None, None));
    let (table, rest) = written.split_once("\n\n").unwrap();
    assert_eq!(
        table,
        concat!(
            "Sheet1 (index 0, worksheet, visible) — used A1:A1, dimension none, shown A1:A1\n",
            "|  | A |\n",
            "| --- | --- |\n",
            "| 1 | 1 |"
        )
    );
    assert!(
        rest.starts_with("error\n  xml_schema_invalid /xl/worksheets/sheet1.xml"),
        "{rest}"
    );
    assert!(
        rest.ends_with("1 cell, 1 error, 0 warnings, 0 notices — not ok\n"),
        "{rest}"
    );
}

#[test]
fn a_sheet_that_is_not_read_has_could_not_read_in_place_of_the_heading() {
    let bytes = book(&[("A", &rows_only(""))]);
    let named = |choice| {
        let request = SheetRequest::new(Some(choice), None, None, None).unwrap();
        text(&bytes, &request)
    };
    assert_eq!(
        named(SheetChoice::Name("nope")),
        concat!(
            "could not read: no sheet is named \"nope\"\n",
            "0 cells, 0 errors, 0 warnings, 0 notices — not ok\n",
        )
    );
    let chart = r#"<sheet name="Chart" sheetId="1" r:id="rId1"/>"#;
    let related = crate::support::rel_of("rId1", &rel_type("chartsheet"), "chartsheets/c.xml");
    let bytes = workbook(
        chart,
        &related,
        &[("xl/chartsheets/c.xml", "<chartsheet/>")],
    );
    assert_eq!(
        text(&bytes, &asking(None, None, None)),
        concat!(
            "could not read: sheet \"Chart\" is a chartsheet, which holds no cells\n",
            "0 cells, 0 errors, 0 warnings, 0 notices — not ok\n",
        )
    );
    let bytes = b"not a zip".as_slice();
    let written = text(bytes, &asking(None, None, None));
    assert!(
        written.starts_with(
            "could not read: the workbook cannot be read; the diagnostics say why\n\n"
        )
    );
}

#[test]
fn the_rows_read_before_a_failure_of_the_host_are_written_after_could_not_read() {
    let rows =
        r#"<row r="1"><c r="A1"><v>1</v></c></row><row r="2"><c r="A2" t="s"><v>0</v></c></row>"#;
    let mut limits = HostLimits::default();
    limits.read.shared_strings_spill_bytes = xlsx_graft_diagnostics::Limit::policy_of(0);
    let view = view_with(
        &single(&rows_only(rows)),
        &asking(None, None, None),
        &limits,
        &mut Unreadable,
    );
    let written = written(&view);
    let (first, rest) = written.split_once('\n').unwrap();
    assert!(
        first.starts_with("could not read: the shared strings could not be moved"),
        "{first}"
    );
    assert_eq!(
        rest,
        concat!(
            "|  | A |\n| --- | --- |\n| 1 | 1 |\n",
            "1 cell, 0 errors, 0 warnings, 0 notices — not ok\n"
        )
    );
}

#[test]
fn the_heading_names_the_sheet_by_its_state_and_a_hidden_sheet_is_still_read() {
    let tags = r#"<sheet name="Pla&#x202e;n" sheetId="1" state="veryHidden" r:id="rId1"/>"#;
    let related = crate::support::rel_of("rId1", &rel_type("worksheet"), "worksheets/p.xml");
    let sheet = worksheet(
        r#"<dimension ref="A1&#x85;"/>"#,
        r#"<row><c r="A1"><v>1</v></c></row>"#,
        "",
    );
    let bytes = workbook(tags, &related, &[("xl/worksheets/p.xml", &sheet)]);
    let written = text(&bytes, &asking(None, None, None));
    assert_eq!(
        written.lines().next().unwrap(),
        "Plan (index 0, worksheet, veryHidden) — used A1:A1, dimension A1, shown A1:A1"
    );
}

#[test]
fn a_table_of_a_hundred_thousand_cells_is_drawn_and_a_larger_one_is_a_list() {
    let rows = |last: u32| {
        let at = xlsx_graft_read::CellRef::new(1_000, last).unwrap();
        let rows = format!(
            r#"<row r="1"><c r="A1"><v>1</v></c></row><row r="{last}"><c r="{at}" t="inlineStr"><is><t>a|b</t></is></c></row>"#
        );
        single(&rows_only(&rows))
    };
    let drawn = text(&rows(100), &asking(None, Some(101), Some(1_000)));
    assert!(drawn.contains("| 100 |"), "{}", &drawn[..300]);
    assert!(!drawn.contains("too large"));
    let listed = text(&rows(101), &asking(None, Some(101), Some(1_000)));
    assert_eq!(
        listed,
        concat!(
            "Sheet1 (index 0, worksheet, visible) — used A1:ALL101, dimension none, shown A1:ALL101\n",
            "table of 101 rows and 1000 columns is too large to draw; the cells with values follow\n",
            "A1: 1\n",
            "ALL101: a\\|b\n",
            "2 cells, 0 errors, 0 warnings, 0 notices — ok\n",
        )
    );
}

#[test]
fn merged_ranges_that_were_cut_are_followed_by_a_line_saying_so() {
    let rows = r#"<row r="1"><c r="A1"><v>1</v></c><c r="B1"><v>2</v></c></row>"#;
    let merges = r#"<mergeCells><mergeCell ref="A1:B1"/><mergeCell ref="A1:A1"/></mergeCells>"#;
    let bytes = single(&worksheet("", rows, merges));
    let mut limits = HostLimits::default();
    limits.verify.max_merged_cells = xlsx_graft_diagnostics::Limit::policy_of(1);
    let view = view_with(
        &bytes,
        &asking(None, None, None),
        &limits,
        &mut MemorySpools,
    );
    let written = written(&view);
    assert!(
        written.contains("Merged: A1:B1\n… the sheet has more merged ranges than were read; some may overlap the range shown\n"),
        "{written}"
    );
    let two = text(&bytes, &asking(None, None, None));
    assert!(two.contains("Merged: A1:B1, A1:A1\n"), "{two}");
    assert!(!two.contains("more merged"));
}

#[test]
fn more_merged_ranges_than_are_listed_say_that_they_overlap_the_range_shown() {
    let rows = r#"<row r="1"><c r="A1"><v>1</v></c></row>"#;
    let merges = r#"<mergeCell ref="A1"/>"#.repeat(100_001);
    let bytes = single(&worksheet(
        "",
        rows,
        &format!("<mergeCells>{merges}</mergeCells>"),
    ));
    let written = text(&bytes, &asking(None, None, None));
    assert!(written.contains("\n… more merged ranges overlap the range shown\n"));
    assert!(!written.contains("than were read"));
}
