//! `describe-sheet` when something goes wrong: a request the host does not
//! take (exit 2, the reason on stderr and no view), and a workbook or a sheet
//! it cannot read (exit 1 and a view with an `error`).

use crate::workbook::{CHARTSHEET, Sheet, worksheet, write, xlsx};
use crate::{fixture, printed, run, stderr, stdout};
use serde_json::Value;

#[test]
fn a_sheet_that_is_not_there_is_an_error_of_the_view_with_no_sheet() {
    let book = fixture("merged.xlsx");
    for choice in [["--sheet", "Nope"], ["--sheet-index", "9"]] {
        let output = run(&[
            "describe-sheet",
            &book,
            choice[0],
            choice[1],
            "--format",
            "json",
        ]);
        assert_eq!(output.status.code(), Some(1), "{choice:?}");
        let view = printed(&output);
        assert_eq!(view["ok"], false);
        assert_eq!(view["error"]["code"], "sheet_not_found");
        assert_eq!(view["sheet"], Value::Null);
        assert_eq!(stderr(&output), "");
    }
    let output = run(&["describe-sheet", &book, "--sheet", "Nope"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).starts_with("could not read: "));
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_chartsheet_is_not_a_worksheet() {
    let chart = Sheet {
        name: "Chart",
        rel_type: Some(CHARTSHEET),
        xml: "<chartsheet/>".to_owned(),
        state: None,
    };
    let (_dir, path) = write(&xlsx(&[worksheet("Data", ""), chart]));
    let output = run(&[
        "describe-sheet",
        path.to_str().unwrap(),
        "--sheet-index",
        "1",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let view = printed(&output);
    assert_eq!(view["error"]["code"], "sheet_not_worksheet");
    assert_eq!(view["sheet"]["kind"], "chartsheet");
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_sheet_by_name_and_by_index_is_a_usage_error_with_no_output() {
    let output = run(&[
        "describe-sheet",
        &fixture("merged.xlsx"),
        "--sheet",
        "a",
        "--sheet-index",
        "0",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("cannot be used with"));
}

#[test]
fn a_request_the_host_does_not_take_is_exit_2_with_the_reason_on_stderr() {
    let book = fixture("merged.xlsx");
    let cases = [
        (
            ["--range", "A0"],
            r#"xlsx-graft: --range "A0" is not a range of cells such as A1:H50"#,
        ),
        (
            ["--range", "XFE1"],
            r#"xlsx-graft: --range "XFE1" is not a range of cells such as A1:H50"#,
        ),
        (
            ["--max-rows", "0"],
            "xlsx-graft: --max-rows 0 is not between 1 and 10000\n",
        ),
        (
            ["--max-rows", "10001"],
            "xlsx-graft: --max-rows 10001 is not between 1 and 10000\n",
        ),
        (
            ["--max-columns", "0"],
            "xlsx-graft: --max-columns 0 is not between 1 and 1000\n",
        ),
        (
            ["--max-columns", "1001"],
            "xlsx-graft: --max-columns 1001 is not between 1 and 1000\n",
        ),
    ];
    for (args, reason) in cases {
        for format in ["text", "json"] {
            let output = run(&[
                "describe-sheet",
                &book,
                args[0],
                args[1],
                "--format",
                format,
            ]);
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            assert_eq!(stdout(&output), "", "{args:?}");
            assert!(
                stderr(&output).starts_with(reason),
                "{args:?} {}",
                stderr(&output)
            );
        }
    }
}

#[test]
fn a_negative_sheet_index_is_clap_s_usage_error() {
    let output = run(&[
        "describe-sheet",
        &fixture("merged.xlsx"),
        "--sheet-index",
        "-1",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(!stderr(&output).is_empty());
}

#[test]
fn a_max_rows_that_is_not_a_number_is_clap_s_usage_error() {
    let output = run(&[
        "describe-sheet",
        &fixture("merged.xlsx"),
        "--max-rows",
        "abc",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("invalid value 'abc'"));
}

#[test]
fn a_missing_file_is_an_io_error_and_a_file_that_is_no_zip_is_unreadable() {
    let (dir, path) = write(b"not a zip");
    let missing = dir.path().join("missing.xlsx");
    let output = run(&[
        "describe-sheet",
        missing.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let view = printed(&output);
    assert_eq!(view["error"]["code"], "io");
    assert_eq!(view["sheet"], Value::Null);
    assert_eq!(stderr(&output), "");
    let output = run(&["describe-sheet", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let view = printed(&output);
    assert_eq!(view["error"]["code"], "workbook_unreadable");
    assert_eq!(view["sheet"], Value::Null);
    assert_eq!(stderr(&output), "");
}
