//! Unit tests for the options of a view: what is read as the defaults, the
//! faults of the container (size, JSON, object, keys) and the faults of the
//! keys of `describe_sheet`.

use super::{ViewOptionsError, sheet_options, tree_options};
use crate::input::MAX_REQUEST_BYTES;
use crate::sheet::{SheetArgumentsError, SheetRequest};

#[test]
fn empty_whitespace_null_and_an_empty_object_are_the_defaults() {
    for json in [&b""[..], b" \n\t", b"null", b" null ", b"{}"] {
        assert_eq!(tree_options(json), Ok(()));
        assert_eq!(
            sheet_options(json).unwrap(),
            SheetRequest::new(None, None, None, None).unwrap()
        );
    }
}

#[test]
fn the_keys_of_describe_sheet_make_a_request() {
    let request = sheet_options(br#"{"sheetIndex": 1, "range": "A1:C3", "maxRows": 2}"#).unwrap();
    let expected = SheetRequest::new(
        Some(xlsx_graft_read::SheetChoice::Index(1)),
        Some("A1:C3"),
        Some(2),
        None,
    )
    .unwrap();
    assert_eq!(request, expected);
}

#[test]
fn options_larger_than_the_bound_are_too_large_and_the_bound_itself_is_read() {
    let mut json = vec![b' '; MAX_REQUEST_BYTES];
    assert_eq!(tree_options(&json), Ok(()));
    assert!(sheet_options(&json).is_ok());
    json.push(b' ');
    assert_eq!(tree_options(&json), Err(ViewOptionsError::TooLarge));
    assert_eq!(sheet_options(&json), Err(ViewOptionsError::TooLarge));
}

#[test]
fn text_that_is_not_json_or_not_an_object_is_refused() {
    for json in [&b"{"[..], b"\xff", b"nul"] {
        assert!(matches!(
            tree_options(json),
            Err(ViewOptionsError::NotJson(_))
        ));
        assert!(matches!(
            sheet_options(json),
            Err(ViewOptionsError::NotJson(_))
        ));
    }
    for json in [&b"[]"[..], b"1", b"\"a\"", b"true"] {
        assert_eq!(tree_options(json), Err(ViewOptionsError::NotObject));
        assert_eq!(sheet_options(json), Err(ViewOptionsError::NotObject));
    }
}

#[test]
fn a_key_the_view_does_not_take_is_refused_and_quoted_cleaned() {
    assert_eq!(
        tree_options(br#"{"sheet": "a"}"#),
        Err(ViewOptionsError::UnknownKey("sheet".to_owned()))
    );
    let error = sheet_options(br#"{"path": "a"}"#).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the options have a key the view does not take: path"
    );
    let long = format!(r#"{{"{}": 1}}"#, "k".repeat(1_000));
    match sheet_options(long.as_bytes()).unwrap_err() {
        ViewOptionsError::UnknownKey(key) => assert!(key.chars().count() < 300, "{key}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn keys_that_are_not_a_request_are_the_arguments_error_with_its_message() {
    let error = sheet_options(br#"{"sheet": "a", "sheetIndex": 0}"#).unwrap_err();
    assert_eq!(
        error,
        ViewOptionsError::Arguments(SheetArgumentsError::BothSheets)
    );
    assert_eq!(
        error.to_string(),
        "describe_sheet takes sheet or sheetIndex, not both"
    );
    assert_eq!(
        sheet_options(br#"{"maxRows": 0}"#).unwrap_err().to_string(),
        "maxRows 0 is not between 1 and 10000"
    );
}
