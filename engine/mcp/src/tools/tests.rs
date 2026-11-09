//! Unit tests for the tools: the `version` descriptor and result, the calls
//! that are refused, and the descriptor of a tool with no output schema.

use super::{Tool, call, descriptor, list};
use crate::rpc::INVALID_PARAMS;
use serde_json::{Value, json};
use xlsx_graft_core::version;

#[test]
fn tools_list_returns_only_the_version_descriptor_with_every_field() {
    assert_eq!(
        list(),
        json!({
            "tools": [{
                "name": "version",
                "title": "Engine version",
                "description": "The xlsx-graft engine version.",
                "inputSchema": { "type": "object", "properties": {} },
                "outputSchema": {
                    "type": "object",
                    "properties": { "version": { "type": "string" } },
                    "required": ["version"],
                },
            }],
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

#[test]
fn a_tool_without_an_output_schema_has_no_output_schema_key_in_its_descriptor() {
    fn no_arguments() -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn run(_arguments: &Value) -> Result<Value, crate::rpc::RpcError> {
        Ok(json!({}))
    }
    let tool = Tool {
        name: "plain",
        title: "Plain",
        description: "A tool whose result carries no structuredContent.",
        input_schema: no_arguments,
        output_schema: None,
        run,
    };
    assert_eq!(run(&Value::Null), Ok(json!({})));
    assert_eq!(
        descriptor(&tool),
        json!({
            "name": "plain",
            "title": "Plain",
            "description": "A tool whose result carries no structuredContent.",
            "inputSchema": { "type": "object", "properties": {} },
        })
    );
}
