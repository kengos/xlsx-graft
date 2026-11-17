//! Unit tests for the view handle's logic: the tree and the cells of a
//! sheet from a path and from bytes, the options that are refused, and the
//! keys that `SheetRequest::from_arguments` refuses.

use super::XlsxGraftView;
use crate::message::lent;
use crate::status::{XLSX_GRAFT_ERR_INVALID_ARG, XLSX_GRAFT_ERR_REQUEST_INVALID};
use serde_json::Value;

/// The fixture workbooks.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

/// The result of `view` as a JSON value.
fn json(view: Result<XlsxGraftView, i32>) -> Value {
    serde_json::from_str(view.expect("a result").json()).expect("JSON")
}

/// This thread's message, copied out.
fn message() -> String {
    let (ptr, len) = lent();
    // SAFETY: `lent` points at `len` bytes of this thread's message, which
    // nothing replaces before they are copied.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The bytes of the fixture workbook `name`.
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{FIXTURES}/{name}")).expect("a fixture")
}

#[test]
fn the_tree_of_a_workbook_is_viewed_from_a_path_and_from_bytes() {
    let by_path = json(XlsxGraftView::tree_path(
        &format!("{FIXTURES}/table.xlsx"),
        b"",
    ));
    assert_eq!(by_path["ok"], true);
    assert_eq!(by_path["error"], Value::Null);
    assert!(!by_path["parts"].as_array().unwrap().is_empty());
    let data = fixture("table.xlsx");
    assert_eq!(json(XlsxGraftView::tree_bytes(&data, b"{}")), by_path);
    assert_eq!(json(XlsxGraftView::tree_bytes(&data, b"null")), by_path);
}

#[test]
fn the_cells_of_a_sheet_are_viewed_from_a_path_and_from_bytes() {
    let by_path = json(XlsxGraftView::describe_sheet_path(
        &format!("{FIXTURES}/merged.xlsx"),
        b"",
    ));
    assert_eq!(by_path["ok"], true);
    assert_eq!(by_path["sheet"]["index"], 0);
    assert!(!by_path["rows"].as_array().unwrap().is_empty());
    let data = fixture("merged.xlsx");
    assert_eq!(
        json(XlsxGraftView::describe_sheet_bytes(&data, b"{}")),
        by_path
    );
    let window = json(XlsxGraftView::describe_sheet_bytes(
        &data,
        br#"{"range": "A1:C3", "maxRows": 2}"#,
    ));
    assert_eq!(window["shown"], "A1:C2");
}

#[test]
fn a_file_that_cannot_be_opened_and_a_missing_sheet_are_the_results_error() {
    let missing = json(XlsxGraftView::tree_path("/no/such/book.xlsx", b""));
    assert_eq!(
        (missing["ok"].clone(), missing["error"]["code"].clone()),
        (false.into(), "io".into())
    );
    let data = fixture("table.xlsx");
    let nope = json(XlsxGraftView::describe_sheet_bytes(
        &data,
        br#"{"sheet": "Nope"}"#,
    ));
    assert_eq!(nope["ok"], false);
    assert_eq!(nope["error"]["code"], "sheet_not_found");
}

#[test]
fn bytes_that_are_not_a_zip_give_ok_false_and_not_a_status() {
    let tree = json(XlsxGraftView::tree_bytes(b"not a zip", b""));
    assert_eq!(tree["ok"], false);
    let grid = json(XlsxGraftView::describe_sheet_bytes(b"not a zip", b""));
    assert_eq!(grid["ok"], false);
}

#[test]
fn options_that_are_not_a_json_object_of_known_keys_are_invalid_arg_and_leave_the_earlier_message()
{
    crate::message::failed("an earlier failure");
    let options: [&[u8]; 4] = [b"[]", br#"{"x":1}"#, b"{", b"\xff\xfe"];
    for options in options {
        assert_eq!(
            XlsxGraftView::tree_bytes(b"", options).err(),
            Some(XLSX_GRAFT_ERR_INVALID_ARG)
        );
        assert_eq!(
            XlsxGraftView::describe_sheet_path("/no/such/book.xlsx", options).err(),
            Some(XLSX_GRAFT_ERR_INVALID_ARG)
        );
    }
    assert_eq!(
        XlsxGraftView::tree_path("/no/such/book.xlsx", br#"{"sheet":"a"}"#).err(),
        Some(XLSX_GRAFT_ERR_INVALID_ARG)
    );
    assert_eq!(message(), "an earlier failure");
    // A key the view does not take is checked before the values of the keys.
    assert_eq!(
        XlsxGraftView::describe_sheet_bytes(b"", br#"{"x":1,"maxRows":0}"#).err(),
        Some(XLSX_GRAFT_ERR_INVALID_ARG)
    );
    assert_eq!(message(), "an earlier failure");
}

#[test]
fn keys_that_from_arguments_refuses_return_request_invalid_with_the_reason() {
    let cases = [
        (
            r#"{"range":"nope"}"#,
            "range \"nope\" is not a range of cells",
        ),
        (r#"{"maxRows":0}"#, "maxRows 0 is not between 1 and 10000"),
        (
            r#"{"sheet":"a","sheetIndex":0}"#,
            "describe_sheet takes sheet or sheetIndex, not both",
        ),
        (
            r#"{"sheet":null}"#,
            "describe_sheet's sheet must be a string",
        ),
    ];
    for (options, reason) in cases {
        let by_path = XlsxGraftView::describe_sheet_path("/no/such/book.xlsx", options.as_bytes());
        assert_eq!(by_path.err(), Some(XLSX_GRAFT_ERR_REQUEST_INVALID));
        assert!(message().starts_with(reason), "{options}: {}", message());
        let by_bytes = XlsxGraftView::describe_sheet_bytes(b"", options.as_bytes());
        assert_eq!(by_bytes.err(), Some(XLSX_GRAFT_ERR_REQUEST_INVALID));
        assert!(message().starts_with(reason), "{options}: {}", message());
    }
}

#[test]
fn a_string_with_an_unpaired_surrogate_escape_is_not_json_and_is_invalid_arg() {
    let options = br#"{"sheet":"\ud800"}"#;
    assert_eq!(
        XlsxGraftView::describe_sheet_bytes(b"", options).err(),
        Some(XLSX_GRAFT_ERR_INVALID_ARG)
    );
}

#[test]
fn a_range_with_a_control_character_is_quoted_clipped_and_has_its_control_characters_removed_in_the_message()
 {
    let options = format!(r#"{{"range": "{}\u0007"}}"#, "r".repeat(1_000));
    let status = XlsxGraftView::describe_sheet_bytes(b"", options.as_bytes());
    assert_eq!(status.err(), Some(XLSX_GRAFT_ERR_REQUEST_INVALID));
    let message = message();
    assert!(message.starts_with("range \"rrr"), "{message}");
    assert!(message.contains('…'), "{message}");
    assert!(message.chars().count() < 400, "{message}");
    assert!(!message.chars().any(char::is_control), "{message:?}");
}

#[test]
fn the_handle_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<XlsxGraftView>();
}
