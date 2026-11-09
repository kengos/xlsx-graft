//! The `version` tool: the version of the engine the server was built from.

use super::{Tool, structured};
use crate::rpc::RpcError;
use serde_json::{Value, json};

/// The `version` row of [`super::TOOLS`].
pub(super) const TOOL: Tool = Tool {
    name: "version",
    title: "Engine version",
    description: "The xlsx-graft engine version.",
    input_schema,
    output_schema: Some(output_schema),
    run,
};

/// An object schema that declares no properties: the `version` tool
/// ignores any `arguments` a call passes.
fn input_schema() -> Value {
    json!({ "type": "object", "properties": {} })
}

/// The `structuredContent` of a `version` result: `{ "version": string }`,
/// with the `version` key required.
fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "version": { "type": "string" } },
        "required": ["version"],
    })
}

/// Returns the engine version as `structuredContent` and as text. The tool
/// ignores any `arguments`.
fn run(_arguments: &Value) -> Result<Value, RpcError> {
    Ok(structured(
        &json!({ "version": xlsx_graft_core::version() }),
    ))
}
