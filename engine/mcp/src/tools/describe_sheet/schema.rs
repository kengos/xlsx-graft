//! The JSON Schemas of the `describe_sheet` tool: its arguments and the
//! `structuredContent` of its result.

use crate::tools::schema;
use serde_json::{Value, json};
use xlsx_graft_host::SheetRequest;

/// `{ path, sheet?, sheetIndex?, range?, maxRows?, maxColumns? }` and nothing
/// else.
pub(super) fn input() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "The workbook to describe. A relative path is read from the server's working directory.",
            },
            "sheet": {
                "type": "string",
                "description": "The name of the sheet, letter case aside. Not with `sheetIndex`. Without either, the first sheet.",
            },
            "sheetIndex": {
                "type": "integer",
                "minimum": 0,
                "description": "The index of the sheet in the workbook's list, from 0, hidden sheets included. Not with `sheet`.",
            },
            "range": {
                "type": "string",
                "description": "An A1 range such as `A1:H50`, or one cell such as `B2`. The window starts at its top left cell and does not go past it. Without it, the whole sheet.",
            },
            "maxRows": {
                "type": "integer",
                "minimum": 1,
                "maximum": SheetRequest::HARD_MAX_ROWS,
                "description": format!("The most rows to show. The default is {}.", SheetRequest::DEFAULT_MAX_ROWS),
            },
            "maxColumns": {
                "type": "integer",
                "minimum": 1,
                "maximum": SheetRequest::HARD_MAX_COLUMNS,
                "description": format!("The most columns to show. The default is {}.", SheetRequest::DEFAULT_MAX_COLUMNS),
            },
        },
        "required": ["path"],
        "additionalProperties": false,
    })
}

/// The `sheet` of a result: the sheet that was read, or `null`.
fn sheet() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "index": { "type": "integer" },
            "name": { "type": "string" },
            "kind": {
                "type": "string",
                "enum": ["worksheet", "chartsheet", "dialogsheet", "macrosheet", "unknown"],
            },
            "state": { "type": "string", "enum": ["visible", "hidden", "veryHidden"] },
        },
        "required": ["index", "name", "kind", "state"],
    })
}

/// A row: its number and the cells that hold a value. A cell's `value` has
/// no type here: it is a string, a number, a boolean or whatever its `type`
/// says.
fn row() -> Value {
    json!({
        "type": "object",
        "properties": {
            "row": { "type": "integer" },
            "cells": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "ref": { "type": "string" },
                        "type": { "type": "string" },
                        "value": {},
                        "truncated": { "type": "boolean" },
                    },
                    "required": ["ref", "type", "value"],
                },
            },
        },
        "required": ["row", "cells"],
    })
}

/// The result: `ok`, `error`, `diagnostics`, `sheet`, `dimension`, `used`,
/// `shown`, `truncated`, `complete`, `rows` and `merges`.
pub(super) fn output() -> Value {
    let range = json!({ "type": ["string", "null"] });
    json!({
        "type": "object",
        "properties": {
            "ok": { "type": "boolean" },
            "error": schema::error(),
            "diagnostics": schema::diagnostics(),
            "sheet": sheet(),
            "dimension": range,
            "used": range,
            "shown": range,
            "truncated": {
                "type": "object",
                "properties": {
                    "rows": { "type": "boolean" },
                    "columns": { "type": "boolean" },
                    "merges": { "type": "boolean" },
                },
                "required": ["rows", "columns", "merges"],
            },
            "complete": { "type": "boolean" },
            "rows": { "type": "array", "items": row() },
            "merges": { "type": "array", "items": { "type": "string" } },
        },
        "required": [
            "ok",
            "error",
            "diagnostics",
            "sheet",
            "dimension",
            "used",
            "shown",
            "truncated",
            "complete",
            "rows",
            "merges",
        ],
    })
}
