//! Unit tests for `arguments::read`: the call it accepts and the three
//! messages of a call it refuses, with a list of one, two and three keys.

use crate::rpc::INVALID_PARAMS;
use crate::tools::arguments::read;
use serde_json::json;

#[test]
fn a_call_with_a_string_path_and_only_keys_the_tool_takes_is_read() {
    let value = json!({ "path": "a.xlsx", "extra": 1 });
    let read = read(&value, "tool", &["path", "extra"]).unwrap();
    assert_eq!(read.path, "a.xlsx");
    assert_eq!(read.map.len(), 2);
}

#[test]
fn a_call_without_an_object_with_a_string_path_is_invalid_params() {
    for value in [json!(null), json!([]), json!({}), json!({ "path": 1 })] {
        let err = read(&value, "tool", &["path"]).err().unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
        assert_eq!(err.message, "tool needs a string `path`");
    }
}

#[test]
fn a_key_the_tool_does_not_take_is_named_with_the_keys_it_takes() {
    let value = json!({ "path": "a.xlsx", "x": 1 });
    for (keys, taken) in [
        (&["path"][..], "path"),
        (&["path", "a"][..], "path and a"),
        (&["path", "a", "b"][..], "path, a and b"),
    ] {
        let err = read(&value, "tool", keys).err().unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
        assert_eq!(err.message, format!("tool takes {taken}, not x"));
    }
}
