//! `read` on a workbook it reads to the end: each value in each format,
//! the sheet asked for, and the help. The JSON cases pin the contract:
//! whole lines are compared, so a renamed, added or retyped key fails
//! here.

use crate::workbook::{worksheet, write, xlsx};
use crate::{run, stderr, stdout};
use serde_json::{Value, json};

/// A row of each kind of value, with an empty column and a row left out:
/// a shared string, an inline string, then a number, a boolean, an error
/// value, a formula's string and an ISO 8601 date-time, then a date, a
/// time, a date-time and elapsed time made of numbers by their formats.
const ROWS: &str = concat!(
    r#"<row r="1"><c r="A1" t="s"><v>0</v></c>"#,
    r#"<c r="C1" t="inlineStr"><is><t>in&#9;line</t></is></c></row>"#,
    r#"<row r="3"><c r="A3"><v>3</v></c><c r="B3" t="b"><v>1</v></c>"#,
    r#"<c r="C3" t="e"><v>#N/A</v></c><c r="D3" t="str"><f>A3</f><v>formula</v></c>"#,
    r#"<c r="E3" t="d"><v>2024-01-31T08:30:00</v></c></row>"#,
    r#"<row r="4"><c r="A4" s="1"><v>45322</v></c><c r="B4" s="2"><v>0.5</v></c>"#,
    r#"<c r="C4" s="3"><v>45322.25</v></c><c r="D4" s="4"><v>1.5</v></c></row>"#,
);

/// Each line of a JSON Lines `stdout`.
fn lines(output: &std::process::Output) -> Vec<Value> {
    (stdout(output).lines())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn json_prints_a_typed_line_a_row_then_the_end() {
    let (_dir, path) = write(&xlsx(&[worksheet("Data", ROWS)]));
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let string = |value: &str| json!({ "type": "string", "value": value });
    let typed = |kind: &str, value: Value| json!({ "type": kind, "value": value });
    assert_eq!(
        lines(&output),
        [
            json!({ "row": 1, "cells": [
                string("a"),
                typed("empty", Value::Null),
                string("in\tline"),
            ] }),
            json!({ "row": 3, "cells": [
                typed("number", json!(3.0)),
                typed("boolean", json!(true)),
                typed("error", json!("#N/A")),
                string("formula"),
                typed("datetime", json!("2024-01-31T08:30:00.000")),
            ] }),
            json!({ "row": 4, "cells": [
                typed("date", json!("2024-01-31")),
                typed("time", json!("12:00:00.000")),
                typed("datetime", json!("2024-01-31T06:00:00.000")),
                typed("elapsed", json!(129_600_000)),
            ] }),
            json!({
                "ok": true,
                "sheet": { "index": 0, "name": "Data" },
                "rows": 3,
                "error": null,
                "diagnostics": [],
            }),
        ]
    );
    // A number keeps its fraction, so it reads as a float.
    assert!(stdout(&output).contains(r#"{"type":"number","value":3.0}"#));
}

#[test]
fn text_prints_a_tab_separated_line_a_row_and_nothing_more() {
    let (_dir, path) = write(&xlsx(&[worksheet("Data", ROWS)]));
    let output = run(&["read", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        concat!(
            "1\ta\t\tin\\tline\n",
            "3\t3\tTRUE\t#N/A\tformula\t2024-01-31T08:30:00.000\n",
            "4\t2024-01-31\t12:00:00.000\t2024-01-31T06:00:00.000\t36:00:00.000\n",
        )
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_sheet_is_chosen_by_name_letter_case_aside_or_by_index() {
    let other = r#"<row r="2"><c r="B2" t="s"><v>1</v></c></row>"#;
    let sheets = [worksheet("Data", ROWS), worksheet("Other", other)];
    let (_dir, path) = write(&xlsx(&sheets));
    let path = path.to_str().unwrap();
    let ends = [
        &["read", path, "--sheet", "oTHER", "--format", "json"][..],
        &["--format=json", "read", "--sheet-index", "1", path],
    ];
    for args in ends {
        let output = run(args);
        assert_eq!(output.status.code(), Some(0));
        let lines = lines(&output);
        assert_eq!(lines.len(), 2);
        let empty = json!({ "type": "empty", "value": null });
        let b = json!({ "type": "string", "value": "b" });
        assert_eq!(lines[0], json!({ "row": 2, "cells": [empty, b] }));
        assert_eq!(lines[1]["sheet"], json!({ "index": 1, "name": "Other" }));
        assert_eq!(lines[1]["rows"], 1);
    }
}

#[test]
fn a_sheet_with_no_rows_prints_the_end_alone() {
    let (_dir, path) = write(&xlsx(&[worksheet("Empty", "")]));
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        lines(&output),
        [json!({
            "ok": true,
            "sheet": { "index": 0, "name": "Empty" },
            "rows": 0,
            "error": null,
            "diagnostics": [],
        })]
    );
    let output = run(&["read", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_row_without_cells_is_its_number_alone() {
    let (_dir, path) = write(&xlsx(&[worksheet("Data", r#"<row r="5"/>"#)]));
    let output = run(&["read", path.to_str().unwrap()]);
    assert_eq!(stdout(&output), "5\n");
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(lines(&output)[0], json!({ "row": 5, "cells": [] }));
}

#[test]
fn the_help_of_read_gives_the_json_contract() {
    let output = run(&["read", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = stdout(&output);
    assert!(help.contains("Usage: xlsx-graft read [OPTIONS] <PATH>"));
    assert!(help.contains("--sheet <NAME>"));
    assert!(help.contains("--sheet-index <N>"));
    assert!(help.contains("JSON Lines"));
    assert!(help.contains("sheet_not_worksheet"));
}
