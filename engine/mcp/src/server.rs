//! The dispatch loop: the initialize handshake, method routing, and which
//! JSON-RPC error each bad request gets.

use crate::McpError;
use crate::rpc::{
    Frame, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, RpcError, clip, error_response,
    read_frame, result_response, write_frame,
};
use crate::tools;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

/// The MCP revision this server speaks, and answers with when the client
/// asks for one it does not know.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Every revision this server accepts. A revision is added only after its
/// changelog has been read against this loop: 2025-03-26, for one, requires
/// a server to accept JSON-RPC batches, which this loop answers with
/// INVALID_REQUEST (batching was removed in 2025-06-18), and has no
/// `structuredContent`, which every tool here answers with.
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[PROTOCOL_VERSION];

/// Answers newline-delimited JSON-RPC requests from `reader` on `writer`
/// until `reader` ends.
///
/// A malformed, unknown or failing request is answered with a JSON-RPC error
/// and the loop goes on. A notification (a request with a method but no `id`)
/// is never answered.
///
/// # Errors
///
/// [`McpError::Io`] when `reader` cannot be read or `writer` refuses a
/// response.
pub fn serve<R: BufRead, W: Write>(reader: &mut R, writer: &mut W) -> Result<(), McpError> {
    loop {
        match read_frame(reader)? {
            Frame::Eof => return Ok(()),
            Frame::Oversized => {
                // Says MAX_LINE in words: change the two together.
                let error = RpcError::new(PARSE_ERROR, "request line exceeds the 1 MiB cap");
                write_frame(writer, &error_response(Value::Null, &error))?;
            }
            Frame::Line(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                if let Some(response) = handle_line(&line) {
                    write_frame(writer, &response)?;
                }
            }
        }
    }
}

/// Parses one line and dispatches it; `None` means nothing to send (a
/// notification, which is never answered, not even with an error).
fn handle_line(line: &str) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(err) => {
            let error = RpcError::new(PARSE_ERROR, format!("invalid JSON: {err}"));
            return Some(error_response(Value::Null, &error));
        }
    };
    let Value::Object(request) = message else {
        let error = RpcError::new(INVALID_REQUEST, "expected a JSON-RPC request object");
        return Some(error_response(Value::Null, &error));
    };
    // MCP allows a string or an integer; any other id is echoed back as it
    // came rather than refused, since echoing it harms nothing here.
    let id = request.get("id").cloned();
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        // Not a request, so not a notification either: answered even without
        // an id, with `id: null` (JSON-RPC 2.0 § 5). A client's response
        // (an id with `result` or `error`) lands here too; this server sends
        // no requests, so a client has none to answer. Decide how to take
        // them when a server-to-client request (sampling, elicitation) lands.
        let error = RpcError::new(INVALID_REQUEST, "missing or non-string method");
        return Some(error_response(id.unwrap_or(Value::Null), &error));
    };
    // No id: a notification. Nothing here acts on one, and none is answered.
    let id = id?;
    let params = request.get("params").unwrap_or(&Value::Null);
    Some(match dispatch(method, params) {
        Ok(result) => result_response(id, result),
        Err(error) => error_response(id, &error),
    })
}

/// Routes a request to its method.
fn dispatch(method: &str, params: &Value) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(initialize_result(params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools::list()),
        "tools/call" => tools::call(params),
        _ => Err(RpcError::new(
            METHOD_NOT_FOUND,
            format!("unknown method: {}", clip(method)),
        )),
    }
}

/// The `initialize` result: the agreed revision, the capabilities (tools
/// only) and this build's identity.
fn initialize_result(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    json!({
        "protocolVersion": negotiate_version(requested),
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "xlsx-graft-mcp", "version": xlsx_graft_core::version() },
    })
}

/// Echoes a requested revision this server accepts; anything else, or none,
/// gets [`PROTOCOL_VERSION`], and the client decides whether to go on.
fn negotiate_version(requested: Option<&str>) -> &'static str {
    SUPPORTED_PROTOCOL_VERSIONS
        .iter()
        .copied()
        .find(|v| Some(*v) == requested)
        .unwrap_or(PROTOCOL_VERSION)
}

#[cfg(test)]
mod tests;
