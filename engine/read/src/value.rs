//! [`Value`]: what a cell holds, typed: a string, a number, a boolean, a
//! date, a time of day, a date-time, an elapsed time, an error value, or
//! nothing. A string and an error value borrow their text, so a row can
//! hold its cells' text in one buffer it reuses and hand out values that
//! point into it. The wall-clock types are `clock.rs`; a cell's number made
//! one is `serial.rs`, and a cell's ISO 8601 text (`t="d"`) `iso.rs`.
//!
//! # Numbers as dates and times
//!
//! A number under a date, time, date-time or elapsed time format
//! ([`FormatKind`](crate::FormatKind)) is a serial date-time
//! (ECMA-376-1:2016 §18.17.4.1): days from serial 0, the fraction a part
//! of a day. It is made a wall-clock value without a time zone, the
//! workbook's own: §18.17.4.1 says the base dates and the serial
//! date-times stand for local dates and times.
//!
//! - **The day**: serial 0 is 1899-12-30 in the standard's 1900
//!   ([`DateSystem::Base1900`](crate::DateSystem::Base1900)) and
//!   1904-01-01 in 1904 (§18.17.4.1). In the 1900 that keeps Lotus 1-2-3's
//!   day that never was
//!   ([`DateSystem::Compatibility1900`](crate::DateSystem::Compatibility1900)),
//!   serial 1 is 1900-01-01, 60 is 1900-02-29, and 61 on are the
//!   standard's 1900 again; serial 1 is its earliest date (ECMA-376-1:2016
//!   Annex L.2.16.9.1, informative), so a date or date-time before day 1
//!   or on day 60 is no date and stays a number.
//! - **The range**: a serial past 9999-12-31T23:59:59.999, the latest
//!   date-time (§18.17.4), is invalid (§18.17.4.1) and stays a number, as
//!   does a negative one: the standard counts back to 0001-01-01, but
//!   Excel takes no negative serial date-time (MS-OI29500 on §18.2.28,
//!   note j). Nor is a number that is not finite a date-time.
//! - **The millisecond**: the serial is rounded to the nearest millisecond
//!   first, half away from zero, then split into the day and the time of
//!   day, so 23:59:59.9995 is the next midnight. The standard's latest
//!   date-time ends at the millisecond, its fractional seconds are written
//!   to three decimals at most (§18.17.4), and MS-OI29500 notes nothing
//!   on §18.17.4, so Office keeps to it: a finer fraction is the binary
//!   number's noise, not a time.
//! - **The kind**: a date format makes a [`Date`], the time of day
//!   dropped; a date-time format a [`DateTime`]. A time format makes a
//!   [`Time`], the day dropped, and an elapsed time format an [`Elapsed`],
//!   which does not wrap at a day: both count from serial 0, so 0 and the
//!   fraction of day 0 are times in either 1900 (a time of day is a serial
//!   from 0 to below 1, §18.17.4.2), and serial 60 is 60 days of elapsed
//!   time. Both still hold to the range, counted from serial 0 to the
//!   latest date-time, so the longest elapsed time is 1,462 days shorter
//!   in 1904, whose serial 0 is that much later. Any other format leaves
//!   the number as it is, the unknown one included.
//!
//! # ISO 8601 text
//!
//! A cell of `t="d"` holds a date, a time of day or both as §18.17.4
//! writes them: the complete extended forms `YYYY-MM-DD`, `hh:mm:ss` (the
//! seconds and their decimals optional, the decimals only after seconds)
//! and the two joined by `T`, with no time zone, years 0001 to 9999, hours
//! 00 to 23 and seconds 00 to 59. Decimals of a second past the third are
//! taken, rounded to the millisecond as a serial is; a time of day rounded
//! to midnight wraps, as a time format drops the day.
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

mod clock;
mod iso;
mod serial;

pub use clock::{Date, DateTime, Elapsed, Time};
use thiserror::Error;

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

/// Why text is not the value it stands for: a cell's ISO 8601 date-time
/// ([`Value::from_iso`]) or its reference ([`CellRef::parse`](crate::CellRef::parse)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ValueError {
    /// The text is not in the form the value is written in.
    #[error("not in the form of the value")]
    Form,
    /// The text is in the form, but a part of it is out of its range.
    #[error("out of the value's range")]
    Range,
}
