//! The pieces of an output schema that every tool with `error` and
//! `diagnostics` in its result shares.

use serde_json::{Value, json};

/// The schema of a result's `error`: `null`, or `{ code, message }`.
pub(super) fn error() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "code": { "type": "string" },
            "message": { "type": "string" },
        },
        "required": ["code", "message"],
    })
}

/// The schema of a result's `diagnostics`: an array of `{ code, severity,
/// message }` and the keys after them.
pub(super) fn diagnostics() -> Value {
    json!({
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
    })
}
