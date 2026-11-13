//! Unit tests for `list_reference` and `get_reference`: the page list, one
//! page's markdown, a URI that names no page, and a call without a `uri`.

use super::call;
use crate::rpc::INVALID_PARAMS;
use crate::tools::list;
use serde_json::{Value, json};
use xlsx_graft_reference::{HOW_TO_READ, ReferenceError, pages};

/// The `tools/call` result of `get_reference` with `arguments`.
fn get(arguments: Value) -> Result<Value, crate::rpc::RpcError> {
    call(&json!({ "name": "get_reference", "arguments": arguments }))
}

/// The `tools/list` descriptor of the tool named `name`.
fn descriptor(name: &str) -> Value {
    let tools = list();
    tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap()
        .clone()
}

#[test]
fn list_reference_declares_no_arguments_and_the_page_list_as_its_output_schema() {
    assert_eq!(
        descriptor("list_reference"),
        json!({
            "name": "list_reference",
            "title": "List reference pages",
            "description": "Lists the reference pages compiled into the server: each page's uri, \
                title, summary and the tools it documents. Read a tool's page with get_reference \
                before calling the tool. Reference page: xlsx-graft://reference/reference",
            "inputSchema": { "type": "object", "properties": {} },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "pages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "uri": { "type": "string" },
                                "title": { "type": "string" },
                                "summary": { "type": "string" },
                                "tools": { "type": "array", "items": { "type": "string" } },
                            },
                            "required": ["uri", "title", "summary", "tools"],
                        },
                    },
                    "howToRead": { "type": "string" },
                },
                "required": ["pages", "howToRead"],
            },
        })
    );
}

#[test]
fn list_reference_answers_every_page_in_order_and_how_to_read_them() {
    let result = call(&json!({ "name": "list_reference" })).unwrap();
    assert_eq!(result["isError"], false);
    let content = &result["structuredContent"];
    assert_eq!(content["howToRead"], HOW_TO_READ);
    let uris: Vec<String> = content["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|page| page["uri"].as_str().unwrap().to_string())
        .collect();
    let expected: Vec<String> = pages().iter().map(|page| page.uri()).collect();
    assert_eq!(uris, expected);
    let reference = content["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|page| page["uri"] == "xlsx-graft://reference/reference")
        .unwrap();
    assert_eq!(
        reference,
        &json!({
            "uri": "xlsx-graft://reference/reference",
            "title": "Reading the reference",
            "summary": "How to list the reference pages and fetch one by its URI.",
            "tools": ["list_reference", "get_reference"],
        })
    );
    let text = result["content"][0]["text"].as_str().unwrap();
    assert_eq!(&serde_json::from_str::<Value>(text).unwrap(), content);
}

#[test]
fn get_reference_answers_each_page_as_one_text_part_without_structured_content() {
    for page in pages() {
        let result = get(json!({ "uri": page.uri() })).unwrap();
        assert_eq!(
            result,
            json!({
                "content": [{ "type": "text", "text": page.body() }],
                "isError": false,
            }),
            "{}",
            page.uri()
        );
    }
}

#[test]
fn get_reference_answers_a_uri_that_names_no_page_as_a_tool_error_whose_text_does_not_repeat_the_uri()
 {
    for uri in [
        "xlsx-graft://reference/nope",
        "xlsx-graft://reference/../version",
        "https://example.com/version",
        "",
    ] {
        let result = get(json!({ "uri": uri })).unwrap();
        assert_eq!(
            result,
            json!({
                "content": [{ "type": "text", "text": ReferenceError::UnknownUri.to_string() }],
                "isError": true,
            }),
            "{uri}"
        );
    }
}

#[test]
fn get_reference_without_a_string_uri_is_invalid_params() {
    for arguments in [json!(null), json!({}), json!({ "uri": 1 }), json!("uri")] {
        let err = get(arguments).unwrap_err();
        assert_eq!(err.code, INVALID_PARAMS);
        assert_eq!(err.message, "get_reference needs a string `uri`");
    }
    // A `tools/call` whose params have no `arguments` key.
    let err = call(&json!({ "name": "get_reference" })).unwrap_err();
    assert_eq!(err.code, INVALID_PARAMS);
}

#[test]
fn get_reference_declares_a_required_uri_and_no_output_schema() {
    assert_eq!(
        descriptor("get_reference"),
        json!({
            "name": "get_reference",
            "title": "Get a reference page",
            "description": "Returns one reference page's markdown, given the page's uri from \
                list_reference or from the end of a tool's description in tools/list. A uri \
                that names no page returns a result with isError true. Reference page: \
                xlsx-graft://reference/reference",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "A page's uri, from list_reference or from the end of a tool's description in tools/list: xlsx-graft://reference/<page>.",
                    },
                },
                "required": ["uri"],
            },
        })
    );
}
