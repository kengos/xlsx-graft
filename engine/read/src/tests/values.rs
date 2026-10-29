//! Cell values, driven through `Value::from_serial`, `Value::from_iso` and
//! `CellRef::parse`: `serial.rs` covers a number made a date or time in
//! each date system, `iso.rs` the ISO 8601 text of a `t="d"` cell, and
//! `reference.rs` a cell's `r`.

mod iso;
mod reference;
mod serial;

use crate::Value;

/// `value` as ISO 8601 text, or `None` when it is no date or time.
fn iso(value: Value<'_>) -> Option<String> {
    match value {
        Value::Date(date) => Some(date.to_string()),
        Value::Time(time) => Some(time.to_string()),
        Value::DateTime(date_time) => Some(date_time.to_string()),
        _ => None,
    }
}
