//! [`ValueError`] and [`CellError`]: why text is not the value it stands
//! for, and why a value cannot be written as a cell.

use std::io;
use thiserror::Error;

/// Why text is not the value it stands for: a cell's ISO 8601 date, time or
/// date-time ([`Date::from_iso`](crate::Date::from_iso),
/// [`Time::from_iso`](crate::Time::from_iso),
/// [`DateTime::from_iso`](crate::DateTime::from_iso)) or its reference
/// ([`CellRef::parse`](crate::CellRef::parse)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ValueError {
    /// The text is not in the form the value is written in.
    #[error("not in the form of the value")]
    Form,
    /// The text is in the form, but a part of it is out of its range.
    #[error("out of the value's range")]
    Range,
}

/// Why a value cannot be written as a cell ([`write_cell`](crate::write_cell),
/// [`SharedStringsWriter`](crate::SharedStringsWriter)). Every error but
/// [`CellError::Io`] is the value's, found before anything is written.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CellError {
    /// A number that is not finite (NaN or an infinity), which a cell
    /// cannot hold.
    #[error("the number is not finite")]
    NotFinite,
    /// An integer whose absolute value is past 2^53: a cell's number is an
    /// `f64`, which cannot hold every integer past 2^53.
    #[error("the integer is past 2^53")]
    InexactInteger,
    /// A date, date-time or elapsed time that has no serial date-time in
    /// the date system: before its first day, or past 9999-12-31T23:59:59.999.
    #[error("the date or time has no serial date-time in the date system")]
    DateOutOfRange,
    /// Writing to `out` failed.
    #[error("writing failed: {0}")]
    Io(#[from] io::Error),
}
