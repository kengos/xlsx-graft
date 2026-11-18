//! Unit tests for [`SheetRequest::from_arguments`]: the keys it reads, the
//! order it checks them in and the messages of its faults.

use super::{SheetArgumentsError, SheetRequest};
use crate::sheet::request::Selection;
use serde_json::{Value, json};

/// The result of reading `value`, which the test knows is an object.
fn read(value: &Value) -> Result<SheetRequest, SheetArgumentsError> {
    SheetRequest::from_arguments(value.as_object().unwrap())
}

/// The message of the fault reading `value` gives.
fn message(value: &Value) -> String {
    read(value).unwrap_err().to_string()
}

#[test]
fn an_object_without_keys_is_the_default_request() {
    let request = read(&json!({})).unwrap();
    assert_eq!(request.sheet, Selection::First);
    assert_eq!(request.range, None);
    assert_eq!(
        (request.max_rows, request.max_columns),
        (
            SheetRequest::DEFAULT_MAX_ROWS,
            SheetRequest::DEFAULT_MAX_COLUMNS
        )
    );
}

#[test]
fn every_key_of_describe_sheet_is_read_and_another_key_is_ignored() {
    let request = read(&json!({
        "sheet": "Data", "range": "B2:D9", "maxRows": 5, "maxColumns": 7, "path": 1
    }))
    .unwrap();
    assert_eq!(request.sheet, Selection::Name("Data".to_owned()));
    assert_eq!(request.range.unwrap().to_string(), "B2:D9");
    assert_eq!((request.max_rows, request.max_columns), (5, 7));
    assert_eq!(
        read(&json!({"sheetIndex": 3})).unwrap().sheet,
        Selection::Index(3)
    );
    assert_eq!(
        SheetRequest::ARGUMENTS,
        ["sheet", "sheetIndex", "range", "maxRows", "maxColumns"]
    );
}

#[test]
fn a_value_of_the_wrong_type_is_a_fault_that_names_the_key() {
    for value in [json!(1), json!(null), json!(true), json!(["a"])] {
        assert_eq!(
            message(&json!({"sheet": value})),
            "describe_sheet's sheet must be a string"
        );
        assert_eq!(
            message(&json!({"range": value})),
            "describe_sheet's range must be a string"
        );
    }
    for value in [json!(-1), json!(5.0), json!("1"), json!(null)] {
        assert_eq!(
            message(&json!({"sheetIndex": value})),
            "describe_sheet's sheetIndex must be an integer from 0"
        );
        assert_eq!(
            message(&json!({"maxRows": value})),
            "describe_sheet's maxRows must be an integer from 1 to 10000"
        );
        assert_eq!(
            message(&json!({"maxColumns": value})),
            "describe_sheet's maxColumns must be an integer from 1 to 1000"
        );
    }
}

#[test]
fn sheet_and_sheet_index_together_are_a_fault() {
    assert_eq!(
        message(&json!({"sheet": "a", "sheetIndex": 0})),
        "describe_sheet takes sheet or sheetIndex, not both"
    );
}

#[test]
fn the_faults_are_found_in_the_order_the_keys_are_checked() {
    assert_eq!(
        message(&json!({"sheet": 1, "sheetIndex": "x", "range": 1})),
        "describe_sheet's sheet must be a string"
    );
    assert_eq!(
        message(&json!({"sheetIndex": "x", "sheet": "a", "range": 1})),
        "describe_sheet's sheetIndex must be an integer from 0"
    );
    assert_eq!(
        message(&json!({"sheet": "a", "sheetIndex": 0, "range": 1})),
        "describe_sheet takes sheet or sheetIndex, not both"
    );
    assert_eq!(
        message(&json!({"range": 1, "maxRows": "x", "maxColumns": "x"})),
        "describe_sheet's range must be a string"
    );
    assert_eq!(
        message(&json!({"maxRows": "x", "maxColumns": "x"})),
        "describe_sheet's maxRows must be an integer from 1 to 10000"
    );
}

#[test]
fn a_fault_of_the_request_keeps_the_message_of_the_request_error() {
    assert!(matches!(
        read(&json!({"range": "nope"})).unwrap_err(),
        SheetArgumentsError::Request(_)
    ));
    assert_eq!(
        message(&json!({"maxRows": 0})),
        "maxRows 0 is not between 1 and 10000"
    );
    assert_eq!(
        message(&json!({"maxColumns": 1001})),
        "maxColumns 1001 is not between 1 and 1000"
    );
}
