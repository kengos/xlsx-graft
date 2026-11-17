//! The options of a view as the C ABI takes them: a JSON object, read as
//! untrusted input of at most [`MAX_REQUEST_BYTES`] bytes.
//!
//! The options of a tree have no key; the options of `describe_sheet` have
//! the keys of [`SheetRequest::ARGUMENTS`]. Empty text, text of only
//! white space and `null` are the same as `{}`: every default holds.

use crate::input::MAX_REQUEST_BYTES;
use crate::sheet::{SheetArgumentsError, SheetRequest};
use serde_json::{Map, Value};
use thiserror::Error;
use xlsx_graft_diagnostics::Echo;

/// Why the options of a view are not a JSON object of the keys the view
/// takes, or do not make a sheet request. This type separates a fault of the
/// options from a failure to read the workbook; the message is English for a
/// person and may change.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ViewOptionsError {
    /// The options are larger than [`MAX_REQUEST_BYTES`].
    #[error("the options are larger than {} bytes", MAX_REQUEST_BYTES)]
    TooLarge,
    /// The options are not JSON.
    #[error("the options are not JSON: {0}")]
    NotJson(String),
    /// The options are JSON, but not an object.
    #[error("the options are not a JSON object")]
    NotObject,
    /// The options have a key the view does not take, which is quoted as a
    /// diagnostic quotes a name (cleaned and clipped).
    #[error("the options have a key the view does not take: {0}")]
    UnknownKey(String),
    /// The values of the keys of `describe_sheet` are refused by
    /// [`SheetRequest::from_arguments`].
    #[error(transparent)]
    Arguments(#[from] SheetArgumentsError),
}

/// Checks the options of a tree, which takes no key: empty text, text of
/// only white space, `null` and `{}` are the options with every default.
///
/// # Errors
///
/// [`ViewOptionsError`] when `json` is larger than [`MAX_REQUEST_BYTES`],
/// is not JSON, is not an object, or has a key.
pub fn tree_options(json: &[u8]) -> Result<(), ViewOptionsError> {
    object(json, &[]).map(drop)
}

/// The [`SheetRequest`] the options of `describe_sheet` make: an object of
/// the keys in [`SheetRequest::ARGUMENTS`], each of which may be absent.
/// Empty text, text of only white space and `null` are the default request.
///
/// # Errors
///
/// [`ViewOptionsError`] when `json` is larger than [`MAX_REQUEST_BYTES`], is
/// not JSON, is not an object, has a key that is not in
/// [`SheetRequest::ARGUMENTS`], or has keys that [`SheetRequest::from_arguments`]
/// refuses ([`ViewOptionsError::Arguments`]).
pub fn sheet_options(json: &[u8]) -> Result<SheetRequest, ViewOptionsError> {
    let map = object(json, &SheetRequest::ARGUMENTS)?.unwrap_or_default();
    Ok(SheetRequest::from_arguments(&map)?)
}

/// The object `json` holds, or `None` when it is empty, only white space or
/// `null`; its keys are all in `keys`.
fn object(json: &[u8], keys: &[&str]) -> Result<Option<Map<String, Value>>, ViewOptionsError> {
    if json.len() > MAX_REQUEST_BYTES {
        return Err(ViewOptionsError::TooLarge);
    }
    if json.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    let value: Value = serde_json::from_slice(json)
        .map_err(|error| ViewOptionsError::NotJson(error.to_string()))?;
    let map = match value {
        Value::Object(map) => map,
        Value::Null => return Ok(None),
        _ => return Err(ViewOptionsError::NotObject),
    };
    if let Some(key) = map.keys().find(|key| !keys.contains(&key.as_str())) {
        return Err(ViewOptionsError::UnknownKey(Echo::new(key).to_string()));
    }
    Ok(Some(map))
}

#[cfg(test)]
mod tests;
