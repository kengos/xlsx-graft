//! The JSON Schemas of the `verify` tool: its arguments and the
//! `structuredContent` of its result.

use serde_json::{Value, json};

/// `{ path, targets?, bestEffort?, failOn? }` and nothing else.
pub(super) fn input() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "The workbook to verify. A relative path is read from the server's working directory.",
            },
            "targets": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Versions the workbook must work in, such as `excel >= 2016`. What it does not support is an error.",
            },
            "bestEffort": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Versions the workbook should work in, such as `excel-mac >= 2019`. What it does not support is a warning.",
            },
            "failOn": {
                "type": "string",
                "enum": ["error", "warn"],
                "description": "`warn` makes a warning fail the check too. The server's policy may be stricter.",
            },
        },
        "required": ["path"],
        "additionalProperties": false,
    })
}

/// The result: `ok`, `failOn`, `counts`, `error` and `diagnostics`.
pub(super) fn output() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ok": { "type": "boolean" },
            "failOn": { "type": "string", "enum": ["error", "warn"] },
            "counts": {
                "type": "object",
                "properties": {
                    "error": { "type": "integer" },
                    "warn": { "type": "integer" },
                    "notice": { "type": "integer" },
                },
                "required": ["error", "warn", "notice"],
            },
            "error": {
                "type": ["object", "null"],
                "properties": {
                    "code": { "type": "string" },
                    "message": { "type": "string" },
                },
                "required": ["code", "message"],
            },
            "diagnostics": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string" },
                        "severity": { "type": "string" },
                        "message": { "type": "string" },
                    },
                    "required": ["code", "severity", "message"],
                },
            },
        },
        "required": ["ok", "failOn", "counts", "error", "diagnostics"],
    })
}
