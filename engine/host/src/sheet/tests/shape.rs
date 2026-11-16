//! The JSON of a grid: its keys and their order, each type of cell, the
//! cells that are not listed, `dimension` and the text of a cell.

use super::{asking, described, rows_only, single, view, worksheet};
use serde_json::{Value, json};

#[test]
fn the_json_of_a_grid_is_one_line_with_the_keys_of_the_contract_in_order() {
    let rows = concat!(
        r#"<row r="1"><c r="A1" t="s"><v>0</v></c><c r="C1"><v>2.5</v></c></row>"#,
        r#"<row r="2"><c r="B2" t="b"><v>1</v></c></row>"#,
    );
    let merges = r#"<mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells>"#;
    let bytes = single(&worksheet(r#"<dimension ref="A1:C2"/>"#, rows, merges));
    let line = view(&bytes, &asking(None, None, None)).to_json().unwrap();
    let expected = concat!(
        r#"{"ok":true,"error":null,"diagnostics":[],"#,
        r#""sheet":{"index":0,"name":"Sheet1","kind":"worksheet","state":"visible"},"#,
        r#""dimension":"A1:C2","used":"A1:C2","shown":"A1:C2","#,
        r#""truncated":{"rows":false,"columns":false,"merges":false},"complete":true,"#,
        r#""rows":[{"row":1,"cells":[{"ref":"A1","type":"string","value":"shared"},"#,
        r#"{"ref":"C1","type":"number","value":2.5}]},"#,
        r#"{"row":2,"cells":[{"ref":"B2","type":"boolean","value":true}]}],"#,
        r#""merges":["A1:B1"]}"#,
    );
    assert_eq!(line, expected);
}

#[test]
fn each_type_of_value_is_listed_with_its_type() {
    let cells = concat!(
        r#"<c r="A1" t="s"><v>0</v></c>"#,
        r#"<c r="B1"><v>2.5</v></c>"#,
        r#"<c r="C1" t="b"><v>0</v></c>"#,
        r#"<c r="D1" s="1"><v>45306</v></c>"#,
        r#"<c r="E1" s="2"><v>0.5</v></c>"#,
        r#"<c r="F1" s="3"><v>45306.354166666664</v></c>"#,
        r#"<c r="G1" s="4"><v>1.0854166666666667</v></c>"#,
        r#"<c r="H1" t="e"><v>#N/A</v></c>"#,
        r#"<c r="I1" t="inlineStr"><is><t>inline</t></is></c>"#,
        r#"<c r="J1" t="d"><v>2024-01-15</v></c>"#,
    );
    let value = described(
        &single(&rows_only(&format!("<row>{cells}</row>"))),
        &asking(None, None, None),
    );
    let listed: Vec<(&str, &Value, &Value)> = value["rows"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cell| (cell["ref"].as_str().unwrap(), &cell["type"], &cell["value"]))
        .collect();
    let expected = [
        ("A1", json!("string"), json!("shared")),
        ("B1", json!("number"), json!(2.5)),
        ("C1", json!("boolean"), json!(false)),
        ("D1", json!("date"), json!("2024-01-15")),
        ("E1", json!("time"), json!("12:00:00.000")),
        ("F1", json!("datetime"), json!("2024-01-15T08:30:00.000")),
        ("G1", json!("elapsed"), json!(93_780_000)),
        ("H1", json!("error"), json!("#N/A")),
        ("I1", json!("string"), json!("inline")),
        ("J1", json!("date"), json!("2024-01-15")),
    ];
    assert_eq!(listed.len(), expected.len());
    for ((reference, kind, written), (want_ref, want_kind, want)) in listed.iter().zip(&expected) {
        assert_eq!(
            (reference, *kind, *written),
            (want_ref, want_kind, want),
            "{reference}"
        );
    }
    assert_eq!(value["used"], "A1:J1");
}

#[test]
fn a_cell_with_no_value_is_not_listed_and_a_row_of_none_is_not_a_row() {
    let rows = concat!(
        r#"<row r="1"><c r="A1"/><c r="B1"><v>1</v></c><c r="C1" t="s"/></row>"#,
        r#"<row r="2"><c r="A2"/></row>"#,
        r#"<row r="3"/>"#,
        r#"<row r="4"><c r="A4"><v>4</v></c></row>"#,
    );
    let value = described(&single(&rows_only(rows)), &asking(None, None, None));
    assert_eq!(
        value["rows"],
        json!([
            {"row": 1, "cells": [{"ref": "B1", "type": "number", "value": 1.0}]},
            {"row": 4, "cells": [{"ref": "A4", "type": "number", "value": 4.0}]},
        ])
    );
    // The empty cells do not count in the used range either.
    assert_eq!(value["used"], "A1:B4");
    assert_eq!(value["shown"], "A1:B4");
}

#[test]
fn the_dimension_is_as_written_or_null_and_is_not_the_used_range() {
    let rows = r#"<row r="2"><c r="B2"><v>1</v></c></row>"#;
    let none = described(&single(&rows_only(rows)), &asking(None, None, None));
    assert_eq!(none.get("dimension"), Some(&Value::Null));
    let wrong = worksheet(r#"<dimension ref="A1:Z99"/>"#, rows, "");
    let value = described(&single(&wrong), &asking(None, None, None));
    assert_eq!(
        (&value["dimension"], &value["used"]),
        (&json!("A1:Z99"), &json!("B2:B2"))
    );
    // Not checked to be a range.
    let odd = worksheet(r#"<dimension ref="not a range"/>"#, rows, "");
    let value = described(&single(&odd), &asking(None, None, None));
    assert_eq!(value["dimension"], "not a range");
}

#[test]
fn the_dimension_is_written_as_an_echo_does() {
    let long = "A".repeat(300);
    let head = format!(r#"<dimension ref="A1&#x202E;:C3&#x85;{long}"/>"#);
    let bytes = single(&worksheet(
        &head,
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        "",
    ));
    let value = described(&bytes, &asking(None, None, None));
    let written = value["dimension"].as_str().unwrap();
    assert!(written.starts_with("A1:C3AAA"), "{written}");
    assert!(written.ends_with('…'));
    assert_eq!(written.chars().count(), 201);
}

#[test]
fn the_text_of_a_cell_keeps_tab_line_feed_and_return_and_loses_the_other_control_characters() {
    let cells = concat!(
        "<c r=\"A1\" t=\"inlineStr\"><is><t>a\tb&#13;c\nd&#x202E;e</t></is></c>",
        r#"<c r="B1" t="str"><v>x</v></c>"#,
    );
    let value = described(
        &single(&rows_only(&format!("<row>{cells}</row>"))),
        &asking(None, None, None),
    );
    let first = &value["rows"][0]["cells"][0];
    assert_eq!(first["value"], "a\tb\rc\nde");
    assert!(first.get("truncated").is_none());
}

#[test]
fn text_past_a_thousand_characters_is_cut_and_says_so_and_a_whole_text_does_not() {
    let exact = "e".repeat(1_000);
    let long = "l".repeat(1_001);
    let cells = format!(
        concat!(
            r#"<c r="A1" t="inlineStr"><is><t>{exact}</t></is></c>"#,
            r#"<c r="B1" t="inlineStr"><is><t>{long}</t></is></c>"#,
            r#"<c r="C1" t="e"><v>#{long}</v></c>"#,
        ),
        exact = exact,
        long = long
    );
    let value = described(
        &single(&rows_only(&format!("<row>{cells}</row>"))),
        &asking(None, None, None),
    );
    let listed = &value["rows"][0]["cells"];
    assert_eq!(listed[0]["value"], exact);
    assert!(listed[0].get("truncated").is_none());
    assert_eq!(listed[1]["value"], long[..1_000]);
    assert_eq!(listed[1]["truncated"], json!(true));
    assert_eq!(listed[2]["type"], "error");
    assert_eq!(listed[2]["truncated"], json!(true));
    assert_eq!(listed[2]["value"].as_str().unwrap().chars().count(), 1_000);
}

#[test]
fn a_sheet_with_no_cell_has_no_used_range_and_nothing_shown() {
    let value = described(&single(&rows_only("")), &asking(None, None, None));
    assert_eq!(value["ok"], true);
    assert_eq!(value["used"], Value::Null);
    assert_eq!(value["shown"], Value::Null);
    assert_eq!(value["rows"], json!([]));
    assert_eq!(value["merges"], json!([]));
    assert_eq!(value["complete"], true);
    assert_eq!(
        value["truncated"],
        json!({"rows": false, "columns": false, "merges": false})
    );
}
