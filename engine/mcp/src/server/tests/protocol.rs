//! The JSON-RPC error each bad frame gets, and the loop going on after it.

use super::{PING, talk};
use crate::rpc::MAX_LINE;
use serde_json::json;

#[test]
fn broken_json_is_a_parse_error_and_the_loop_goes_on() {
    let responses = talk(&format!("{{oops\n{PING}\n"));
    assert_eq!(responses.len(), 2);
    assert!(responses[0]["id"].is_null());
    assert_eq!(responses[0]["error"]["code"], -32700);
    let message = responses[0]["error"]["message"].as_str().unwrap();
    assert!(message.starts_with("invalid JSON: "), "{message}");
    assert_eq!(responses[1]["id"], 9);
}

#[test]
fn input_that_is_not_an_object_is_an_invalid_request() {
    for input in ["[1,2]", "[]", "\"ping\"", "3", "null"] {
        let responses = talk(input);
        assert_eq!(
            responses,
            [json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32600, "message": "expected a JSON-RPC request object" },
            })],
            "{input}"
        );
    }
}

#[test]
fn a_message_without_a_string_method_is_invalid_with_or_without_an_id() {
    let error = json!({ "code": -32600, "message": "missing or non-string method" });
    for (input, id) in [
        (r#"{"jsonrpc":"2.0","id":1,"method":3}"#, json!(1)),
        (r#"{"jsonrpc":"2.0","id":"a"}"#, json!("a")),
        // JSON-RPC 2.0 § 5's own example: no id to echo, so `id: null`.
        (
            r#"{"jsonrpc":"2.0","method":1,"params":"bar"}"#,
            json!(null),
        ),
        (r#"{"jsonrpc":"2.0"}"#, json!(null)),
    ] {
        assert_eq!(
            talk(input),
            [json!({ "jsonrpc": "2.0", "id": id, "error": error })],
            "{input}"
        );
    }
}

#[test]
fn an_unknown_method_is_method_not_found_with_a_bounded_echo() {
    // Built with json! so the BEL arrives escaped: the method name itself
    // carries it, not the frame.
    let hostile = format!("x\u{7}{}", "y".repeat(400));
    let request = json!({ "jsonrpc": "2.0", "id": 1, "method": hostile }).to_string();
    let responses = talk(&request);
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(
        responses[0]["error"],
        json!({ "code": -32601, "message": format!("unknown method: x{}", "y".repeat(199)) })
    );
}

/// A ping padded with trailing spaces to exactly `len` bytes.
fn ping_of_length(len: usize) -> String {
    format!("{PING}{}", " ".repeat(len - PING.len()))
}

#[test]
fn a_request_line_of_exactly_the_cap_is_answered() {
    let line = ping_of_length(MAX_LINE);
    assert_eq!(line.len(), MAX_LINE);
    let responses = talk(&format!("{line}\n"));
    assert_eq!(
        responses,
        [json!({ "jsonrpc": "2.0", "id": 9, "result": {} })]
    );
}

#[test]
fn a_request_line_one_byte_past_the_cap_is_a_parse_error_and_the_loop_goes_on() {
    let input = format!("{}\n{PING}\n", ping_of_length(MAX_LINE + 1));
    let responses = talk(&input);
    assert_eq!(
        responses,
        [
            json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": "request line exceeds the 1 MiB cap" },
            }),
            json!({ "jsonrpc": "2.0", "id": 9, "result": {} }),
        ]
    );
}

#[test]
fn blank_lines_are_skipped() {
    assert_eq!(
        talk(&format!("\n  \r\n{PING}\n")),
        [json!({ "jsonrpc": "2.0", "id": 9, "result": {} })]
    );
}
