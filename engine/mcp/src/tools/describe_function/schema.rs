//! The JSON Schemas of the `describe_function` tool: its arguments and the
//! `structuredContent` of its result.

use serde_json::{Value, json};

/// The applications of `support`, in the order the table has them.
const APPS: [&str; 6] = [
    "excel",
    "excel-mac",
    "libreoffice",
    "google-sheets",
    "numbers",
    "onlyoffice",
];

/// `{ name }` and nothing else.
pub(super) fn input() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The function's name as a formula writes it, such as SUM or _xlfn.SEQUENCE. A leading `_xlfn.` and `_xlws.` and the letter case are ignored.",
            },
        },
        "required": ["name"],
        "additionalProperties": false,
    })
}

/// One application's cell of the table: `added` is a version, `false` for
/// no version or `null` for not known.
fn entry() -> Value {
    json!({
        "type": "object",
        "properties": {
            "added": { "type": ["string", "boolean", "null"] },
            "removed": { "type": ["string", "null"] },
            "partial": { "type": "boolean" },
            "fallback": { "type": ["string", "null"] },
            "notes": { "type": ["string", "null"] },
        },
        "required": ["added", "removed", "partial", "fallback", "notes"],
    })
}

/// The `support` of a function or of a feature: one entry for each
/// application.
fn support() -> Value {
    let properties: serde_json::Map<String, Value> = APPS
        .iter()
        .map(|app| ((*app).to_owned(), entry()))
        .collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": APPS,
    })
}

/// The `requires` of a versioned function: the features it needs as well.
fn requires() -> Value {
    json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "feature": { "type": "string" },
                "support": support(),
                "sources": { "type": "array", "items": { "type": "string" } },
            },
            "required": ["feature", "support", "sources"],
        },
    })
}

/// The `safety` of a function: its class and, for a dangerous one, the kind
/// of danger.
fn safety() -> Value {
    json!({
        "type": "object",
        "properties": {
            "class": {
                "type": "string",
                "enum": ["reviewed_safe", "dangerous", "unreviewed"],
            },
            "danger": { "type": ["string", "null"] },
        },
        "required": ["class", "danger"],
    })
}

/// One of the three shapes of a result: the one `known` names and the keys
/// it has.
fn shape(known: &str, required: &[&str]) -> Value {
    json!({
        "properties": { "known": { "const": known } },
        "required": required,
    })
}

/// A result has `name`, `known` and, by `known`, the rest: `versioned` has
/// every key, `baseline` has no `support` and no `requires`, `unknown` has
/// `name` and `known` only.
pub(super) fn output() -> Value {
    let common = ["name", "known", "fileName", "prefix"];
    let tail = ["safety", "sources"];
    let versioned = [&common[..], &["support", "requires"], &tail].concat();
    let baseline = [&common[..], &tail].concat();
    json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "known": { "type": "string", "enum": ["versioned", "baseline", "unknown"] },
            "fileName": { "type": ["string", "null"] },
            "prefix": { "type": ["string", "null"] },
            "support": support(),
            "requires": requires(),
            "safety": safety(),
            "sources": { "type": "array", "items": { "type": "string" } },
        },
        "required": ["name", "known"],
        "oneOf": [
            shape("versioned", &versioned),
            shape("baseline", &baseline),
            shape("unknown", &["name", "known"]),
        ],
    })
}
