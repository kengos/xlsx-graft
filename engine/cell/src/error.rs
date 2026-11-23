//! [`ValueError`]: why text is not the value it stands for.

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
