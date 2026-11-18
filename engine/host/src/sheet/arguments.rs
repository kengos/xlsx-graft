//! [`SheetArgumentsError`] and [`SheetRequest::from_arguments`]: a
//! [`SheetRequest`] read from the keys of a JSON object (`sheet`,
//! `sheetIndex`, `range`, `maxRows`, `maxColumns`), the arguments of the MCP
//! server's `describe_sheet` tool and the options of the C ABI's.

use super::{SheetRequest, SheetRequestError};
use serde_json::{Map, Value};
use thiserror::Error;
use xlsx_graft_read::SheetChoice;

/// Why the keys of a JSON object are not a [`SheetRequest`]. This type
/// separates a fault of the keys from a failure to read the workbook (the MCP
/// server answers invalid params, the C ABI answers
/// `XLSX_GRAFT_ERR_REQUEST_INVALID`).
/// The message is English for a person and may change.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SheetArgumentsError {
    /// `sheet` or `range` (the key is named) is not a string; `null` is not
    /// a string.
    #[error("describe_sheet's {0} must be a string")]
    NotString(&'static str),
    /// `sheetIndex` is not a JSON integer from 0 to the platform's `usize`
    /// maximum: a negative number, a number with a fraction or an exponent
    /// such as `5.0`, a number above that maximum, a string, or `null`.
    #[error("describe_sheet's sheetIndex must be an integer from 0")]
    NotIndex,
    /// `maxRows` or `maxColumns` (the key is named) is not a JSON integer
    /// from 0 to 2^64 - 1: a negative number, a number with a fraction or an
    /// exponent such as `5.0`, a number above 2^64 - 1, a string, or `null`.
    /// 0 and a number above the key's bound but not above 2^64 - 1 pass this
    /// check, and [`SheetRequest::new`] refuses them. `max` is the most the key takes,
    /// which the message gives.
    #[error("describe_sheet's {key} must be an integer from 1 to {max}")]
    NotCount {
        /// The key: `maxRows` or `maxColumns`.
        key: &'static str,
        /// The most the key takes.
        max: u64,
    },
    /// Both `sheet` and `sheetIndex` are given.
    #[error("describe_sheet takes sheet or sheetIndex, not both")]
    BothSheets,
    /// The keys have the right types, but [`SheetRequest::new`] refuses their
    /// values.
    #[error(transparent)]
    Request(#[from] SheetRequestError),
}

impl SheetRequest {
    /// The keys [`from_arguments`](Self::from_arguments) reads.
    pub const ARGUMENTS: [&'static str; 5] =
        ["sheet", "sheetIndex", "range", "maxRows", "maxColumns"];

    /// The request the keys of `map` make: `sheet`, `sheetIndex`, `range`,
    /// `maxRows` and `maxColumns`, each of which may be absent. Any other key
    /// is not read; the caller decides what a key it does not know means.
    ///
    /// # Errors
    ///
    /// [`SheetArgumentsError`], the first fault in this order: the type of
    /// `sheet`, the type of `sheetIndex`, both of them given, the type of
    /// `range`, the type of `maxRows`, the type of `maxColumns`, and then the
    /// faults of [`SheetRequest::new`]. A key of another type, `null`
    /// included, is a type fault.
    pub fn from_arguments(map: &Map<String, Value>) -> Result<Self, SheetArgumentsError> {
        let name = text(map, "sheet")?;
        let index = sheet_index(map)?;
        let choice = match (name, index) {
            (Some(_), Some(_)) => return Err(SheetArgumentsError::BothSheets),
            (Some(name), None) => Some(SheetChoice::Name(name)),
            (None, Some(index)) => Some(SheetChoice::Index(index)),
            (None, None) => None,
        };
        let range = text(map, "range")?;
        let max_rows = count(map, "maxRows", SheetRequest::HARD_MAX_ROWS.into())?;
        let max_columns = count(map, "maxColumns", SheetRequest::HARD_MAX_COLUMNS.into())?;
        Ok(SheetRequest::new(choice, range, max_rows, max_columns)?)
    }
}

/// The string `key` holds, or `None` when `map` has no such key.
fn text<'a>(
    map: &'a Map<String, Value>,
    key: &'static str,
) -> Result<Option<&'a str>, SheetArgumentsError> {
    match map.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .map(Some)
            .ok_or(SheetArgumentsError::NotString(key)),
    }
}

/// The `sheetIndex` of `map`: an integer from 0, or `None` when it has none.
fn sheet_index(map: &Map<String, Value>) -> Result<Option<usize>, SheetArgumentsError> {
    match map.get("sheetIndex") {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .map(Some)
            .ok_or(SheetArgumentsError::NotIndex),
    }
}

/// The non-negative integer `key` (`maxRows` or `maxColumns`) holds, or
/// `None` when `map` has no such key. Only the type is checked: that the
/// value is from 1 to `max` is checked by [`SheetRequest::new`]; `max` is
/// used only in the message.
fn count(
    map: &Map<String, Value>,
    key: &'static str,
    max: u64,
) -> Result<Option<u64>, SheetArgumentsError> {
    match map.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or(SheetArgumentsError::NotCount { key, max }),
    }
}

#[cfg(test)]
mod tests;
