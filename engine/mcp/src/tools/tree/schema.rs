//! The JSON Schemas of the `tree` tool: its arguments and the
//! `structuredContent` of its result.

use crate::tools::schema;
use serde_json::{Value, json};

/// `{ path }` and nothing else.
pub(super) fn input() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "The workbook to list. A relative path is read from the server's working directory.",
            },
        },
        "required": ["path"],
        "additionalProperties": false,
    })
}

/// A relationship: `id`, `type` and `target`, which is a part of the package
/// (`part`) or a resource outside it (`external`).
fn relationship() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "type": { "type": "string" },
            "target": {
                "type": "object",
                "properties": {
                    "part": { "type": "string" },
                    "external": { "type": "string" },
                },
            },
        },
        "required": ["id", "type", "target"],
    })
}

/// A part: its name, content type, sizes and relationships.
fn part() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "contentType": { "type": ["string", "null"] },
            "contentTypeKnown": { "type": "boolean" },
            "size": { "type": "integer" },
            "compressedSize": { "type": "integer" },
            "readable": { "type": "boolean" },
            "referenced": { "type": ["boolean", "null"] },
            "relationshipsKnown": { "type": "boolean" },
            "relationships": { "type": "array", "items": relationship() },
        },
        "required": [
            "name",
            "contentType",
            "contentTypeKnown",
            "size",
            "compressedSize",
            "readable",
            "referenced",
            "relationshipsKnown",
            "relationships",
        ],
    })
}

/// The result: `ok`, `error`, `diagnostics`, `complete`,
/// `relationshipsKnown`, `relationships` and `parts`.
pub(super) fn output() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ok": { "type": "boolean" },
            "error": schema::error(),
            "diagnostics": schema::diagnostics(),
            "complete": { "type": "boolean" },
            "relationshipsKnown": { "type": "boolean" },
            "relationships": { "type": "array", "items": relationship() },
            "parts": { "type": "array", "items": part() },
        },
        "required": [
            "ok",
            "error",
            "diagnostics",
            "complete",
            "relationshipsKnown",
            "relationships",
            "parts",
        ],
    })
}
