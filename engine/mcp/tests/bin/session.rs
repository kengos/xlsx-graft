//! A client's whole session: initialize, the initialized notification, then
//! listing and calling `version`. The `structuredContent` case pins the
//! contract: the whole object is compared, so a renamed, added or retyped
//! key fails here.

use crate::{responses, stderr, talk_to, xlsx_graft_mcp};
use serde_json::json;

#[test]
fn a_session_lists_and_calls_version() {
    let output = talk_to(
        xlsx_graft_mcp(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"version","arguments":{}}}"#,
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let responses = responses(&output);
    // Four lines in, three out: the notification is not answered.
    assert_eq!(responses.len(), 3);

    assert_eq!(responses[0]["id"], 1);
    assert_eq!(
        responses[0]["result"],
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "xlsx-graft-mcp", "version": "0.0.0" },
        })
    );

    assert_eq!(responses[1]["id"], 2);
    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "version");
    assert_eq!(tools[0]["outputSchema"]["required"], json!(["version"]));

    assert_eq!(responses[2]["id"], 3);
    let result = &responses[2]["result"];
    assert_eq!(result["structuredContent"], json!({ "version": "0.0.0" }));
    assert_eq!(
        result["content"],
        json!([{ "type": "text", "text": "{\"version\":\"0.0.0\"}" }])
    );
    assert_eq!(result["isError"], false);
}

#[test]
fn a_bad_request_is_answered_and_the_session_goes_on() {
    let output = talk_to(
        xlsx_graft_mcp(),
        &["{oops", r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#],
    );
    assert_eq!(output.status.code(), Some(0));
    let responses = responses(&output);
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["error"]["code"], -32700);
    assert_eq!(
        responses[1],
        json!({ "jsonrpc": "2.0", "id": 1, "result": {} })
    );
}
