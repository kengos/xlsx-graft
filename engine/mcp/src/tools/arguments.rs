//! The `arguments` of a tool call that every tool with a workbook `path`
//! shares: an object with a string `path` and no key the tool does not take.

use crate::rpc::{INVALID_PARAMS, RpcError, clip};
use serde_json::{Map, Value};

/// The `arguments` of a call to a tool: the `path` and the object that holds
/// it and every other key.
pub(super) struct Arguments<'a> {
    /// The workbook the call names.
    pub(super) path: &'a str,
    /// Every key of `arguments`, `path` included; each key is one the tool
    /// takes.
    pub(super) map: &'a Map<String, Value>,
}

/// Reads the `arguments` of a call to `tool`, which takes `keys` (`path`
/// first).
///
/// # Errors
///
/// A protocol error (`INVALID_PARAMS`) when `arguments` is not an object with
/// a string `path`, and, when it is, when it holds a key that is not in
/// `keys`. A key is never silently dropped.
pub(super) fn read<'a>(
    arguments: &'a Value,
    tool: &str,
    keys: &[&str],
) -> Result<Arguments<'a>, RpcError> {
    let Some(map) = arguments.as_object() else {
        return Err(needs_path(tool));
    };
    let Some(path) = map.get("path").and_then(Value::as_str) else {
        return Err(needs_path(tool));
    };
    if let Some(key) = map.keys().find(|key| !keys.contains(&key.as_str())) {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("{tool} takes {}, not {}", list(keys), clip(key)),
        ));
    }
    Ok(Arguments { path, map })
}

/// The protocol error of a call to `tool` without a string `path`.
fn needs_path(tool: &str) -> RpcError {
    RpcError::new(INVALID_PARAMS, format!("{tool} needs a string `path`"))
}

/// `keys` joined with `, `, and ` and ` between the last two.
fn list(keys: &[&str]) -> String {
    match keys.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => keys.join(", "),
    }
}
