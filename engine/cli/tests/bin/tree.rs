//! `tree` on the workbooks in `fixtures/`: the view in each format, the
//! exit status of each way it ends, and the help. The JSON cases pin the
//! contract: the keys of the view, of one relationship and of one part are
//! looked for in the order they are printed (from the first relationship and
//! the first part on), and the first part, the first relationship and its
//! target have exactly 9, 3 and 1 keys, with the values `table.xlsx` gives.

use crate::workbook::write;
use crate::{fixture, printed, run, stderr, stdout};
use serde_json::Value;

/// Asserts that `text` holds each of `keys` as a JSON key, in that order,
/// from the first `marker` on. A parsed `serde_json::Value` does not keep the
/// order of an object's keys, so the order printed is read from the text.
fn assert_keys_in_order(text: &str, marker: &str, keys: &[&str]) {
    let mut rest = &text[text.find(marker).unwrap()..];
    for key in keys {
        let found = rest.find(&format!("\"{key}\":"));
        assert!(found.is_some(), "{key} not found, in order, in {rest}");
        rest = &rest[found.unwrap() + key.len()..];
    }
}

/// Every `target` of every relationship in `view`, the package's and each
/// part's.
fn targets(view: &Value) -> Vec<&Value> {
    let package = view["relationships"].as_array().unwrap().iter();
    let parts = (view["parts"].as_array().unwrap().iter())
        .flat_map(|part| part["relationships"].as_array().unwrap());
    package.chain(parts).map(|rel| &rel["target"]).collect()
}

#[test]
fn json_is_one_line_of_the_view_with_the_package_relationships() {
    let output = run(&["tree", &fixture("table.xlsx"), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let view = printed(&output);
    assert_eq!(view.as_object().unwrap().len(), 7);
    let text = stdout(&output);
    let top = [
        "ok",
        "error",
        "diagnostics",
        "complete",
        "relationshipsKnown",
        "relationships",
        "parts",
    ];
    assert_keys_in_order(text, "{", &top);
    assert_keys_in_order(text, r#""relationships":[{"#, &["id", "type", "target"]);
    let part = [
        "name",
        "contentType",
        "contentTypeKnown",
        "size",
        "compressedSize",
        "readable",
        "referenced",
        "relationshipsKnown",
        "relationships",
    ];
    assert_keys_in_order(text, r#""parts":[{"#, &part);
    // The order says the keys are there; the counts say there are no others,
    // in the first part, the first relationship and its target.
    let count = |value: &Value| value.as_object().unwrap().len();
    assert_eq!(count(&view["parts"][0]), part.len());
    assert_eq!(count(&view["relationships"][0]), 3);
    assert_eq!(count(&view["relationships"][0]["target"]), 1);
    assert_eq!(view["ok"], true);
    assert_eq!(view["complete"], true);
    let office = (view["relationships"].as_array().unwrap().iter())
        .find(|rel| rel["type"].as_str().unwrap().ends_with("/officeDocument"))
        .unwrap();
    assert_eq!(office["target"]["part"], "/xl/workbook.xml");
    let workbook = (view["parts"].as_array().unwrap().iter())
        .find(|part| part["name"] == "/xl/workbook.xml")
        .unwrap();
    assert_eq!(workbook["referenced"], true);
    assert_eq!(workbook["readable"], true);
}

#[test]
fn an_external_target_is_written_as_external() {
    let output = run(&["tree", &fixture("external-link.xlsx"), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let view = printed(&output);
    let external = targets(&view)
        .into_iter()
        .any(|target| target.get("external").is_some());
    assert!(external, "{view}");
}

#[test]
fn text_is_a_tree_from_the_package_and_ends_with_the_counts() {
    let output = run(&["tree", &fixture("table.xlsx")]);
    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    assert_eq!(text.lines().next(), Some("/ (package)"));
    assert!(text.contains("officeDocument"), "{text}");
    assert!(text.ends_with(" — ok\n"), "{text}");
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_missing_file_and_a_directory_are_an_io_error_with_no_part() {
    let (dir, _path) = write(b"x");
    let missing = dir.path().join("missing.xlsx");
    for target in [missing.to_str().unwrap(), dir.path().to_str().unwrap()] {
        let output = run(&["tree", target, "--format", "json"]);
        assert_eq!(output.status.code(), Some(1), "{target}");
        let view = printed(&output);
        assert_eq!(view["ok"], false);
        assert_eq!(view["error"]["code"], "io");
        assert_eq!(view["parts"], serde_json::json!([]));
        assert_eq!(stderr(&output), "");
        let output = run(&["tree", target]);
        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).starts_with("could not read: "));
        assert_eq!(stderr(&output), "");
    }
}

#[test]
fn a_file_that_is_no_zip_is_a_diagnostic_and_no_part() {
    let (_dir, path) = write(b"not a zip");
    let output = run(&["tree", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let view = printed(&output);
    assert_eq!(view["ok"], false);
    assert_eq!(view["error"], Value::Null);
    assert_eq!(view["complete"], false);
    assert_eq!(view["parts"], serde_json::json!([]));
    assert!(!view["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(stderr(&output), "");
    let output = run(&["tree", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).ends_with(" — not ok\n"),
        "{}",
        stdout(&output)
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn the_help_of_tree_gives_the_json_contract_and_the_exit_status() {
    let output = run(&["tree", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = stdout(&output);
    assert!(help.contains("Usage: xlsx-graft tree [OPTIONS] <PATH>"));
    assert!(help.contains(r#""relationshipsKnown": BOOL"#));
    assert!(help.contains(r#"{"external": TEXT}"#));
    assert!(help.contains("tidied for display"));
    assert!(help.contains("Exit status: 0 when ok."));
}
