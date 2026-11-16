//! `describe-sheet` on `fixtures/merged.xlsx`: the view in each format, the
//! sheet and the window asked for, and the help. The JSON case pins the
//! contract: a whole line is compared. What goes wrong is in
//! `describe_sheet_failures`.

use crate::{fixture, printed, run, stderr, stdout};
use serde_json::json;

#[test]
fn json_of_the_merged_workbook_is_the_whole_contract() {
    let output = run(&[
        "describe-sheet",
        &fixture("merged.xlsx"),
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let rows = [
        r#"{"row":1,"cells":[{"ref":"A1","type":"string","value":"Quarterly report"}]}"#,
        concat!(
            r#"{"row":2,"cells":[{"ref":"A2","type":"string","value":"Group"},"#,
            r#"{"ref":"B2","type":"string","value":"Item"},"#,
            r#"{"ref":"C2","type":"string","value":"Amount"},"#,
            r#"{"ref":"E2","type":"string","value":"after the gap"}]}"#,
        ),
        concat!(
            r#"{"row":3,"cells":[{"ref":"A3","type":"string","value":"Fruit"},"#,
            r#"{"ref":"B3","type":"string","value":"apple"},"#,
            r#"{"ref":"C3","type":"number","value":1.5}]}"#,
        ),
        concat!(
            r#"{"row":4,"cells":[{"ref":"B4","type":"string","value":"line one\nline two"},"#,
            r#"{"ref":"C4","type":"number","value":2.0}]}"#,
        ),
        concat!(
            r#"{"row":5,"cells":[{"ref":"B5","type":"string","value":"a|b"},"#,
            r#"{"ref":"C5","type":"number","value":3.0}]}"#,
        ),
        concat!(
            r#"{"row":6,"cells":[{"ref":"B6","type":"string","value":"date"},"#,
            r#"{"ref":"C6","type":"date","value":"2024-01-15"}]}"#,
        ),
        concat!(
            r#"{"row":8,"cells":[{"ref":"A8","type":"string","value":"Total"},"#,
            r#"{"ref":"C8","type":"number","value":6.5}]}"#,
        ),
    ];
    let expected = format!(
        concat!(
            r#"{{"ok":true,"error":null,"diagnostics":[],"#,
            r#""sheet":{{"index":0,"name":"Sheet1","kind":"worksheet","state":"visible"}},"#,
            r#""dimension":"A1:E8","used":"A1:E8","shown":"A1:E8","#,
            r#""truncated":{{"rows":false,"columns":false,"merges":false}},"#,
            r#""complete":true,"rows":[{}],"merges":["A1:C1","A3:A5"]}}"#,
            "\n",
        ),
        rows.join(","),
    );
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
}

#[test]
fn text_of_the_merged_workbook_is_a_table_and_the_merged_ranges() {
    let output = run(&["describe-sheet", &fixture("merged.xlsx")]);
    assert_eq!(output.status.code(), Some(0));
    let expected = concat!(
        "Sheet1 (index 0, worksheet, visible) — used A1:E8, dimension A1:E8, shown A1:E8\n",
        "|  | A | B | C | D | E |\n",
        "| --- | --- | --- | --- | --- | --- |\n",
        "| 1 | Quarterly report |  |  |  |  |\n",
        "| 2 | Group | Item | Amount |  | after the gap |\n",
        "| 3 | Fruit | apple | 1.5 |  |  |\n",
        "| 4 |  | line one\\nline two | 2 |  |  |\n",
        "| 5 |  | a\\|b | 3 |  |  |\n",
        "| 6 |  | date | 2024-01-15 |  |  |\n",
        "| 7 |  |  |  |  |  |\n",
        "| 8 | Total |  | 6.5 |  |  |\n",
        "Merged: A1:C1, A3:A5\n",
        "16 cells, 0 errors, 0 warnings, 0 notices — ok\n",
    );
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_range_starts_the_window_and_limits_the_merged_ranges_shown() {
    let book = fixture("merged.xlsx");
    let output = run(&[
        "describe-sheet",
        &book,
        "--range",
        "B2:C3",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let view = printed(&output);
    assert_eq!(view["shown"], "B2:C3");
    assert_eq!(view["used"], "A1:E8");
    assert_eq!(view["merges"], json!([]));
    let cells: Vec<&str> = (view["rows"].as_array().unwrap().iter())
        .flat_map(|row| row["cells"].as_array().unwrap())
        .map(|cell| cell["ref"].as_str().unwrap())
        .collect();
    assert_eq!(cells, ["B2", "C2", "B3", "C3"]);
}

#[test]
fn max_rows_and_max_columns_cut_the_window_and_say_so() {
    let book = fixture("merged.xlsx");
    let output = run(&[
        "describe-sheet",
        &book,
        "--max-rows",
        "2",
        "--format",
        "json",
    ]);
    let view = printed(&output);
    assert_eq!(view["shown"], "A1:E2");
    assert_eq!(
        view["truncated"],
        json!({"rows": true, "columns": false, "merges": false})
    );
    let output = run(&[
        "describe-sheet",
        &book,
        "--max-columns",
        "1",
        "--format",
        "json",
    ]);
    let view = printed(&output);
    assert_eq!(view["shown"], "A1:A8");
    assert_eq!(view["truncated"]["columns"], true);
    assert_eq!(view["truncated"]["rows"], false);
}

#[test]
fn a_sheet_is_chosen_by_name_without_regard_to_case_or_by_index() {
    let book = fixture("merged.xlsx");
    for choice in [["--sheet", "sheet1"], ["--sheet-index", "0"]] {
        let output = run(&[
            "describe-sheet",
            &book,
            choice[0],
            choice[1],
            "--format",
            "json",
        ]);
        assert_eq!(output.status.code(), Some(0), "{choice:?}");
        let view = printed(&output);
        assert_eq!(view["sheet"]["name"], "Sheet1");
        assert_eq!(view["shown"], "A1:E8");
    }
}

#[test]
fn the_help_of_describe_sheet_gives_the_json_contract_the_bounds_and_the_exit_status() {
    let output = run(&["describe-sheet", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = stdout(&output);
    assert!(help.contains("Usage: xlsx-graft describe-sheet [OPTIONS] <PATH>"));
    assert!(help.contains(r#""truncated": {"rows": BOOL"#));
    assert!(help.contains(r#""truncated": true, only when"#));
    assert!(help.contains("1,000,000 characters in all"));
    assert!(help.contains("tidied for display"));
    assert!(help.contains("Exit status: 0 when ok."));
}
