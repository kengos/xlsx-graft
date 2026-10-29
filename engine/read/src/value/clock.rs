//! The wall-clock values a cell's number or ISO 8601 text can be: [`Date`],
//! [`Time`], [`DateTime`] and [`Elapsed`], none with a time zone, and the
//! proleptic Gregorian calendar (§18.17.4) they are counted on: a day is a
//! count of days from 0001-01-01, a time of day a count of milliseconds
//! from midnight.

use std::fmt;

/// Milliseconds in a day.
pub(super) const DAY_MS: i64 = 86_400_000;
/// The day of 9999-12-31, the latest date (§18.17.4).
pub(super) const LAST_DAY: i64 = 3_652_058;

/// A date, without a time of day: 0001-01-01 to 9999-12-31.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

/// A time of day, to the millisecond: 00:00:00.000 to 23:59:59.999.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u16,
}

/// A date and a time of day, without a time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    date: Date,
    time: Time,
}

/// Elapsed time, to the millisecond, which does not wrap at a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed {
    milliseconds: u64,
}

impl Date {
    /// The year, 1 to 9999.
    #[must_use]
    pub fn year(self) -> u16 {
        self.year
    }

    /// The month, 1 to 12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, 1 to 31.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }

    /// The date `day` days after 0001-01-01; `None` past 9999-12-31 or
    /// before 0001-01-01.
    pub(super) fn from_day(day: i64) -> Option<Self> {
        if !(0..=LAST_DAY).contains(&day) {
            return None;
        }
        // Howard Hinnant's civil_from_days, counted from 0000-03-01 so a
        // leap day ends its year; every value here is positive and small.
        let z = day + 306;
        let era = z / 146_097;
        let doe = z % 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = era * 400 + yoe + i64::from(month <= 2);
        Some(Self {
            year: u16::try_from(year).ok()?,
            month: u8::try_from(month).ok()?,
            day: u8::try_from(day).ok()?,
        })
    }

    /// The days from 0001-01-01 to `year-month-day`; `None` for a date
    /// the calendar does not have, or one outside 0001 to 9999.
    pub(super) fn day_of(year: i64, month: i64, day: i64) -> Option<i64> {
        let last = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
            2 => 28,
            _ => return None,
        };
        if !(1..=9999).contains(&year) || !(1..=last).contains(&day) {
            return None;
        }
        // Howard Hinnant's days_from_civil, the inverse of `from_day`.
        let year = year - i64::from(month <= 2);
        let era = year / 400;
        let yoe = year % 400;
        let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(era * 146_097 + doe - 306)
    }
}

impl Time {
    /// The hour, 0 to 23.
    #[must_use]
    pub fn hour(self) -> u8 {
        self.hour
    }

    /// The minute, 0 to 59.
    #[must_use]
    pub fn minute(self) -> u8 {
        self.minute
    }

    /// The second, 0 to 59.
    #[must_use]
    pub fn second(self) -> u8 {
        self.second
    }

    /// The millisecond, 0 to 999.
    #[must_use]
    pub fn millisecond(self) -> u16 {
        self.millisecond
    }

    /// The time `ms` milliseconds after midnight, wrapped at a day.
    pub(super) fn from_ms(ms: i64) -> Option<Self> {
        let ms = ms.rem_euclid(DAY_MS);
        Some(Self {
            hour: u8::try_from(ms / 3_600_000).ok()?,
            minute: u8::try_from(ms / 60_000 % 60).ok()?,
            second: u8::try_from(ms / 1000 % 60).ok()?,
            millisecond: u16::try_from(ms % 1000).ok()?,
        })
    }
}

impl DateTime {
    /// The date.
    #[must_use]
    pub fn date(self) -> Date {
        self.date
    }

    /// The time of day.
    #[must_use]
    pub fn time(self) -> Time {
        self.time
    }

    /// The instant `ms` milliseconds after 0001-01-01T00:00:00.000; `None`
    /// past 9999-12-31T23:59:59.999.
    pub(super) fn from_ms(ms: i64) -> Option<Self> {
        Some(Self {
            date: Date::from_day(ms.div_euclid(DAY_MS))?,
            time: Time::from_ms(ms)?,
        })
    }
}

impl Elapsed {
    /// The elapsed time in milliseconds.
    #[must_use]
    pub fn milliseconds(self) -> u64 {
        self.milliseconds
    }

    /// `ms` milliseconds of elapsed time; `None` when negative.
    pub(super) fn from_ms(ms: i64) -> Option<Self> {
        Some(Self {
            milliseconds: u64::try_from(ms).ok()?,
        })
    }
}

/// ISO 8601's complete extended calendar date, as §18.17.4 writes one:
/// `1975-10-05`.
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// ISO 8601's complete extended time of day, with its milliseconds:
/// `08:30:00.000`.
impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}.{:03}",
            self.hour, self.minute, self.second, self.millisecond
        )
    }
}

/// The date and the time of day, joined by `T`: `1976-11-22T08:30:00.000`.
impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}
