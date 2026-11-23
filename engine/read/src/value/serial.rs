//! A cell's number as its format shows it: a serial date-time (§18.17.4.1)
//! made a [`Date`], [`Time`], [`DateTime`] or [`Elapsed`] in a
//! [`DateSystem`], or left a number where it is none.

use super::Value;
use crate::{Date, DateSystem, DateTime, Elapsed, FormatKind, Time};

impl Value<'_> {
    /// The value of a cell's number `serial` under a format of `kind`, its
    /// day counted in `system`: a date-time for a date, time or elapsed
    /// time format, and the number as it is for any other format or when
    /// the number is no date-time of Excel's: negative, not finite, past
    /// 9999-12-31T23:59:59.999, or, as a date or date-time in
    /// [`DateSystem::Compatibility1900`], before day 1 or on day 60.
    #[must_use]
    pub fn from_serial(serial: f64, kind: FormatKind, system: DateSystem) -> Self {
        let date_time = match kind {
            FormatKind::Date => Date::from_serial(serial, system).map(Value::Date),
            FormatKind::DateTime => DateTime::from_serial(serial, system).map(Value::DateTime),
            FormatKind::Time => Time::from_serial(serial, system).map(Value::Time),
            FormatKind::Duration => Elapsed::from_serial(serial, system).map(Value::Elapsed),
            FormatKind::General | FormatKind::Number | FormatKind::Text | FormatKind::Unknown => {
                None
            }
        };
        date_time.unwrap_or(Value::Number(serial))
    }
}
