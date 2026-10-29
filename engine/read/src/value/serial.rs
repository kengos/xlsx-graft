//! A cell's number as its format shows it: a serial date-time (§18.17.4.1)
//! made a [`Date`], [`Time`], [`DateTime`] or [`Elapsed`] in a
//! [`DateSystem`], or left a number where it is none.

use super::Value;
use super::clock::{DAY_MS, Date, DateTime, Elapsed, LAST_DAY, Time};
use crate::{DateSystem, FormatKind};

/// Past every serial date-time of either date system, and small enough
/// that its milliseconds are exact in an `i64`.
const PAST_LAST: f64 = 3_000_000.0;

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
            FormatKind::Date => instant(serial, system)
                .and_then(|ms| Date::from_day(ms.div_euclid(DAY_MS)).map(Value::Date)),
            FormatKind::DateTime => {
                instant(serial, system).and_then(|ms| DateTime::from_ms(ms).map(Value::DateTime))
            }
            FormatKind::Time => {
                millis(serial, system).and_then(|ms| Time::from_ms(ms).map(Value::Time))
            }
            FormatKind::Duration => {
                millis(serial, system).and_then(|ms| Elapsed::from_ms(ms).map(Value::Elapsed))
            }
            FormatKind::General | FormatKind::Number | FormatKind::Text | FormatKind::Unknown => {
                None
            }
        };
        date_time.unwrap_or(Value::Number(serial))
    }
}

/// `serial` rounded to the millisecond, from serial 0; `None` when it is
/// negative, not a number, or past 9999-12-31T23:59:59.999 once rounded.
fn millis(serial: f64, system: DateSystem) -> Option<i64> {
    if !(0.0..PAST_LAST).contains(&serial) {
        return None;
    }
    // A serial in 0..PAST_LAST rounds to whole milliseconds well inside an
    // i64, so the cast neither saturates nor truncates.
    let ms = (serial * 86_400_000.0).round() as i64;
    let last = (LAST_DAY + 1 - base(system)) * DAY_MS;
    (ms < last).then_some(ms)
}

/// The instant of `serial`, in milliseconds from 0001-01-01T00:00:00.000;
/// `None` when `serial` is no date-time ([`millis`]), or in 1900 with its
/// day that never was falls before day 1 or on day 60.
fn instant(serial: f64, system: DateSystem) -> Option<i64> {
    let ms = millis(serial, system)?;
    let day = ms / DAY_MS;
    let shift = match system {
        DateSystem::Compatibility1900 if day == 0 || day == 60 => return None,
        // 1900-01-01 is day 1 and 1900-02-28 day 59: one day later than
        // the standard's 1900 counts them.
        DateSystem::Compatibility1900 if day < 60 => DAY_MS,
        _ => 0,
    };
    Some(base(system) * DAY_MS + ms + shift)
}

/// The day serial 0 stands for, from 0001-01-01: 1899-12-30, as the
/// standard's 1900 has it and as Lotus's 1900 lines up from 1900-03-01, or
/// 1904-01-01.
fn base(system: DateSystem) -> i64 {
    match system {
        DateSystem::Compatibility1900 | DateSystem::Base1900 => 693_593,
        DateSystem::Base1904 => 695_055,
    }
}
