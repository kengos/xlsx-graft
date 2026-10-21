//! The MCP tools: what `tools/list` describes and what `tools/call` runs.
//! For now the one tool is `version`.

use crate::rpc::{INVALID_PARAMS, RpcError, clip};
use serde_json::{Value, json};

/// The `tools/list` result.
pub(crate) fn list() -> Value {
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
}

/// `tools/call`: runs the tool `params.name` names.
///
/// A tool that does not exist, or a call without a name, is a protocol error
/// (`INVALID_PARAMS`), as MCP 2025-06-18 says for an unknown tool.
pub(crate) fn call(params: &Value) -> Result<Value, RpcError> {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err(RpcError::new(
            INVALID_PARAMS,
            "tools/call needs a string `name`",
        ));
    };
    match name {
        "version" => Ok(structured(
            &json!({ "version": xlsx_graft_core::version() }),
        )),
        other => Err(RpcError::new(
            INVALID_PARAMS,
            format!("unknown tool: {}", clip(other)),
        )),
    }
}

/// A successful tool result carrying `value` as `structuredContent`, and the
/// same JSON as a text part for a client that reads only `content`.
fn structured(value: &Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": value.to_string() }],
        "structuredContent": value,
        "isError": false,
    })
}

#[cfg(test)]
mod tests;
