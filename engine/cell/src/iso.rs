//! A cell's ISO 8601 date-time (`t="d"`) in the three forms §18.17.4
//! allows: a calendar date, a time of day, or both joined by `T`.

use crate::ValueError;
use crate::clock::{DAY_MS, Date, DateTime, Time};

impl Date {
    /// The date of ISO 8601 text `YYYY-MM-DD`.
    ///
    /// # Errors
    ///
    /// [`ValueError::Form`] for text not in that form; [`ValueError::Range`]
    /// for a month or day the calendar does not have (month 13, 02-30) or
    /// year 0.
    pub fn from_iso(text: &str) -> Result<Self, ValueError> {
        Date::from_day(day(text.as_bytes())?).ok_or(ValueError::Range)
    }
}

impl Time {
    /// The time of day of ISO 8601 text `hh:mm`, `hh:mm:ss` or
    /// `hh:mm:ss.s…`, the seconds rounded to the millisecond. A time that
    /// rounds up to midnight wraps to 00:00:00.000: `23:59:59.9995` is
    /// `00:00:00.000`.
    ///
    /// # Errors
    ///
    /// [`ValueError::Form`] for text not in one of those forms, a time zone
    /// included; [`ValueError::Range`] for a field the clock does not have
    /// (hour 24, second 60).
    pub fn from_iso(text: &str) -> Result<Self, ValueError> {
        Time::from_ms(clock(text.as_bytes())?).ok_or(ValueError::Range)
    }
}

impl DateTime {
    /// The date and time of day of ISO 8601 text `YYYY-MM-DDThh:mm`,
    /// `YYYY-MM-DDThh:mm:ss` or `YYYY-MM-DDThh:mm:ss.s…`, the seconds
    /// rounded to the millisecond.
    ///
    /// # Errors
    ///
    /// [`ValueError::Form`] for text not in one of those forms, a time zone
    /// included; [`ValueError::Range`] for a field the calendar or the clock
    /// does not have, year 0, or a date-time rounded past
    /// 9999-12-31T23:59:59.999.
    pub fn from_iso(text: &str) -> Result<Self, ValueError> {
        let bytes = text.as_bytes();
        if bytes.get(10) != Some(&b'T') {
            return Err(ValueError::Form);
        }
        let (date, time) = bytes.split_at(10);
        // A day to 9999-12-31 and a clock to a day are far inside an i64 of
        // milliseconds.
        let ms = day(date)? * DAY_MS + clock(time.get(1..).unwrap_or_default())?;
        DateTime::from_ms(ms).ok_or(ValueError::Range)
    }
}

/// The day of `YYYY-MM-DD`, from 0001-01-01.
fn day(bytes: &[u8]) -> Result<i64, ValueError> {
    let [y1, y2, y3, y4, b'-', m1, m2, b'-', d1, d2] = *bytes else {
        return Err(ValueError::Form);
    };
    let year = number(&[y1, y2, y3, y4])?;
    let month = number(&[m1, m2])?;
    let day = number(&[d1, d2])?;
    Date::day_of(year, month, day).ok_or(ValueError::Range)
}

/// The milliseconds from midnight of `hh:mm`, `hh:mm:ss` or
/// `hh:mm:ss.s…`, rounded half up at the fourth decimal of the second:
/// 23:59:59.9995 is a day.
fn clock(bytes: &[u8]) -> Result<i64, ValueError> {
    let (whole, fraction) = match bytes.iter().position(|&b| b == b'.') {
        Some(dot) => (
            bytes.get(..dot).unwrap_or_default(),
            Some(bytes.get(dot + 1..).unwrap_or_default()),
        ),
        None => (bytes, None),
    };
    let (hour, minute, second) = match *whole {
        [h1, h2, b':', m1, m2] if fraction.is_none() => (number(&[h1, h2])?, number(&[m1, m2])?, 0),
        [h1, h2, b':', m1, m2, b':', s1, s2] => {
            (number(&[h1, h2])?, number(&[m1, m2])?, number(&[s1, s2])?)
        }
        _ => return Err(ValueError::Form),
    };
    if hour > 23 || minute > 59 || second > 59 {
        return Err(ValueError::Range);
    }
    let ms = match fraction {
        Some(digits) => thousandths(digits)?,
        None => 0,
    };
    Ok(((hour * 60 + minute) * 60 + second) * 1000 + ms)
}

/// The decimals of a second as whole milliseconds, rounded half up: 0 to
/// 1000.
fn thousandths(digits: &[u8]) -> Result<i64, ValueError> {
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(ValueError::Form);
    }
    let mut padded = [b'0'; 4];
    for (slot, &digit) in padded.iter_mut().zip(digits) {
        *slot = digit;
    }
    let [a, b, c, d] = padded;
    Ok(number(&[a, b, c])? + i64::from(d >= b'5'))
}

/// The value of ASCII digits, a few at most.
fn number(digits: &[u8]) -> Result<i64, ValueError> {
    digits.iter().try_fold(0, |value, &digit| {
        if digit.is_ascii_digit() {
            Ok(value * 10 + i64::from(digit - b'0'))
        } else {
            Err(ValueError::Form)
        }
    })
}

#[cfg(test)]
mod tests;
