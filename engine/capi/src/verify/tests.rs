//! Unit tests for the verify handle's logic: the result's JSON for a
//! workbook from a path and from bytes, a request that is refused, and the
//! faults of a policy.

use super::XlsxGraftVerify;
use crate::status::XLSX_GRAFT_ERR_INVALID_ARG;
use serde_json::Value;

/// The fixture workbooks.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

/// The result of `verify` as a JSON value.
fn json(verify: Result<XlsxGraftVerify, i32>) -> Value {
    serde_json::from_str(verify.expect("a result").json()).expect("JSON")
}

/// The codes of the result's diagnostics.
fn codes(result: &Value) -> Vec<&str> {
    let list = result["diagnostics"].as_array().expect("a list");
    list.iter()
        .map(|d| d["code"].as_str().expect("a code"))
        .collect()
}

#[test]
fn a_workbook_at_a_path_is_verified() {
    let path = format!("{FIXTURES}/macro.xlsm");
    let result = json(XlsxGraftVerify::path(&path, b""));
    assert_eq!(result["ok"], false);
    assert_eq!(result["error"], Value::Null);
    assert!(codes(&result).contains(&"part_denied"));
}

#[test]
fn a_file_that_cannot_be_opened_is_the_results_io_error() {
    let result = json(XlsxGraftVerify::path("/no/such/book.xlsx", b""));
    assert_eq!(result["ok"], false);
    assert_eq!(result["error"]["code"], "io");
}

#[test]
fn a_workbook_in_bytes_is_verified_in_place() {
    let data = std::fs::read(format!("{FIXTURES}/table.xlsx")).expect("table.xlsx");
    let result = json(XlsxGraftVerify::bytes(&data, b"{}"));
    assert_eq!(
        (result["ok"].as_bool(), codes(&result)),
        (Some(true), vec![])
    );
}

#[test]
fn bytes_that_are_no_workbook_are_a_diagnostic_not_an_error() {
    let result = json(XlsxGraftVerify::bytes(b"not a zip", b"null"));
    assert_eq!(result["ok"], false);
    assert_eq!(result["error"], Value::Null);
    assert!(!codes(&result).is_empty());
}

#[test]
fn a_request_that_is_not_an_object_of_policy_and_rules_is_invalid() {
    let requests: [&[u8]; 4] = [b"[]", br#"{"limits":{}}"#, b"{", b"\xff\xfe"];
    for options in requests {
        assert_eq!(
            XlsxGraftVerify::path("/no/such/book.xlsx", options).err(),
            Some(XLSX_GRAFT_ERR_INVALID_ARG)
        );
        assert_eq!(
            XlsxGraftVerify::bytes(b"", options).err(),
            Some(XLSX_GRAFT_ERR_INVALID_ARG)
        );
    }
}

#[test]
fn a_fault_of_the_policy_is_a_diagnostic_and_the_workbook_is_not_opened() {
    let options = br#"{"policy":{"maxEntries":0},"rules":{"failOn":"warn"}}"#;
    let result = json(XlsxGraftVerify::path("/no/such/book.xlsx", options));
    assert_eq!(result["ok"], false);
    assert_eq!(result["failOn"], "warn");
    assert_eq!(result["error"], Value::Null);
    assert_eq!(codes(&result), ["policy_invalid"]);
}

#[test]
fn a_policy_is_checked_alone() {
    let valid = json(XlsxGraftVerify::policy(b"{}"));
    assert_eq!((valid["ok"].as_bool(), codes(&valid)), (Some(true), vec![]));
    for policy in [&br#"{"maxEntries":0}"#[..], b"\xff", b"["] {
        let result = json(XlsxGraftVerify::policy(policy));
        assert_eq!(
            (result["ok"].as_bool(), codes(&result)),
            (Some(false), vec!["policy_invalid"])
        );
    }
}

#[test]
fn the_handle_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<XlsxGraftVerify>();
}
