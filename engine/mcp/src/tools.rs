//! The MCP tools: what `tools/list` describes and what `tools/call` runs.
//!
//! Every tool is one row of [`TOOLS`]: its descriptor, the function that
//! runs it and the reference page that documents it. Adding a tool is
//! adding a row (and the module that holds the row), naming the tool in
//! its page's `tools`, and writing a `tools/call` example for the tool in
//! that page. A new page is a file `engine/reference/pages/<page>.md` and a
//! `page!("<page>")` line, in name order, in the `PAGES` list of
//! `engine/reference/src/embed.rs`. `list` and `call` read the table and
//! name no tool themselves.

mod reference;
mod version;

use crate::Context;
use crate::rpc::{INVALID_PARAMS, RpcError, clip};
use serde_json::{Map, Value, json};
use xlsx_graft_reference::URI_PREFIX;

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
    /// none), under the server's `context`, and returns the `tools/call`
    /// result.
    ///
    /// Returns `Err` only for a protocol error, such as `arguments` that do
    /// not match `input_schema`; the server sends it as a JSON-RPC error
    /// response. A failure while the tool runs is an `Ok` result with
    /// `isError: true`, as MCP 2025-06-18 separates the two.
    pub(crate) run: fn(&Value, &Context) -> Result<Value, RpcError>,
    /// The name of the reference page that documents the tool:
    /// `engine/reference/pages/<page>.md`, whose front matter names the
    /// tool in its `tools`. `tools/list` ends the description with the
    /// page's URI. `tools/tests/pages.rs` checks that the page is embedded
    /// and names the tool in its `tools`; `server/tests/examples.rs` checks
    /// that the page has a `tools/call` example for the tool.
    pub(crate) page: &'static str,
}

/// Every tool, in the order `tools/list` returns them.
pub(crate) const TOOLS: &[Tool] = &[version::TOOL, reference::LIST, reference::GET];

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
    let description = format!(
        "{} Reference page: {URI_PREFIX}{}",
        tool.description, tool.page
    );
    entry.insert("description".into(), description.into());
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
pub(crate) fn call(params: &Value, context: &Context) -> Result<Value, RpcError> {
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
    (tool.run)(params.get("arguments").unwrap_or(&Value::Null), context)
}

/// An input schema that declares no properties. A tool with this schema
/// ignores any `arguments` a call passes.
fn no_arguments() -> Value {
    json!({ "type": "object", "properties": {} })
}

/// The `isError` of a tool result: whether the tool failed while it ran.
#[derive(Clone, Copy)]
enum IsError {
    Yes,
    No,
}

/// A tool result whose `content` is one text part holding `body`, with no
/// `structuredContent`.
fn text_result(body: &str, is_error: IsError) -> Value {
    json!({
        "content": [{ "type": "text", "text": body }],
        "isError": matches!(is_error, IsError::Yes),
    })
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
