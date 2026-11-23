//! [`Value`]: what a cell holds, typed: a string, a number, a boolean, a
//! date, a time of day, a date-time, an elapsed time, an error value, or
//! nothing. A string and an error value borrow their text, so a row can
//! hold its cells' text in one buffer it reuses and hand out values that
//! point into it. The wall-clock types come from xlsx-graft-cell: `serial.rs`
//! picks the wall-clock type by the cell's format and `iso.rs` by the form
//! of the text, and xlsx-graft-cell converts the number or text.
//!
//! The rules a cell's number or ISO 8601 text is made a date or time by are
//! in the docs of [`xlsx_graft_cell`].
//!
//! # Numbers as dates and times
//!
//! The format of a cell's number
//! ([`FormatKind`](crate::FormatKind)) decides the kind: a date format
//! makes a [`Date`], the time of day dropped; a date-time format a
//! [`DateTime`]; a time format a [`Time`], the day dropped; and an
//! elapsed time format an [`Elapsed`]. Any other format leaves the number
//! as it is, the unknown one included.
//!
//! # Error values
//!
//! An error value is its text as written ([`Value::Error`]): `#DIV/0!`,
//! `#N/A`, `#NAME?`, `#NULL!`, `#NUM!`, `#REF!`, `#VALUE!` and
//! `#GETTING_DATA` (§18.17.3, and MS-OI29500 on §18.17.3, note a). One
//! not among them is not refused and not made a string: Excel adds error
//! values the standard does not list, and a reader that refused them would
//! fail on a newer workbook, while one that made them strings would hide
//! that the cell is an error.

mod iso;
mod serial;

pub use xlsx_graft_cell::{Date, DateTime, Elapsed, Time, ValueError};

/// What a cell holds, typed. A string and an error value borrow their
/// text from whatever holds the cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    /// No value: an empty cell, or one without a cached value.
    Empty,
    /// A string.
    String(&'a str),
    /// A number, under a format that makes no date or time of it, or one
    /// that is no date-time.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// A date, under a date format.
    Date(Date),
    /// A time of day, under a time format.
    Time(Time),
    /// A date and a time of day, under a date-time format.
    DateTime(DateTime),
    /// Elapsed time, under an elapsed time format (`[h]:mm`).
    Elapsed(Elapsed),
    /// An error value, as written (`#N/A`).
    Error(&'a str),
}
