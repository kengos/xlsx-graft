//! A cell's ISO 8601 date-time (`t="d"`) in the three forms §18.17.4
//! allows: a calendar date, a time of day, or both joined by `T`.
//! [`Value::from_iso`] tells the three forms apart by the bytes at offsets
//! 2, 4 and 10, and converts the text with [`Date::from_iso`],
//! [`Time::from_iso`] or [`DateTime::from_iso`] of xlsx-graft-cell.

use super::{Value, ValueError};
use crate::{Date, DateTime, Time};

impl Value<'_> {
    /// The value of a cell's ISO 8601 text: `YYYY-MM-DD` a [`Date`],
    /// `hh:mm`, `hh:mm:ss` or `hh:mm:ss.s…` a [`Time`], and a date and a
    /// time joined by `T` a [`DateTime`], the seconds rounded to the
    /// millisecond.
    ///
    /// # Errors
    ///
    /// [`ValueError::Form`] for text in none of the forms, a time zone
    /// included; [`ValueError::Range`] for a field the calendar or the
    /// clock does not have (month 13, 02-30, hour 24, second 60), year 0,
    /// or a date-time rounded past 9999-12-31T23:59:59.999.
    pub fn from_iso(text: &str) -> Result<Self, ValueError> {
        let bytes = text.as_bytes();
        match (bytes.get(2), bytes.get(4), bytes.get(10)) {
            (Some(b':'), _, _) => Time::from_iso(text).map(Value::Time),
            (_, Some(b'-'), None) => Date::from_iso(text).map(Value::Date),
            (_, Some(b'-'), Some(b'T')) => DateTime::from_iso(text).map(Value::DateTime),
            _ => Err(ValueError::Form),
        }
    }
}
