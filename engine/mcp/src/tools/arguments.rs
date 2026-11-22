//! The `arguments` of a tool call that every tool shares: an object with one
//! required string key (the first of the keys the tool takes) and no key the
//! tool does not take.

use crate::rpc::{INVALID_PARAMS, RpcError, clip};
use serde_json::{Map, Value};

/// The `arguments` of a call to a tool: the value of the required key and the
/// object that holds it and every other key.
pub(super) struct Arguments<'a> {
    /// The value of the required key (the first of the tool's keys): the
    /// workbook `path` of a workbook tool, the function `name` of
    /// `describe_function`.
    pub(super) required: &'a str,
    /// Every key of `arguments`, the required key included; each key is one
    /// the tool takes.
    pub(super) map: &'a Map<String, Value>,
}

/// Reads the `arguments` of a call to `tool`, which takes `keys` (the
/// required string key first). `keys` is not empty: every tool has a required
/// key.
///
/// # Errors
///
/// A protocol error (`INVALID_PARAMS`) when `arguments` is not an object with
/// a string value for the first of `keys`, and, when it is, when it holds a
/// key that is not in `keys`. A key is never silently dropped.
pub(super) fn read<'a>(
    arguments: &'a Value,
    tool: &str,
    keys: &[&str],
) -> Result<Arguments<'a>, RpcError> {
    let required = keys.first().copied().unwrap_or_default();
    let Some(map) = arguments.as_object() else {
        return Err(needs(tool, required));
    };
    let Some(value) = map.get(required).and_then(Value::as_str) else {
        return Err(needs(tool, required));
    };
    if let Some(key) = map.keys().find(|key| !keys.contains(&key.as_str())) {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("{tool} takes {}, not {}", list(keys), clip(key)),
        ));
    }
    Ok(Arguments {
        required: value,
        map,
    })
}

/// The protocol error of a call to `tool` without a string `key`.
fn needs(tool: &str, key: &str) -> RpcError {
    RpcError::new(INVALID_PARAMS, format!("{tool} needs a string `{key}`"))
}

/// `keys` joined with `, `, and ` and ` between the last two.
fn list(keys: &[&str]) -> String {
    match keys.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => keys.join(", "),
    }
}
