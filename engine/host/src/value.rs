//! A cell's value written as text and as the `type` and `value` of JSON. The
//! CLI's `read` and the host's views write a value with these functions, and
//! the CLI's `sheets` escapes a sheet's name with [`write_escaped`]. The text
//! is for a person and not a contract; the JSON is.

use serde::Serialize;
use serde::ser::SerializeMap;
use std::fmt;
use xlsx_graft_read::Value;

/// Which characters [`write_escaped`] and [`write_value`] escape in a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    /// The backslash, the tab, the line feed and the carriage return,
    /// written `\\`, `\t`, `\n` and `\r`: the text stays on its line and in
    /// its tab-separated column.
    Line,
    /// As [`Line`](Self::Line), and `|` written `\|`: the text stays in its
    /// column of a Markdown table.
    Table,
}

/// Writes `text` to `out` with the characters `escape` names escaped.
///
/// # Errors
///
/// An error of `out`.
pub fn write_escaped(out: &mut dyn fmt::Write, text: &str, escape: Escape) -> fmt::Result {
    for char in text.chars() {
        match char {
            '\\' => out.write_str("\\\\")?,
            '\t' => out.write_str("\\t")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '|' if escape == Escape::Table => out.write_str("\\|")?,
            char => out.write_char(char)?,
        }
    }
    Ok(())
}

/// Writes `value` to `out` for a person to read.
///
/// An empty value is written as nothing. A number is written as Rust writes
/// it when it is 0 or its magnitude is from 1e-5 up to but not including
/// 1e16, and in an exponent's form (`1e16`, `9.9e-6`) otherwise. A boolean is
/// `TRUE` or `FALSE`, as Excel shows one. A date, a time and a date with a
/// time are in ISO 8601 (`2024-01-31T08:30:00.000`). Elapsed time is in hours
/// that do not wrap at a day (`26:03:00.000`). A string or an error value is
/// written by [`write_escaped`] with `escape`.
///
/// # Errors
///
/// An error of `out`.
pub fn write_value(out: &mut dyn fmt::Write, value: Value<'_>, escape: Escape) -> fmt::Result {
    match value {
        Value::Empty => Ok(()),
        Value::String(text) | Value::Error(text) => write_escaped(out, text, escape),
        Value::Number(number) if number == 0.0 || (1e-5..1e16).contains(&number.abs()) => {
            write!(out, "{number}")
        }
        Value::Number(number) => write!(out, "{number:e}"),
        Value::Bool(bool) => out.write_str(if bool { "TRUE" } else { "FALSE" }),
        Value::Date(date) => write!(out, "{date}"),
        Value::Time(time) => write!(out, "{time}"),
        Value::DateTime(at) => write!(out, "{at}"),
        Value::Elapsed(elapsed) => {
            let ms = elapsed.milliseconds();
            let (hours, minutes) = (ms / 3_600_000, ms / 60_000 % 60);
            let (seconds, ms) = (ms / 1_000 % 60, ms % 1_000);
            write!(out, "{hours}:{minutes:02}:{seconds:02}.{ms:03}")
        }
    }
}

/// Writes the entries `type` and `value` of `value` to `map`, the JSON of a
/// cell.
///
/// `type` is `empty` (with a `value` of null), `string` (the text),
/// `number` (a JSON number, with a fraction or an exponent always, so that a
/// reader that tells integers from floats reads a float), `boolean`, `date`
/// (`2024-01-31`), `time` (`08:30:00.000`), `datetime`
/// (`2024-01-31T08:30:00.000`), `elapsed` (the milliseconds, an integer) or
/// `error` (the text, as `#N/A`).
///
/// # Errors
///
/// An error of `map`.
pub fn serialize_value<M: SerializeMap>(map: &mut M, value: Value<'_>) -> Result<(), M::Error> {
    match value {
        Value::Empty => typed(map, "empty", &()),
        Value::String(text) => typed(map, "string", text),
        Value::Number(number) => typed(map, "number", &number),
        Value::Bool(bool) => typed(map, "boolean", &bool),
        Value::Date(date) => typed(map, "date", &format_args!("{date}")),
        Value::Time(time) => typed(map, "time", &format_args!("{time}")),
        Value::DateTime(at) => typed(map, "datetime", &format_args!("{at}")),
        Value::Elapsed(elapsed) => typed(map, "elapsed", &elapsed.milliseconds()),
        Value::Error(text) => typed(map, "error", text),
    }
}

/// Writes `type` and `value` to `map`.
fn typed<M: SerializeMap, V: Serialize + ?Sized>(
    map: &mut M,
    kind: &str,
    value: &V,
) -> Result<(), M::Error> {
    map.serialize_entry("type", kind)?;
    map.serialize_entry("value", value)
}

#[cfg(test)]
mod tests;
