//! The types a diagnostic argument may have, and how an argument is written
//! into the `args` object.

use crate::{Echo, Limit};
use serde::Serialize;
use serde::ser::SerializeMap;
use std::fmt;

/// A type a [`Diagnostic`](crate::Diagnostic) field may have: what a
/// consumer can render from without parsing engine prose.
///
/// The set is closed on purpose: `u64`, `bool`, [`Echo`] (never a raw
/// `String`, so no unsanitized text reaches a diagnostic), [`Limit`] (written
/// as its bare number) and reason enums.
/// A reason enum derives `Serialize` with `#[serde(rename_all = "snake_case")]`,
/// implements `Display` with the same strings, and implements this trait.
pub(crate) trait ArgValue: Serialize + fmt::Display {}

impl ArgValue for u64 {}
impl ArgValue for bool {}
impl ArgValue for Echo {}
impl ArgValue for Limit {}

/// Writes the argument `field` (a Rust field name) into `map` under its
/// camelCase wire key.
pub(crate) fn entry<M: SerializeMap, T: ArgValue>(
    map: &mut M,
    field: &str,
    value: &T,
) -> Result<(), M::Error> {
    map.serialize_entry(&wire_key(field), value)
}

/// The camelCase wire key of a snake_case field name: `relationship_id`
/// becomes `relationshipId`.
pub(crate) fn wire_key(field: &str) -> String {
    let mut key = String::with_capacity(field.len());
    let mut upper = false;
    for c in field.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            key.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            key.push(c);
        }
    }
    key
}

#[cfg(test)]
mod tests;
