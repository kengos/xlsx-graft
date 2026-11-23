//! xlsx-graft cell: the types a cell's value is made of and the
//! conversions between them and a cell's number or text, shared by reading
//! and writing: the wall-clock values [`Date`], [`Time`], [`DateTime`] and
//! [`Elapsed`] (`clock.rs`), the [`DateSystem`] a serial date-time counts
//! its days in, a cell's position [`CellRef`] (`reference.rs`), why text is
//! not the date, time of day, date-time or cell reference it is written as
//! ([`ValueError`]), and the `_xHHHH_` escapes of ST_Xstring
//! ([`unescape`], [`Xstring`], [`escape`]). A cell's number made one of the
//! wall-clock types is `serial.rs`, and a cell's ISO 8601 text (`t="d"`)
//! `iso.rs`. Writing goes the other way: a cell's value ([`CellValue`]) as
//! its `<c>` element ([`write_cell`], `encode.rs`) and the shared strings
//! part ([`SharedStringsWriter`]).
//!
//! # Numbers as dates and times
//!
//! A number under a date, time, date-time or elapsed time format
//! (`FormatKind` of xlsx-graft-read) is a serial date-time
//! (ECMA-376-1:2016 §18.17.4.1): days from serial 0, the fraction a part
//! of a day. It is made a wall-clock value without a time zone, the
//! workbook's own: §18.17.4.1 says the base dates and the serial
//! date-times stand for local dates and times.
//!
//! - **The day**: serial 0 is 1899-12-30 in the standard's 1900
//!   ([`DateSystem::Base1900`]) and
//!   1904-01-01 in 1904 (§18.17.4.1). In the 1900 that keeps Lotus 1-2-3's
//!   day that never was
//!   ([`DateSystem::Compatibility1900`]),
//!   serial 1 is 1900-01-01, 60 is 1900-02-29, and 61 on are the
//!   standard's 1900 again; serial 1 is its earliest date (ECMA-376-1:2016
//!   Annex L.2.16.9.1, informative), so a date or date-time before day 1
//!   or on day 60 is no date: [`Date::from_serial`] and
//!   [`DateTime::from_serial`] return `None` (xlsx-graft-read then keeps
//!   the cell's number).
//! - **The range**: a serial past 9999-12-31T23:59:59.999, the latest
//!   date-time (§18.17.4), is invalid (§18.17.4.1), and every `from_serial`
//!   returns `None` for it, as for a negative one: the standard counts back
//!   to 0001-01-01, but Excel takes no negative serial date-time
//!   (MS-OI29500 on §18.2.28, note j). Nor is a number that is not finite a
//!   date-time.
//! - **The millisecond**: the serial is rounded to the nearest millisecond
//!   first, half away from zero, then split into the day and the time of
//!   day, so 23:59:59.9995 is the next midnight. The standard's latest
//!   date-time ends at the millisecond, its fractional seconds are written
//!   to three decimals at most (§18.17.4), and MS-OI29500 notes nothing
//!   on §18.17.4, so Office keeps to it: a finer fraction is the binary
//!   number's noise, not a time.
//! - **The kind**: [`Date::from_serial`] drops the time of day,
//!   [`DateTime::from_serial`] keeps both, [`Time::from_serial`] drops the
//!   day, and [`Elapsed::from_serial`] does not wrap at a day. [`Time`] and
//!   [`Elapsed`] count from serial 0, so serial 0 and the fraction of day 0
//!   are times in every date system, `Compatibility1900` included, where
//!   serial 0 is no date (a time of day is a serial from 0 to below 1,
//!   §18.17.4.2), and serial 60 is 60 days of
//!   elapsed time. [`Time::from_serial`] and [`Elapsed::from_serial`] also
//!   return `None` past the latest date-time, counted from serial 0, so the
//!   longest elapsed time is 1,462 days shorter in 1904, whose serial 0 is
//!   that much later.
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
//! # Writing a cell
//!
//! [`write_cell`] writes what the value is and guesses nothing about it:
//!
//! - **The `<c>`**: `r` always; `s` only for a cell format that is not 0;
//!   `t` only for text (`inlineStr`), a boolean (`b`), an error value
//!   (`e`) and a shared string (`s`). A number and a date or time have no
//!   `t`, and `t="d"` and `t="str"` are never written. A string that could
//!   be read as a formula, a number or a date (`=A1`, `00123`,
//!   `2024-01-01`) is also written as an inline string. An empty value is
//!   `<c r="A1" s="3"/>`.
//! - **Checked before writing**: a value `write_cell` refuses (a number that
//!   is not finite, an integer past 2^53, a date, date-time or elapsed time
//!   with no serial date-time in the date system) is checked before the
//!   `<c` is written, so nothing is written to `out`.
//! - **Text**: counted in UTF-16 code units, the text after its 32,767th
//!   code unit is cut at a character boundary, never inside a surrogate
//!   pair, and `write_cell` returns [`Written::Truncated`]. The standard
//!   sets no length; Excel for Mac asks to repair a workbook with a longer
//!   string and cuts it there (`inline_string_too_long` of
//!   xlsx-graft-diagnostics).
//!   `xml:space="preserve"` is written when the cut text starts or ends
//!   with a space, tab, line feed or carriage return, since an XML reader
//!   may remove white space at the edges of an element without it. A
//!   character that is not an XML 1.0 `Char`, and an `_` that would be read
//!   as the start of an escape, are written as `_xHHHH_` ([`escape`], the
//!   inverse of [`unescape`]); the text is then escaped as XML
//!   ([`xml_text`], [`xml_attribute`]), in that order, the order of the
//!   layers: an ST_Xstring value is made first, then written as XML.
//! - **Numbers**: the shortest decimal that reads back to the same `f64`,
//!   plain from 1e-7 up to 1e21 and as `1.5E-8` or `1E+21` outside, and `0`
//!   for `-0.0`. An integer is written whole up to 2^53.
//! - **Dates and times**: the inverse of reading ([`Date::from_serial`] and
//!   the others) in the date system given. A date or date-time before the
//!   system's first day, and an elapsed time `from_serial` does not read,
//!   are refused ([`CellError::DateOutOfRange`]); in
//!   [`DateSystem::Compatibility1900`], which counts the 1900-02-29 that does
//!   not exist as serial 60, serial 60 is never written and the first date
//!   written is 1900-01-01 (serial 1).
//!
//! [`CellValue`], [`ErrorValue`] and [`CellError`] are `#[non_exhaustive]`,
//! so a `match` on them needs a wildcard arm: a kind of value, an error
//! value or a reason a value is refused can be added later. [`Written`] is
//! not, so a `match` on it names both the whole text and the cut one.

mod clock;
mod date_system;
mod encode;
mod error;
mod escape;
mod iso;
mod reference;
mod serial;
mod value;
mod xml;

pub use clock::{Date, DateTime, Elapsed, Time};
pub use date_system::DateSystem;
pub use encode::{SharedStringsWriter, Written, write_cell};
pub use error::{CellError, ValueError};
pub use escape::{Xstring, escape, unescape};
pub use reference::CellRef;
pub use value::{CellValue, ErrorValue};
pub use xml::{xml_attribute, xml_text};
