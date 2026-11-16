//! Which cells are listed: the default window, a range, the rows and columns
//! asked for, the most cells, and the merged ranges that overlap them.

use super::{asking, codes, described, filled, rows_only, single, view_with, worksheet};
use crate::policy::limits::HostLimits;
use serde_json::{Value, json};
use xlsx_graft_diagnostics::Limit;
use xlsx_graft_spool::MemorySpools;

/// The `ref` of each listed cell of a grid's JSON, row by row.
fn listed(value: &Value) -> Vec<Vec<&str>> {
    (value["rows"].as_array().unwrap().iter())
        .map(|row| {
            (row["cells"].as_array().unwrap().iter())
                .map(|cell| cell["ref"].as_str().unwrap())
                .collect()
        })
        .collect()
}

/// The numbers of the rows listed.
fn row_numbers(value: &Value) -> Vec<u64> {
    (value["rows"].as_array().unwrap().iter())
        .map(|row| row["row"].as_u64().unwrap())
        .collect()
}

#[test]
fn the_default_window_is_a_hundred_rows_and_thirty_columns_from_a1() {
    let bytes = single(&rows_only(&filled(120, 32)));
    let value = described(&bytes, &asking(None, None, None));
    assert_eq!(value["used"], "A1:AF120");
    assert_eq!(value["shown"], "A1:AD100");
    assert_eq!(
        value["truncated"],
        json!({"rows": true, "columns": true, "merges": false})
    );
    assert_eq!(row_numbers(&value).len(), 100);
    assert_eq!(row_numbers(&value).last(), Some(&100));
    let last = value["rows"][99]["cells"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(last["ref"], "AD100");
    assert_eq!(value["rows"][0]["cells"].as_array().unwrap().len(), 30);
}

#[test]
fn rows_below_and_columns_to_the_right_set_truncated_each_by_itself() {
    let bytes = single(&rows_only(&filled(4, 3)));
    let below = described(&bytes, &asking(None, Some(3), Some(3)));
    assert_eq!(
        below["truncated"],
        json!({"rows": true, "columns": false, "merges": false})
    );
    assert_eq!(below["shown"], "A1:C3");
    let right = described(&bytes, &asking(None, Some(4), Some(2)));
    assert_eq!(
        right["truncated"],
        json!({"rows": false, "columns": true, "merges": false})
    );
    assert_eq!(right["shown"], "A1:B4");
    let whole = described(&bytes, &asking(None, Some(4), Some(3)));
    assert_eq!(
        whole["truncated"],
        json!({"rows": false, "columns": false, "merges": false})
    );
    assert_eq!(whole["used"], whole["shown"]);
}

#[test]
fn the_window_starts_at_the_top_left_of_the_range_and_stays_inside_it() {
    let bytes = single(&rows_only(&filled(10, 6)));
    let value = described(&bytes, &asking(Some("C3:E5"), None, None));
    assert_eq!(value["shown"], "C3:E5");
    assert_eq!(value["used"], "A1:F10");
    assert_eq!(
        listed(&value),
        [["C3", "D3", "E3"], ["C4", "D4", "E4"], ["C5", "D5", "E5"]]
    );
    // Cells outside the range are neither listed nor left out.
    assert_eq!(
        value["truncated"],
        json!({"rows": false, "columns": false, "merges": false})
    );
    let cut = described(&bytes, &asking(Some("B2:F9"), Some(2), Some(3)));
    assert_eq!(cut["shown"], "B2:D3");
    assert_eq!(
        cut["truncated"],
        json!({"rows": true, "columns": true, "merges": false})
    );
    assert_eq!(listed(&cut), [["B2", "C2", "D2"], ["B3", "C3", "D3"]]);
}

#[test]
fn a_range_may_be_given_with_its_corners_in_either_order_or_as_one_cell() {
    let bytes = single(&rows_only(&filled(5, 5)));
    let value = described(&bytes, &asking(Some("D4:B2"), None, None));
    assert_eq!(value["shown"], "B2:D4");
    let one = described(&bytes, &asking(Some("c3"), None, None));
    assert_eq!(one["shown"], "C3:C3");
    assert_eq!(listed(&one), [["C3"]]);
}

#[test]
fn rows_below_the_window_are_counted_once_each_in_the_range_only() {
    let rows = concat!(
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        r#"<row r="2"><c r="A2"><v>1</v></c><c r="B2"><v>1</v></c></row>"#,
        r#"<row r="3"><c r="A3"><v>1</v></c></row>"#,
        r#"<row r="9"><c r="Z9"><v>1</v></c></row>"#,
    );
    let bytes = single(&rows_only(rows));
    let value = super::view(&bytes, &asking(Some("A1:B8"), Some(1), None));
    assert_eq!(value.content.rows_below, 2);
    // Row 9 lies below the range, which it is not counted for.
    assert!(value.content.truncated.rows);
    let all = super::view(&bytes, &asking(None, Some(1), None));
    assert_eq!(all.content.rows_below, 3);
}

#[test]
fn a_range_with_no_cell_that_has_a_value_shows_nothing_but_the_used_range_stays() {
    let bytes = single(&rows_only(&filled(3, 3)));
    for range in ["Z50:AA60", "C4:D9", "D1:E3"] {
        let value = described(&bytes, &asking(Some(range), None, None));
        assert_eq!(value["shown"], Value::Null, "{range}");
        assert_eq!(value["rows"], json!([]), "{range}");
        assert_eq!(value["used"], "A1:C3");
        assert_eq!(
            value["truncated"],
            json!({"rows": false, "columns": false, "merges": false})
        );
    }
}

#[test]
fn the_range_shown_stops_at_the_last_row_and_column_with_a_value() {
    let rows = r#"<row r="3"><c r="C3"><v>1</v></c><c r="E3"><v>1</v></c></row><row r="5"><c r="D5"><v>1</v></c></row>"#;
    let bytes = single(&rows_only(rows));
    let value = described(&bytes, &asking(None, None, None));
    assert_eq!(value["used"], "C3:E5");
    assert_eq!(value["shown"], "A1:E5");
    let inside = described(&bytes, &asking(Some("D4:Z99"), None, None));
    assert_eq!(inside["shown"], "D4:E5");
    assert_eq!(listed(&inside), [["D5"]]);
}

#[test]
fn a_row_that_would_pass_the_most_cells_is_left_out_whole_with_the_rows_below() {
    // 103 rows of 1,000 cells: 100 rows fill the most cells exactly.
    let bytes = single(&rows_only(&filled(103, 1_000)));
    let value = described(&bytes, &asking(None, Some(200), Some(1_000)));
    assert_eq!(value["used"], "A1:ALL103");
    assert_eq!(value["shown"], "A1:ALL100");
    assert_eq!(
        value["truncated"],
        json!({"rows": true, "columns": false, "merges": false})
    );
    assert_eq!(row_numbers(&value).len(), 100);
    let cells: usize = (value["rows"].as_array().unwrap().iter())
        .map(|row| row["cells"].as_array().unwrap().len())
        .sum();
    assert_eq!(cells, 100_000);
    assert_eq!(
        super::view(&bytes, &asking(None, Some(200), Some(1_000)))
            .content
            .rows_below,
        3
    );
}

#[test]
fn the_merged_ranges_that_overlap_what_is_shown_are_listed_in_the_sheets_order() {
    let merges = concat!(
        r#"<mergeCells count="5"><mergeCell ref="F2:G3"/><mergeCell ref="A1:C1"/>"#,
        r#"<mergeCell ref="Z1:AA2"/><mergeCell ref="B3:B2"/><mergeCell ref="D5"/></mergeCells>"#,
    );
    let sheet = worksheet("", &filled(3, 8), merges);
    let bytes = single(&sheet);
    let value = described(&bytes, &asking(None, None, None));
    assert_eq!(value["shown"], "A1:H3");
    assert_eq!(value["merges"], json!(["F2:G3", "A1:C1", "B2:B3"]));
    let narrow = described(&bytes, &asking(None, None, Some(2)));
    assert_eq!(narrow["shown"], "A1:B3");
    assert_eq!(narrow["merges"], json!(["A1:C1", "B2:B3"]));
    let none = described(&bytes, &asking(Some("D1:E1"), None, None));
    assert_eq!(none["merges"], json!([]));
    let empty = described(&bytes, &asking(Some("Z50:AA60"), None, None));
    assert_eq!(empty["shown"], Value::Null);
    assert_eq!(empty["merges"], json!([]));
}

#[test]
fn more_merged_ranges_than_the_limit_keep_the_first_and_report_the_sheet() {
    let merges = concat!(
        r#"<mergeCells count="3"><mergeCell ref="A1:B1"/><mergeCell ref="A2:B2"/>"#,
        r#"<mergeCell ref="A3:B3"/></mergeCells>"#,
    );
    let bytes = single(&worksheet("", &filled(3, 2), merges));
    let mut limits = HostLimits::default();
    limits.verify.max_merged_cells = Limit::policy_of(2);
    let request = asking(None, None, None);
    let view = view_with(&bytes, &request, &limits, &mut MemorySpools);
    let value = crate::support::json(&view.to_json().unwrap());
    assert_eq!(codes(&value), ["merged_cells_too_many"]);
    assert_eq!(value["merges"], json!(["A1:B1", "A2:B2"]));
    assert_eq!(value["complete"], true);
    assert_eq!(value["ok"], false);
}

/// Rows `1..=rows`, each with one string of 1,000 characters in column A.
fn long_strings(rows: u32) -> String {
    let text = "x".repeat(1_000);
    (1..=rows)
        .map(|n| {
            format!(r#"<row r="{n}"><c r="A{n}" t="inlineStr"><is><t>{text}</t></is></c></row>"#)
        })
        .collect()
}

#[test]
fn strings_of_exactly_a_million_characters_in_all_are_listed() {
    let bytes = single(&rows_only(&long_strings(1_000)));
    let value = described(&bytes, &asking(None, Some(2_000), None));
    assert_eq!(value["shown"], "A1:A1000");
    assert_eq!(
        value["truncated"],
        json!({"rows": false, "columns": false, "merges": false})
    );
    assert_eq!(row_numbers(&value).len(), 1_000);
}

#[test]
fn a_row_that_would_pass_a_million_characters_is_left_out_with_the_rows_below() {
    let bytes = single(&rows_only(&long_strings(1_002)));
    let request = asking(None, Some(2_000), None);
    let value = described(&bytes, &request);
    assert_eq!(value["used"], "A1:A1002");
    assert_eq!(value["shown"], "A1:A1000");
    assert_eq!(
        value["truncated"],
        json!({"rows": true, "columns": false, "merges": false})
    );
    assert_eq!(row_numbers(&value).len(), 1_000);
    assert_eq!(super::view(&bytes, &request).content.rows_below, 2);
}

#[test]
fn a_sheet_of_long_strings_is_cut_short_with_the_default_window_too() {
    // 40 columns of 1,000 characters in each of 60 rows: the window is 100
    // rows by 30 columns, 30 strings of 1,000 characters a row, so the 34th
    // row would pass a million characters.
    let text = "y".repeat(1_000);
    let rows: String = (1..=60)
        .map(|n| {
            let cells: String = (1..=40)
                .map(|c| {
                    let at = xlsx_graft_read::CellRef::new(c, n).unwrap();
                    format!(r#"<c r="{at}" t="inlineStr"><is><t>{text}</t></is></c>"#)
                })
                .collect();
            format!(r#"<row r="{n}">{cells}</row>"#)
        })
        .collect();
    let value = described(&single(&rows_only(&rows)), &asking(None, None, None));
    assert_eq!(value["shown"], "A1:AD33");
    assert_eq!(
        value["truncated"],
        json!({"rows": true, "columns": true, "merges": false})
    );
}

/// A sheet with a value in A1 and B2 and `count` merged ranges, each `A1:B2`.
fn merged(count: usize) -> Vec<u8> {
    let rows = concat!(
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        r#"<row r="2"><c r="B2"><v>2</v></c></row>"#,
    );
    let merges = r#"<mergeCell ref="A1:B2"/>"#.repeat(count);
    single(&worksheet(
        "",
        rows,
        &format!("<mergeCells>{merges}</mergeCells>"),
    ))
}

#[test]
fn a_hundred_thousand_merged_ranges_are_listed_and_one_more_is_cut_and_says_so() {
    let request = asking(None, None, None);
    let exact = described(&merged(100_000), &request);
    assert_eq!(exact["merges"].as_array().unwrap().len(), 100_000);
    assert_eq!(exact["truncated"]["merges"], false);
    let past = described(&merged(100_001), &request);
    assert_eq!(past["merges"].as_array().unwrap().len(), 100_000);
    assert_eq!(past["truncated"]["merges"], true);
    assert_eq!(past["truncated"]["rows"], false);
}

#[test]
fn merged_ranges_the_limit_of_the_sheet_cut_set_truncated_merges() {
    let mut limits = HostLimits::default();
    limits.verify.max_merged_cells = Limit::policy_of(2);
    let request = asking(None, None, None);
    let view = view_with(&merged(3), &request, &limits, &mut MemorySpools);
    let value = crate::support::json(&view.to_json().unwrap());
    assert_eq!(value["merges"], json!(["A1:B2", "A1:B2"]));
    assert_eq!(value["truncated"]["merges"], true);
    let whole = described(&merged(3), &request);
    assert_eq!(whole["truncated"]["merges"], false);
}
