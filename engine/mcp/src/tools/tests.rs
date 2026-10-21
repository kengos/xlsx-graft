//! Unit tests for the tools: the `version` descriptor and result, and the
//! calls that are refused.

use super::{call, list};
use crate::rpc::INVALID_PARAMS;
use serde_json::json;
use xlsx_graft_core::version;

#[test]
fn version_is_the_one_tool_with_its_input_and_output_schema() {
    let tools = list();
    let tools = tools["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "version");
    assert_eq!(
        tools[0]["inputSchema"],
        json!({ "type": "object", "properties": {} })
    );
    assert_eq!(
        tools[0]["outputSchema"],
        json!({
            "type": "object",
            "properties": { "version": { "type": "string" } },
            "required": ["version"],
        })
    );
}

#[test]
fn version_answers_structured_content_and_the_same_json_as_text() {
    let result = call(&json!({ "name": "version" })).unwrap();
    let expected = json!({ "version": version() });
    assert_eq!(result["structuredContent"], expected);
    assert_eq!(result["isError"], false);
    let content = result["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    let text = content[0]["text"].as_str().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(text).unwrap(),
        expected
    );
}

#[test]
fn version_ignores_any_arguments() {
    let with = call(&json!({ "name": "version", "arguments": { "x": 1 } })).unwrap();
    assert_eq!(with, call(&json!({ "name": "version" })).unwrap());
}

#[test]
fn an_unknown_tool_is_invalid_params_with_a_bounded_echo() {
    let err = call(&json!({ "name": format!("nope\u{7}{}", "y".repeat(400)) })).unwrap_err();
    assert_eq!(err.code, INVALID_PARAMS);
    assert_eq!(
        err.message,
        format!("unknown tool: nope{}", "y".repeat(196))
    );
}

#[test]
fn a_call_without_a_string_name_is_invalid_params() {
    for params in [json!(null), json!({}), json!({ "name": 1 })] {
        let err = call(&params).unwrap_err();
        assert_eq!(err.code, INVALID_PARAMS);
        assert_eq!(err.message, "tools/call needs a string `name`");
    }
}
