//! Unit tests for `describe_function`: the descriptor, the arguments that are
//! refused, the result for a function with a row of versions, a baseline
//! function and a name the table does not list, and the echo of the name.

mod schema;

use super::call;
use crate::rpc::INVALID_PARAMS;
use crate::tools::list;
use serde_json::{Value, json};
use xlsx_graft_host::describe_function;

/// The `tools/call` result of `describe_function` on `arguments`.
fn describe(arguments: Value) -> Result<Value, crate::rpc::RpcError> {
    call(&json!({ "name": "describe_function", "arguments": arguments }))
}

/// The result of `describe_function` on the name `name`.
fn described(name: &str) -> Value {
    describe(json!({ "name": name })).unwrap()
}

/// The `tools/list` descriptor of `describe_function`.
fn descriptor() -> Value {
    let tools = list();
    let found = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "describe_function");
    found.unwrap().clone()
}

/// The keys of the object `value`, in the order a sorted map keeps them.
fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn the_descriptor_declares_a_required_name_and_an_output_schema() {
    let mut descriptor = descriptor();
    let output_schema = descriptor.as_object_mut().unwrap().remove("outputSchema");
    assert!(output_schema.is_some());
    assert_eq!(
        descriptor,
        json!({
            "name": "describe_function",
            "title": "Describe a function's support",
            "description": "Looks the function `name` up in the compatibility table that \
                verify uses, and returns the table's entries for it: the name \
                without a leading `_xlfn.` and `_xlws.`, in upper case; the \
                name Excel writes for it in a file (`fileName`) and its \
                `prefix`; the version of each application that added it; the \
                features it needs as well; its safety; and the sources. \
                Returns { name, known, fileName, prefix, support, requires, \
                safety, sources }: `known` is `versioned`, `baseline` (no \
                `support` and no `requires`) or `unknown` (only `name` and \
                `known`; verify denies the name). A name the table does not \
                list is not an error. The `name` in the result is truncated to \
                200 characters, with control characters removed. Reference \
                page: xlsx-graft://reference/describe-function",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The function's name as a formula writes it, such as SUM \
                            or _xlfn.SEQUENCE. A leading `_xlfn.` and `_xlws.` and the letter case \
                            are ignored.",
                    },
                },
                "required": ["name"],
                "additionalProperties": false,
            },
        })
    );
}

#[test]
fn the_output_schema_has_one_shape_per_state_and_the_keys_of_each() {
    let schema = descriptor()["outputSchema"].clone();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["name", "known"]));
    assert_eq!(
        schema["properties"]["known"]["enum"],
        json!(["versioned", "baseline", "unknown"])
    );
    let shapes = schema["oneOf"].as_array().unwrap();
    let states: Vec<(&str, usize)> = shapes
        .iter()
        .map(|shape| {
            let known = shape["properties"]["known"]["const"].as_str().unwrap();
            (known, shape["required"].as_array().unwrap().len())
        })
        .collect();
    assert_eq!(states, [("versioned", 8), ("baseline", 6), ("unknown", 2)]);
    // The keys the schema lists are the keys of each state's result.
    for (name, known) in [("SEQUENCE", 0), ("SUM", 1), ("NOPE", 2)] {
        let structured = &described(name)["structuredContent"];
        let required: Vec<&str> = shapes[known]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|key| key.as_str().unwrap())
            .collect();
        let mut present = keys(structured);
        let mut expected = required.clone();
        present.sort_unstable();
        expected.sort_unstable();
        assert_eq!(present, expected, "{name}");
    }
    let apps = &schema["properties"]["support"]["required"];
    let versioned = &described("SEQUENCE")["structuredContent"];
    let mut app_keys = apps.as_array().unwrap().clone();
    app_keys.sort_by_key(|key| key.as_str().unwrap().to_owned());
    let actual: Vec<Value> = keys(&versioned["support"])
        .into_iter()
        .map(Value::from)
        .collect();
    assert_eq!(app_keys, actual);
    let entry = &schema["properties"]["support"]["properties"]["excel"];
    let mut entry_keys = entry["required"].as_array().unwrap().clone();
    entry_keys.sort_by_key(|key| key.as_str().unwrap().to_owned());
    let actual: Vec<Value> = keys(&versioned["support"]["excel"])
        .into_iter()
        .map(Value::from)
        .collect();
    assert_eq!(entry_keys, actual);
}

#[test]
fn a_call_without_an_object_of_arguments_with_a_string_name_is_invalid_params() {
    for arguments in [json!(null), json!([]), json!({}), json!({ "name": 1 })] {
        let err = describe(arguments).unwrap_err();
        assert_eq!(err.code, INVALID_PARAMS);
        assert_eq!(err.message, "describe_function needs a string `name`");
    }
}

#[test]
fn a_key_the_tool_does_not_take_is_invalid_params() {
    let err = describe(json!({ "name": "SUM", "path": "a.xlsx" })).unwrap_err();
    assert_eq!(err.code, INVALID_PARAMS);
    assert_eq!(err.message, "describe_function takes name, not path");
}

#[test]
fn each_state_is_the_hosts_json_as_structured_content_and_as_text_and_is_not_an_error() {
    let names = [
        "SEQUENCE",
        "XMATCH",
        "SUM",
        "ANCHORARRAY",
        "WEBSERVICE",
        "NOPE",
    ];
    for name in names {
        let result = described(name);
        let expected: Value =
            serde_json::from_str(&describe_function(name).to_json().unwrap()).unwrap();
        assert_eq!(result["structuredContent"], expected, "{name}");
        assert_eq!(result["isError"], false, "{name}");
        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        let text = content[0]["text"].as_str().unwrap();
        assert_eq!(text, expected.to_string(), "{name}");
    }
    let known: Vec<String> = ["SEQUENCE", "SUM", "NOPE"]
        .iter()
        .map(|name| described(name)["structuredContent"]["known"].to_string())
        .collect();
    assert_eq!(known, ["\"versioned\"", "\"baseline\"", "\"unknown\""]);
    let null_prefix = &described("XMATCH")["structuredContent"];
    assert_eq!(null_prefix["prefix"], Value::Null);
    assert_eq!(null_prefix["fileName"], Value::Null);
}

#[test]
fn a_name_is_looked_up_without_its_prefixes_and_in_any_case() {
    let result = described("_xlfn._xlws.sort");
    assert_eq!(result["structuredContent"]["name"], "SORT");
    assert_eq!(result["structuredContent"]["known"], "versioned");
}

#[test]
fn the_name_of_an_unknown_function_is_truncated_and_has_no_control_characters() {
    let result = described(&format!("nope\u{7}{}", "y".repeat(400)));
    let name = format!("NOPE{}", "Y".repeat(196));
    assert_eq!(
        result["structuredContent"],
        json!({ "name": name, "known": "unknown" })
    );
    assert_eq!(result["isError"], false);
    let text = result["content"][0]["text"].as_str().unwrap();
    let in_text: Value = serde_json::from_str(text).unwrap();
    assert_eq!(in_text, result["structuredContent"]);
}
