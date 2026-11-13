//! Unit tests for the tools: the `version` descriptor and result, and the
//! calls that are refused. The reference tools and the check of each tool's
//! page are in the submodules.

mod pages;
mod reference;
mod verify;

use super::list;
use crate::Context;
use crate::rpc::{INVALID_PARAMS, RpcError};
use serde_json::{Value, json};
use xlsx_graft_core::version;

/// The `tools/call` result of `params` under the default context.
fn call(params: &Value) -> Result<Value, RpcError> {
    super::call(params, &Context::default())
}

#[test]
fn tools_list_returns_the_version_descriptor_with_every_field_as_its_first_entry() {
    assert_eq!(
        list()["tools"][0],
        json!({
            "name": "version",
            "title": "Engine version",
            "description": "The xlsx-graft engine version. Reference page: xlsx-graft://reference/version",
            "inputSchema": { "type": "object", "properties": {} },
            "outputSchema": {
                "type": "object",
                "properties": { "version": { "type": "string" } },
                "required": ["version"],
            },
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
