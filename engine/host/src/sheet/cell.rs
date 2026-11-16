//! [`GridCell`]: one cell of the grid, with its type and value as the CLI's
//! `read` writes them, and the JSON of it.

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use xlsx_graft_diagnostics::CellEcho;
use xlsx_graft_read::{CellRef, Date, DateTime, Elapsed, Time, Value};

/// One cell that holds a value, as a row of the grid lists it.
///
/// Written as `{"ref": "B2", "type": "string", "value": "text"}`, with
/// `"truncated": true` after `value` when the text of a string or an error
/// value was cut at [`MAX_CELL_ECHO`](xlsx_graft_diagnostics::MAX_CELL_ECHO)
/// characters. The `truncated` key is there only when the text was cut, and
/// then its value is always `true`: a reader takes a missing `truncated` as
/// the text being whole.
///
/// `type` and `value` are the CLI's `read` `type` and `value`, other than
/// that a cell with no value is not listed: `string` (the text), `number`
/// (a JSON number), `boolean`, `date` (`2024-01-15`), `time`
/// (`08:30:00.000`), `datetime` (`2024-01-15T08:30:00.000`), `elapsed` (the
/// milliseconds, an integer) and `error` (the text, as `#N/A`). The text of
/// a string or an error value has the control characters other than the tab,
/// the line feed and the carriage return removed, and the characters that
/// reorder or hide a line (see [`CellEcho`]).
#[derive(Debug, Clone, PartialEq)]
pub struct GridCell {
    /// Where the cell stands, written as `ref`.
    pub(crate) reference: CellRef,
    /// What it holds.
    pub(crate) value: GridValue,
}

/// What a listed cell holds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GridValue {
    /// A string.
    String(CellEcho),
    /// A number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// A date.
    Date(Date),
    /// A time of day.
    Time(Time),
    /// A date and a time of day.
    DateTime(DateTime),
    /// Elapsed time.
    Elapsed(Elapsed),
    /// An error value.
    Error(CellEcho),
}

impl GridValue {
    /// The value `value` stands for; `None` for a cell that holds none.
    pub(crate) fn of(value: Value<'_>) -> Option<Self> {
        match value {
            Value::Empty => None,
            Value::String(text) => Some(Self::String(CellEcho::new(text))),
            Value::Number(number) => Some(Self::Number(number)),
            Value::Bool(bool) => Some(Self::Bool(bool)),
            Value::Date(date) => Some(Self::Date(date)),
            Value::Time(time) => Some(Self::Time(time)),
            Value::DateTime(at) => Some(Self::DateTime(at)),
            Value::Elapsed(elapsed) => Some(Self::Elapsed(elapsed)),
            Value::Error(text) => Some(Self::Error(CellEcho::new(text))),
        }
    }

    /// How many characters the text of the value has; 0 for a value that is
    /// no string or error value.
    pub(crate) fn text_chars(&self) -> usize {
        match self {
            Self::String(text) | Self::Error(text) => text.as_str().chars().count(),
            _ => 0,
        }
    }

    /// Whether the text of the value was cut.
    fn truncated(&self) -> bool {
        match self {
            Self::String(text) | Self::Error(text) => text.truncated(),
            _ => false,
        }
    }
}

impl Serialize for GridCell {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let truncated = self.value.truncated();
        let mut map = serializer.serialize_map(Some(if truncated { 4 } else { 3 }))?;
        map.serialize_entry("ref", &format_args!("{}", self.reference))?;
        match &self.value {
            GridValue::String(text) => typed(&mut map, "string", text.as_str())?,
            GridValue::Number(number) => typed(&mut map, "number", number)?,
            GridValue::Bool(bool) => typed(&mut map, "boolean", bool)?,
            GridValue::Date(date) => typed(&mut map, "date", &format_args!("{date}"))?,
            GridValue::Time(time) => typed(&mut map, "time", &format_args!("{time}"))?,
            GridValue::DateTime(at) => typed(&mut map, "datetime", &format_args!("{at}"))?,
            GridValue::Elapsed(elapsed) => typed(&mut map, "elapsed", &elapsed.milliseconds())?,
            GridValue::Error(text) => typed(&mut map, "error", text.as_str())?,
        }
        if truncated {
            map.serialize_entry("truncated", &true)?;
        }
        map.end()
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
