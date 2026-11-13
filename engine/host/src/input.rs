//! Where a setting comes from, and reading it into a JSON value under a
//! size bound.

use crate::walk::{KeyPath, Reporter};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use xlsx_graft_diagnostics::SettingReason;

/// The most bytes a policy or the rules may take as JSON text passed as
/// [`Input::Bytes`]; one inside a request is bounded by
/// [`MAX_REQUEST_BYTES`].
pub const MAX_SETTING_BYTES: usize = 1 << 20;

/// The most bytes a request (a policy and the rules) may take as JSON text.
pub const MAX_REQUEST_BYTES: usize = 2 << 20;

/// A setting as its host holds it.
#[derive(Debug, Clone, Copy)]
pub enum Input<'a> {
    /// There is none: every default holds.
    Absent,
    /// JSON text: empty or only whitespace, or `null`, is no setting at
    /// all. At most
    /// [`MAX_SETTING_BYTES`], and nested at most as deep as `serde_json`
    /// reads (128).
    Bytes(&'a [u8]),
    /// JSON already read, such as the object a command line is put into;
    /// `null` is no setting at all.
    Value(&'a Value),
}

/// The bytes of the setting file at `path`, for [`Input::Bytes`]: up to one
/// byte more than [`MAX_SETTING_BYTES`], so that a larger file is reported as
/// too large (`policy_invalid`, reason `too_large`) without being read
/// whole.
///
/// # Errors
///
/// The [`io::Error`] of opening or reading the file.
pub fn read_setting(path: &Path) -> io::Result<Vec<u8>> {
    let limit = u64::try_from(MAX_SETTING_BYTES).map_or(u64::MAX, |max| max.saturating_add(1));
    let mut bytes = Vec::new();
    File::open(path)?.take(limit).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The JSON `input` holds, or `None` with the fault reported.
pub(crate) fn read<'a>(input: Input<'a>, reporter: &mut Reporter<'_>) -> Option<Cow<'a, Value>> {
    let empty = || Cow::Owned(Value::Object(Map::new()));
    match input {
        Input::Absent => Some(empty()),
        Input::Value(Value::Null) => Some(empty()),
        Input::Value(value) => Some(Cow::Borrowed(value)),
        Input::Bytes(bytes) if bytes.len() > MAX_SETTING_BYTES => {
            reporter.push(
                &KeyPath::default(),
                SettingReason::TooLarge,
                &MAX_SETTING_BYTES.to_string(),
            );
            None
        }
        Input::Bytes(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => Some(empty()),
        Input::Bytes(bytes) => match serde_json::from_slice(bytes) {
            Ok(Value::Null) => Some(empty()),
            Ok(value) => Some(Cow::Owned(value)),
            Err(error) => {
                let at = format!("line {} column {}", error.line(), error.column());
                reporter.push(&KeyPath::default(), SettingReason::Malformed, &at);
                None
            }
        },
    }
}

#[cfg(test)]
mod tests;
