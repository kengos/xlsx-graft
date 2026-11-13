//! The `list_reference` and `get_reference` tools: the reference pages of
//! [`xlsx_graft_reference`], listed, and fetched by URI.
//!
//! `get_reference` finds a page with [`xlsx_graft_reference::page`], the
//! crate's one lookup by URI. A `resources/read` method added to the server
//! later must call the same function, so that the tool and the method
//! answer the same markdown for the same URI.

use super::{IsError, Tool, no_arguments, structured, text_result};
use crate::Context;
use crate::rpc::{INVALID_PARAMS, RpcError};
use serde_json::{Value, json};
use xlsx_graft_reference::{HOW_TO_READ, page, pages};

/// The `list_reference` row of [`super::TOOLS`].
pub(super) const LIST: Tool = Tool {
    name: "list_reference",
    title: "List reference pages",
    description: "Lists the reference pages compiled into the server: each page's uri, title, \
                  summary and the tools it documents. Read a tool's page with get_reference \
                  before calling the tool.",
    input_schema: no_arguments,
    output_schema: Some(list_output_schema),
    run: list,
    page: "reference",
};

/// The `get_reference` row of [`super::TOOLS`].
pub(super) const GET: Tool = Tool {
    name: "get_reference",
    title: "Get a reference page",
    description: "Returns one reference page's markdown, given the page's uri from \
                  list_reference or from the end of a tool's description in tools/list. A uri \
                  that names no page returns a result with isError true.",
    input_schema: get_input_schema,
    output_schema: None,
    run: get,
    page: "reference",
};

/// `{ "pages": [{ "uri", "title", "summary", "tools" }], "howToRead" }`.
fn list_output_schema() -> Value {
    json!({
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
    })
}

/// Returns every embedded page's `uri`, `title`, `summary` and `tools` as
/// `pages`, and [`HOW_TO_READ`] as `howToRead`. Any `arguments` are
/// ignored.
fn list(_arguments: &Value, _context: &Context) -> Result<Value, RpcError> {
    let pages: Vec<Value> = pages()
        .iter()
        .map(|page| {
            json!({
                "uri": page.uri(),
                "title": page.title(),
                "summary": page.summary(),
                "tools": page.tools(),
            })
        })
        .collect();
    Ok(structured(
        &json!({ "pages": pages, "howToRead": HOW_TO_READ }),
    ))
}

/// One required string, `uri`.
fn get_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "uri": {
                "type": "string",
                "description": "A page's uri, from list_reference or from the end of a tool's description in tools/list: xlsx-graft://reference/<page>.",
            },
        },
        "required": ["uri"],
    })
}

/// Returns the markdown of the page `arguments.uri` names.
///
/// A `uri` that names no page is a tool error (`isError: true`), which the
/// client's language model reads; the language model can call
/// `list_reference` again to find the right URI. A call whose
/// `arguments` has no string `uri` is a protocol error (`INVALID_PARAMS`).
fn get(arguments: &Value, _context: &Context) -> Result<Value, RpcError> {
    let Some(uri) = arguments.get("uri").and_then(Value::as_str) else {
        return Err(RpcError::new(
            INVALID_PARAMS,
            "get_reference needs a string `uri`",
        ));
    };
    Ok(match page(uri) {
        Ok(page) => text_result(page.body(), IsError::No),
        Err(err) => text_result(&err.to_string(), IsError::Yes),
    })
}
