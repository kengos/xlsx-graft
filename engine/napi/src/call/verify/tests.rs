//! Unit tests for the verify calls: the result's JSON as the value, and a
//! status that is not OK without one.

use super::{check_policy, verify_bytes, verify_path};
use xlsx_graft_capi::{XLSX_GRAFT_ERR_INVALID_ARG, XLSX_GRAFT_OK};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

#[test]
fn a_workbook_at_a_path_is_verified() {
    let outcome = verify_path(&format!("{FIXTURES}/macro.xlsm"), "{}");
    assert_eq!((outcome.status, outcome.message), (XLSX_GRAFT_OK, None));
    let json = outcome.value.unwrap();
    assert!(
        json.starts_with(r#"{"ok":false,"failOn":"error""#),
        "{json}"
    );
    assert!(json.contains(r#""code":"part_denied""#), "{json}");
}

#[test]
fn a_file_that_cannot_be_opened_is_the_results_error_not_a_status() {
    let outcome = verify_path("/no/such/book.xlsx", "");
    assert_eq!(outcome.status, XLSX_GRAFT_OK);
    let json = outcome.value.unwrap();
    assert!(json.contains(r#""error":{"code":"io""#), "{json}");
}

#[test]
fn a_workbook_in_bytes_is_verified() {
    let data = std::fs::read(format!("{FIXTURES}/table.xlsx")).unwrap();
    let outcome = verify_bytes(&data, r#"{"rules":{"failOn":"warn"}}"#);
    let json = outcome.value.unwrap();
    assert!(json.starts_with(r#"{"ok":true,"failOn":"warn""#), "{json}");
}

#[test]
fn a_request_that_is_not_an_object_is_refused_without_a_value() {
    let refused = [
        verify_path("/no/such/book.xlsx", "[]"),
        verify_bytes(b"", "{\"x\":1}"),
    ];
    for outcome in refused {
        assert_eq!(outcome.status, XLSX_GRAFT_ERR_INVALID_ARG);
        assert_eq!((outcome.value, outcome.message), (None, None));
    }
}

#[test]
fn a_policy_is_checked_alone() {
    let valid = check_policy("").value.unwrap();
    assert!(valid.starts_with(r#"{"ok":true"#), "{valid}");
    let invalid = check_policy(r#"{"maxEntries":0}"#).value.unwrap();
    assert!(invalid.contains(r#""code":"policy_invalid""#), "{invalid}");
}
