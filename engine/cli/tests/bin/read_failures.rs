//! `read` when it does not read a sheet to its end: a sheet it cannot
//! choose or read, a problem in the sheet, a workbook it cannot open, a
//! failing host, and a stdout closed early.

use crate::workbook::{CHARTSHEET, Sheet, unrelated, worksheet, write, xlsx};
use crate::{run, stderr, stdout, xlsx_graft};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::process::Stdio;

/// The last line of a JSON Lines `stdout`, and how many came before it.
fn end(output: &std::process::Output) -> (Value, usize) {
    let lines: Vec<&str> = stdout(output).lines().collect();
    let (last, before) = lines.split_last().unwrap();
    (serde_json::from_str(last).unwrap(), before.len())
}

/// The codes of the diagnostics of `end`.
fn codes(end: &Value) -> Vec<&str> {
    let diagnostics = end["diagnostics"].as_array().unwrap();
    (diagnostics.iter())
        .map(|record| record["code"].as_str().unwrap())
        .collect()
}

#[test]
fn a_sheet_not_chosen_ends_in_its_error_alone_and_exits_1() {
    let sheets = [worksheet("Data", ""), worksheet("data", "")];
    let (_dir, path) = write(&xlsx(&sheets));
    let path = path.to_str().unwrap();
    let cases = [
        (
            &["--sheet", "none"][..],
            "sheet_not_found",
            "no sheet is named \"none\"",
        ),
        (
            &["--sheet-index", "2"],
            "sheet_not_found",
            "there is no sheet at index 2: the workbook has 2, from index 0",
        ),
        (
            &["--sheet", "DATA"],
            "sheet_ambiguous",
            "more than one sheet is named \"DATA\", letter case aside",
        ),
    ];
    for (choice, code, message) in cases {
        let mut args = vec!["read", path, "--format", "json"];
        args.extend(choice);
        let output = run(&args);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stderr(&output), "");
        let (end, before) = end(&output);
        assert_eq!(before, 0);
        assert_eq!(end["ok"], false);
        assert_eq!(end["sheet"], Value::Null);
        assert_eq!(end["rows"], 0);
        assert_eq!(end["error"], json!({ "code": code, "message": message }));
        // The workbook's own problem is there whatever is asked.
        assert_eq!(codes(&end), ["sheet_name_duplicate"]);
    }
}

#[test]
fn a_sheet_that_is_no_worksheet_is_not_read() {
    let chart = Sheet {
        name: "Chart",
        rel_type: Some(CHARTSHEET),
        xml: "<chartsheet/>".to_owned(),
    };
    let (_dir, path) = write(&xlsx(&[chart]));
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let (end, _) = end(&output);
    assert_eq!(end["ok"], false);
    assert_eq!(end["sheet"], json!({ "index": 0, "name": "Chart" }));
    assert_eq!(end["error"]["code"], "sheet_not_worksheet");
    assert_eq!(end["diagnostics"], json!([]));
    let output = run(&["read", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "xlsx-graft: sheet \"Chart\" is a chartsheet, which holds no cells\n"
    );
}

#[test]
fn a_sheet_whose_part_is_not_known_is_chosen_but_not_read() {
    let (_dir, path) = write(&xlsx(&[unrelated("Lost")]));
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let (end, before) = end(&output);
    assert_eq!(before, 0);
    assert_eq!(end["sheet"], json!({ "index": 0, "name": "Lost" }));
    assert_eq!(end["error"]["code"], "sheet_unreadable");
    assert_eq!(codes(&end), ["sheet_relationship_missing"]);
}

#[test]
fn a_problem_in_the_sheet_ends_it_after_the_rows_before() {
    let rows = concat!(
        r#"<row r="1"><c t="s"><v>1</v></c></row>"#,
        r#"<row r="2"><c t="s"><v>9</v></c></row>"#,
        r#"<row r="3"><c t="s"><v>0</v></c></row>"#,
    );
    let (_dir, path) = write(&xlsx(&[worksheet("Data", rows)]));
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "");
    let (end, before) = end(&output);
    assert_eq!(before, 1);
    assert_eq!(end["ok"], false);
    assert_eq!(end["rows"], 1);
    assert_eq!(end["error"], Value::Null);
    assert_eq!(codes(&end), ["shared_string_missing"]);
    assert_eq!(end["diagnostics"][0]["args"], json!({ "index": 9 }));
    let location = &end["diagnostics"][0]["location"];
    assert_eq!(location, &json!({ "part": "/xl/sheets/sheet1.xml" }));
    // In text, the problem goes to stderr.
    let output = run(&["read", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "1\tb\n");
    assert_eq!(
        stderr(&output),
        concat!(
            "xlsx-graft: error shared_string_missing in /xl/sheets/sheet1.xml: ",
            "a cell refers to shared string 9, which the shared strings table does not hold\n",
        )
    );
}

#[test]
fn a_file_that_is_no_workbook_is_unreadable() {
    let (_dir, path) = write(b"not a zip");
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let (end, before) = end(&output);
    assert_eq!(before, 0);
    assert_eq!(end["ok"], false);
    assert_eq!(end["sheet"], Value::Null);
    assert_eq!(end["error"]["code"], "workbook_unreadable");
    assert_eq!(codes(&end).len(), 1);
}

#[test]
fn a_file_that_will_not_open_fails_on_the_host() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.xlsx");
    let output = run(&["read", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    let reason = format!("xlsx-graft: failed to open {}: ", path.display());
    assert!(stderr(&output).starts_with(&reason));
}

// A FIFO opens but does not seek, as a package is read.
#[cfg(unix)]
#[test]
fn a_file_that_will_not_seek_fails_on_the_host() {
    use std::fs::OpenOptions;
    use std::process::Command;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fifo.xlsx");
    assert!(
        Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let child = xlsx_graft()
        .args(["read", path.to_str().unwrap(), "--format", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The run's open returns once the FIFO has a writer; closing it at once
    // leaves the run nothing to wait for.
    drop(OpenOptions::new().write(true).open(&path).unwrap());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    let reason = "xlsx-graft: failed to read the workbook: reading the package failed: ";
    assert!(stderr(&output).starts_with(reason));
}

#[test]
fn a_stdout_closed_early_stops_the_reading_and_exits_0() {
    // Far more than a pipe holds, so the run is still writing when the
    // reader goes.
    let row = r#"<row><c t="inlineStr"><is><t>a long enough value</t></is></c></row>"#;
    let (_dir, path) = write(&xlsx(&[worksheet("Data", &row.repeat(50_000))]));
    let mut child = xlsx_graft()
        .args(["read", path.to_str().unwrap(), "--format", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout).read_line(&mut first).unwrap();
    assert!(first.starts_with(r#"{"row":1,"#));
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
}

// /dev/full refuses every write with ENOSPC. The gates run in the Linux
// toolchain image; macOS has no such device.
#[cfg(target_os = "linux")]
#[test]
fn a_full_stdout_fails_the_read() {
    use std::fs::OpenOptions;

    let (_dir, path) = write(&xlsx(&[worksheet("Data", "<row/>")]));
    for format in ["text", "json"] {
        let full = OpenOptions::new().write(true).open("/dev/full").unwrap();
        let output = xlsx_graft()
            .args(["read", path.to_str().unwrap(), "--format", format])
            .stdout(full)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).starts_with("xlsx-graft: failed to write the output: "));
    }
}
