//! The MCP tools: what `tools/list` describes and what `tools/call` runs.
//!
//! Every tool is one row of [`TOOLS`]: its descriptor and the function that
//! runs it. Adding a tool is adding a row (and the module that holds the
//! row); `list` and `call` read the table and name no tool themselves.

mod version;

use crate::rpc::{INVALID_PARAMS, RpcError, clip};
use serde_json::{Map, Value, json};

/// One MCP tool: the descriptor `tools/list` returns and the function
/// `tools/call` runs.
pub(crate) struct Tool {
    /// The name a client passes as `tools/call`'s `params.name`.
    pub(crate) name: &'static str,
    /// The descriptor's `title`: a short display name the client shows to
    /// the user.
    pub(crate) title: &'static str,
    /// The descriptor's `description`: what the tool does, for the language
    /// model that chooses the tool.
    pub(crate) description: &'static str,
    /// The descriptor's `inputSchema`: the JSON Schema of `arguments`.
    pub(crate) input_schema: fn() -> Value,
    /// The descriptor's `outputSchema`: the JSON Schema of the result's
    /// `structuredContent`. `None` for a tool whose result carries no
    /// `structuredContent`; MCP 2025-06-18 requires `structuredContent`
    /// from a tool that declares an `outputSchema`.
    pub(crate) output_schema: Option<fn() -> Value>,
    /// Runs the tool on the call's `arguments` (`null` when the call has
    /// none) and returns the `tools/call` result.
    ///
    /// Returns `Err` only for a protocol error, such as `arguments` that do
    /// not match `input_schema`; the server sends it as a JSON-RPC error
    /// response. A failure while the tool runs is an `Ok` result with
    /// `isError: true`, as MCP 2025-06-18 separates the two.
    pub(crate) run: fn(&Value) -> Result<Value, RpcError>,
}

/// Every tool, in the order `tools/list` returns them.
pub(crate) const TOOLS: &[Tool] = &[version::TOOL];

/// The `tools/list` result.
pub(crate) fn list() -> Value {
    let tools: Vec<Value> = TOOLS.iter().map(descriptor).collect();
    json!({ "tools": tools })
}

/// One tool's entry in the `tools/list` result.
fn descriptor(tool: &Tool) -> Value {
    let mut entry = Map::new();
    entry.insert("name".into(), tool.name.into());
    entry.insert("title".into(), tool.title.into());
    entry.insert("description".into(), tool.description.into());
    entry.insert("inputSchema".into(), (tool.input_schema)());
    if let Some(output_schema) = tool.output_schema {
        entry.insert("outputSchema".into(), output_schema());
    }
    Value::Object(entry)
}

/// `tools/call`: runs the tool `params.name` names on `params.arguments`.
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
    let Some(tool) = TOOLS.iter().find(|tool| tool.name == name) else {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("unknown tool: {}", clip(name)),
        ));
    };
    (tool.run)(params.get("arguments").unwrap_or(&Value::Null))
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
