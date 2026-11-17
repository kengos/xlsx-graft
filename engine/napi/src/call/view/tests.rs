//! Unit tests for the view calls: the result's JSON as the value, the
//! message of keys that `SheetRequest` refuses, and a status that is not OK
//! without a value.

use super::{describe_sheet_bytes, describe_sheet_path, tree_bytes, tree_path};
use xlsx_graft_capi::{XLSX_GRAFT_ERR_INVALID_ARG, XLSX_GRAFT_ERR_REQUEST_INVALID, XLSX_GRAFT_OK};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

#[test]
fn the_tree_of_a_workbook_is_viewed_from_a_path_and_from_bytes() {
    let path = format!("{FIXTURES}/table.xlsx");
    let by_path = tree_path(&path, "{}");
    assert_eq!(
        (by_path.status, by_path.message.clone()),
        (XLSX_GRAFT_OK, None)
    );
    let json = by_path.value.clone().unwrap();
    assert!(json.starts_with(r#"{"ok":true,"error":null"#), "{json}");
    let data = std::fs::read(&path).unwrap();
    assert_eq!(tree_bytes(&data, "").value, by_path.value);
}

#[test]
fn the_cells_of_a_sheet_are_viewed_from_a_path_and_from_bytes() {
    let path = format!("{FIXTURES}/merged.xlsx");
    let by_path = describe_sheet_path(&path, r#"{"range":"A1:C3"}"#);
    assert_eq!(
        (by_path.status, by_path.message.clone()),
        (XLSX_GRAFT_OK, None)
    );
    let json = by_path.value.clone().unwrap();
    assert!(json.contains(r#""shown":"A1:C3""#), "{json}");
    let data = std::fs::read(&path).unwrap();
    let by_bytes = describe_sheet_bytes(&data, r#"{"range":"A1:C3"}"#);
    assert_eq!(by_bytes.value, by_path.value);
}

#[test]
fn a_file_that_cannot_be_opened_is_the_results_error_not_a_status() {
    let outcome = tree_path("/no/such/book.xlsx", "");
    assert_eq!(outcome.status, XLSX_GRAFT_OK);
    let json = outcome.value.unwrap();
    assert!(json.contains(r#""error":{"code":"io""#), "{json}");
}

#[test]
fn options_that_are_not_a_json_object_of_known_keys_are_invalid_arg_without_a_value_or_a_message() {
    let refused = [
        tree_path("/no/such/book.xlsx", "[]"),
        tree_bytes(b"", r#"{"sheet":"a"}"#),
        describe_sheet_path("/no/such/book.xlsx", "{"),
        describe_sheet_bytes(b"", r#"{"x":1}"#),
    ];
    for outcome in refused {
        assert_eq!(outcome.status, XLSX_GRAFT_ERR_INVALID_ARG);
        assert_eq!((outcome.value, outcome.message), (None, None));
    }
}

#[test]
fn keys_that_from_arguments_refuses_return_request_invalid_with_the_reason_as_the_message() {
    let refused = [
        describe_sheet_path("/no/such/book.xlsx", r#"{"maxRows":0}"#),
        describe_sheet_bytes(b"", r#"{"maxRows":0}"#),
    ];
    for outcome in refused {
        assert_eq!(outcome.status, XLSX_GRAFT_ERR_REQUEST_INVALID);
        assert_eq!(outcome.value, None);
        assert_eq!(
            outcome.message.as_deref(),
            Some("maxRows 0 is not between 1 and 10000")
        );
    }
}
