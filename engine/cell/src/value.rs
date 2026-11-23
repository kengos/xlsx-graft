//! [`CellValue`] and [`ErrorValue`]: what a cell is written as.

use crate::{Date, DateTime, Elapsed, Time};

/// What a cell holds, as [`write_cell`](crate::write_cell) writes it.
///
/// A [`CellValue::String`] that could be read as a formula, a number or a
/// date (`=A1`, `00123`, `2024-01-01`) is also written as an inline string.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum CellValue<'a> {
    /// A cell without a value, which may carry only a cell format (`s`).
    Empty,
    /// Text, written as an inline string (`t="inlineStr"`).
    String(&'a str),
    /// A number; NaN and the infinities are refused with
    /// [`CellError::NotFinite`](crate::CellError::NotFinite).
    Number(f64),
    /// An integer; one whose absolute value is past 2^53 is refused with
    /// [`CellError::InexactInteger`](crate::CellError::InexactInteger).
    Integer(i64),
    /// A boolean (`t="b"`).
    Bool(bool),
    /// An error value (`t="e"`).
    Error(ErrorValue),
    /// A date, written as a serial date-time in the date system given to
    /// [`write_cell`](crate::write_cell).
    Date(Date),
    /// A time of day, written as the fraction of a day it is.
    Time(Time),
    /// A date and a time of day, written as a serial date-time.
    DateTime(DateTime),
    /// An elapsed time, written as the days it is.
    Elapsed(Elapsed),
    /// A string held by number in the shared strings table (`t="s"`), as
    /// [`SharedStringsWriter::push`](crate::SharedStringsWriter::push)
    /// numbers it.
    SharedString(u64),
}

/// An error value of a cell (ST_CellType `e`; the seven of ECMA-376-1:2016
/// §18.17.3 and `#GETTING_DATA`, which MS-OI29500 adds on §18.17.3,
/// note a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorValue {
    /// `#NULL!`: an intersection of two ranges that do not intersect.
    Null,
    /// `#DIV/0!`: a division by zero.
    DivZero,
    /// `#VALUE!`: an operand of the wrong type.
    Value,
    /// `#REF!`: a reference to a cell that is not there.
    Ref,
    /// `#NAME?`: a name that is not defined.
    Name,
    /// `#NUM!`: a number that is out of range.
    Num,
    /// `#N/A`: a value that is not available.
    NotAvailable,
    /// `#GETTING_DATA`: a value that is still being fetched (MS-OI29500 on
    /// §18.17.3, note a).
    GettingData,
}

impl ErrorValue {
    /// The text of the error value, as a cell's `<v>` holds it.
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::Null => "#NULL!",
            Self::DivZero => "#DIV/0!",
            Self::Value => "#VALUE!",
            Self::Ref => "#REF!",
            Self::Name => "#NAME?",
            Self::Num => "#NUM!",
            Self::NotAvailable => "#N/A",
            Self::GettingData => "#GETTING_DATA",
        }
    }
}
