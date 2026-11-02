//! `sheets`: the list in each format, every kind and state, a workbook it
//! cannot open, and the help. The JSON cases pin the contract: whole lines
//! are compared, so a renamed, added or retyped key fails here.

use crate::workbook::{
    CHARTSHEET, DIALOGSHEET, MACROSHEET, Sheet, unrelated, worksheet, write, xlsx,
};
use crate::{run, stderr, stdout};
use serde_json::{Value, json};

/// A worksheet, a very hidden chartsheet, a hidden sheet whose part is
/// not known, a worksheet whose name holds a tab, a dialogsheet and a
/// macro sheet.
fn sheets() -> Vec<Sheet<'static>> {
    let other = |name, rel_type| Sheet {
        name,
        rel_type: Some(rel_type),
        xml: "<x/>".to_owned(),
        state: None,
    };
    let chart = Sheet {
        name: "Chart",
        rel_type: Some(CHARTSHEET),
        xml: "<chartsheet/>".to_owned(),
        state: Some("veryHidden"),
    };
    let lost = Sheet {
        state: Some("hidden"),
        ..unrelated("Lost")
    };
    vec![
        worksheet("Data", ""),
        chart,
        lost,
        worksheet("in&#9;tab", ""),
        other("Dialog", DIALOGSHEET),
        other("Macro", MACROSHEET),
    ]
}

#[test]
fn json_prints_each_sheet_with_its_kind_and_state() {
    let (_dir, path) = write(&xlsx(&sheets()));
    let output = run(&["sheets", path.to_str().unwrap(), "--format", "json"]);
    let sheet = |index: usize, name: &str, kind: &str, state: &str| json!({ "index": index, "name": name, "kind": kind, "state": state });
    let listing: Value = serde_json::from_str(stdout(&output)).unwrap();
    assert_eq!(
        listing["sheets"],
        json!([
            sheet(0, "Data", "worksheet", "visible"),
            sheet(1, "Chart", "chartsheet", "veryHidden"),
            sheet(2, "Lost", "unknown", "hidden"),
            sheet(3, "in\ttab", "worksheet", "visible"),
            sheet(4, "Dialog", "dialogsheet", "visible"),
            sheet(5, "Macro", "macrosheet", "visible"),
        ])
    );
    // The sheet whose part is not known is the workbook's error.
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(listing["ok"], false);
    assert_eq!(listing["error"], Value::Null);
    let diagnostics = listing["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], "sheet_relationship_missing");
    assert_eq!(stdout(&output).lines().count(), 1);
    assert_eq!(stderr(&output), "");
}

#[test]
fn json_of_a_workbook_with_no_problem_is_ok() {
    let (_dir, path) = write(&xlsx(&[worksheet("Data", "")]));
    let output = run(&["--format=json", "sheets", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        concat!(
            r#"{"ok":true,"sheets":[{"index":0,"name":"Data","kind":"worksheet","#,
            r#""state":"visible"}],"error":null,"diagnostics":[]}"#,
            "\n",
        )
    );
}

#[test]
fn text_prints_a_tab_separated_line_a_sheet_and_the_problems_on_stderr() {
    let (_dir, path) = write(&xlsx(&sheets()));
    let output = run(&["sheets", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout(&output),
        concat!(
            "0\tData\tworksheet\tvisible\n",
            "1\tChart\tchartsheet\tveryHidden\n",
            "2\tLost\tunknown\thidden\n",
            "3\tin\\ttab\tworksheet\tvisible\n",
            "4\tDialog\tdialogsheet\tvisible\n",
            "5\tMacro\tmacrosheet\tvisible\n",
        )
    );
    assert!(stderr(&output).starts_with("xlsx-graft: error sheet_relationship_missing "));
    assert_eq!(stderr(&output).lines().count(), 1);
}

#[test]
fn text_of_a_workbook_with_no_problem_says_nothing_on_stderr() {
    let (_dir, path) = write(&xlsx(&[worksheet("Data", "")]));
    let output = run(&["sheets", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "0\tData\tworksheet\tvisible\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_file_that_is_no_workbook_lists_no_sheet() {
    let (_dir, path) = write(b"not a zip");
    let output = run(&["sheets", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let listing: Value = serde_json::from_str(stdout(&output)).unwrap();
    assert_eq!(listing["ok"], false);
    assert_eq!(listing["sheets"], json!([]));
    assert_eq!(listing["error"]["code"], "workbook_unreadable");
    assert_eq!(listing["diagnostics"].as_array().unwrap().len(), 1);
    let output = run(&["sheets", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    let lines: Vec<&str> = stderr(&output).lines().collect();
    assert_eq!(lines.len(), 2);
    let refusal = xlsx_graft_read::Refusal::WorkbookUnreadable;
    assert_eq!(lines[1], format!("xlsx-graft: {refusal}"));
}

#[test]
fn a_file_that_will_not_open_fails_on_the_host() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.xlsx");
    let output = run(&["sheets", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    let reason = format!("xlsx-graft: failed to open {}: ", path.display());
    assert!(stderr(&output).starts_with(&reason));
}

// /dev/full refuses every write with ENOSPC. The gates run in the Linux
// toolchain image; macOS has no such device.
#[cfg(target_os = "linux")]
#[test]
fn a_full_stdout_fails_the_listing() {
    use std::fs::OpenOptions;

    let (_dir, path) = write(&xlsx(&[worksheet("Data", "")]));
    for format in ["text", "json"] {
        let full = OpenOptions::new().write(true).open("/dev/full").unwrap();
        let output = crate::xlsx_graft()
            .args(["sheets", path.to_str().unwrap(), "--format", format])
            .stdout(full)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).starts_with("xlsx-graft: failed to write the output: "));
    }
}

#[test]
fn the_help_of_sheets_gives_the_json_contract() {
    let output = run(&["sheets", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = stdout(&output);
    assert!(help.contains("Usage: xlsx-graft sheets [OPTIONS] <PATH>"));
    assert!(help.contains("\"kind\": KIND"));
    assert!(help.contains("veryHidden"));
}
