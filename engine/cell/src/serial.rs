//! A cell's number as a serial date-time (§18.17.4.1) made a [`Date`],
//! [`Time`], [`DateTime`] or [`Elapsed`] in a [`DateSystem`], and each of
//! them made that number again.

use crate::DateSystem;
use crate::clock::{DAY_MS, Date, DateTime, Elapsed, LAST_DAY, Time};

/// Past every serial date-time of either date system, and small enough
/// that its milliseconds are exact in an `i64`.
const PAST_LAST: f64 = 3_000_000.0;

impl Date {
    /// The date of the serial date-time `serial` counted in `system`, the
    /// time of day dropped; `None` when the number is no date-time of
    /// Excel's: negative, not finite, past 9999-12-31T23:59:59.999, or, in
    /// [`DateSystem::Compatibility1900`], before day 1 or on day 60.
    #[must_use]
    pub fn from_serial(serial: f64, system: DateSystem) -> Option<Self> {
        instant(serial, system).and_then(|ms| Date::from_day(ms.div_euclid(DAY_MS)))
    }
}

impl DateTime {
    /// The date and time of day of the serial date-time `serial` counted in
    /// `system`, rounded to the millisecond; `None` when the number is no
    /// date-time of Excel's, as for [`Date::from_serial`].
    #[must_use]
    pub fn from_serial(serial: f64, system: DateSystem) -> Option<Self> {
        instant(serial, system).and_then(DateTime::from_ms)
    }
}

impl Time {
    /// The time of day of the serial date-time `serial` counted in
    /// `system`, the day dropped and the time rounded to the millisecond;
    /// `None` when the number is negative, not finite, or past
    /// 9999-12-31T23:59:59.999 once rounded.
    #[must_use]
    pub fn from_serial(serial: f64, system: DateSystem) -> Option<Self> {
        millis(serial, system).and_then(Time::from_ms)
    }
}

impl Elapsed {
    /// The elapsed time of the serial date-time `serial` counted in
    /// `system`, rounded to the millisecond; `None` when the number is
    /// negative, not finite, or past 9999-12-31T23:59:59.999 once rounded.
    #[must_use]
    pub fn from_serial(serial: f64, system: DateSystem) -> Option<Self> {
        millis(serial, system).and_then(Elapsed::from_ms)
    }
}

impl Date {
    /// The serial date-time of the date in `system`, the inverse of
    /// [`Date::from_serial`]; `None` before the system's first day.
    pub(crate) fn serial(self, system: DateSystem) -> Option<f64> {
        Some(exact(day_serial(self, system)?, 0))
    }
}

impl DateTime {
    /// The serial date-time of the date and time of day in `system`, the
    /// inverse of [`DateTime::from_serial`]; `None` before the system's
    /// first day.
    pub(crate) fn serial(self, system: DateSystem) -> Option<f64> {
        Some(exact(
            day_serial(self.date(), system)?,
            time_ms(self.time()),
        ))
    }
}

impl Time {
    /// The fraction of a day the time of day is, in every date system.
    pub(crate) fn serial(self) -> f64 {
        exact(0, time_ms(self))
    }
}

impl Elapsed {
    /// The days the elapsed time is, the inverse of [`Elapsed::from_serial`];
    /// `None` when the length, counted from the system's serial 0, is past
    /// the serial of 9999-12-31T23:59:59.999.
    pub(crate) fn serial(self, system: DateSystem) -> Option<f64> {
        let ms = i64::try_from(self.milliseconds()).ok()?;
        let last = (LAST_DAY.saturating_add(1).saturating_sub(base(system))).saturating_mul(DAY_MS);
        (ms < last).then(|| exact(0, ms))
    }
}

/// The days of `date` from serial 0 in `system`; `None` before the system's
/// first day: in `Compatibility1900`, which counts the 1900-02-29 that does
/// not exist as serial 60, day 1 is 1900-01-01 and 1899-12-31 is before it.
fn day_serial(date: Date, system: DateSystem) -> Option<i64> {
    let (year, month, day) = (date.year(), date.month(), date.day());
    let day = Date::day_of(i64::from(year), i64::from(month), i64::from(day))?
        .saturating_sub(base(system));
    match system {
        DateSystem::Compatibility1900 => {
            let serial = if day < 61 { day.saturating_sub(1) } else { day };
            (serial >= 1).then_some(serial)
        }
        DateSystem::Base1900 | DateSystem::Base1904 => (day >= 0).then_some(day),
    }
}

/// The milliseconds of a time of day from midnight.
fn time_ms(time: Time) -> i64 {
    i64::from(time.hour())
        .saturating_mul(3_600_000)
        .saturating_add(i64::from(time.minute()).saturating_mul(60_000))
        .saturating_add(i64::from(time.second()).saturating_mul(1000))
        .saturating_add(i64::from(time.millisecond()))
}

/// `day` days and `ms` milliseconds as a serial. `day × 86,400,000 + ms` is
/// below 2^53, so it is held exactly by an `f64`, the division is the
/// nearest `f64` to the exact quotient, and the millisecond is kept
/// (`from_serial` rounds to it).
fn exact(day: i64, ms: i64) -> f64 {
    day.saturating_mul(DAY_MS).saturating_add(ms) as f64 / DAY_MS as f64
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

#[cfg(test)]
mod tests;
