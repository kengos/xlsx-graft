//! The initialize handshake, the revision agreement, notifications and
//! ping, and the tool methods reached through the loop.

use super::{PING, talk};
use serde_json::json;
use xlsx_graft_core::version;

fn initialize_with(params: &str) -> serde_json::Value {
    let request = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"initialize"{params}}}"#);
    let mut responses = talk(&request);
    assert_eq!(responses.len(), 1);
    responses.remove(0)
}

#[test]
fn initialize_agrees_the_revision_and_names_the_server() {
    let response = initialize_with(
        r#","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}"#,
    );
    assert_eq!(
        response,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "xlsx-graft-mcp", "version": version() },
            },
        })
    );
}

#[test]
fn an_unknown_or_missing_revision_gets_the_one_this_server_speaks() {
    // 2025-03-26 is not accepted: it requires JSON-RPC batches.
    for params in [
        r#","params":{"protocolVersion":"2025-03-26"}"#,
        r#","params":{"protocolVersion":"1999-01-01"}"#,
        r#","params":{"protocolVersion":7}"#,
        "",
    ] {
        let response = initialize_with(params);
        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
    }
}

#[test]
fn notifications_are_never_answered_and_the_session_goes_on() {
    let input = [
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}"#,
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"nope"}}"#,
        PING,
    ]
    .join("\n");
    let responses = talk(&input);
    assert_eq!(
        responses,
        [json!({ "jsonrpc": "2.0", "id": 9, "result": {} })]
    );
}

#[test]
fn string_and_integer_ids_are_echoed_back() {
    for id in [json!("abc"), json!(0), json!(-7), json!(u64::MAX)] {
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": "ping" }).to_string();
        assert_eq!(
            talk(&request),
            [json!({ "jsonrpc": "2.0", "id": id, "result": {} })]
        );
    }
}

#[test]
fn the_tool_methods_answer_through_the_loop() {
    let input = [
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"version"}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nope"}}"#,
    ]
    .join("\n");
    let responses = talk(&input);
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["result"]["tools"][0]["name"], "version");
    assert_eq!(
        responses[1]["result"]["structuredContent"],
        json!({ "version": version() })
    );
    assert_eq!(
        responses[2]["error"],
        json!({ "code": -32602, "message": "unknown tool: nope" })
    );
}
