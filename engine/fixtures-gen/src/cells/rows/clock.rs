//! The date, time, date-time and elapsed time rows: values at the edges of
//! each date system, around the 1900-02-29 that `Compatibility1900` counts
//! and does not have, and at the latest date-time.

use super::{Row, Sample};
use xlsx_graft_cell::{Date, DateSystem, DateTime, Elapsed, Time, ValueError};

/// The latest date-time as a serial date-time in the 1900 date systems:
/// 9999-12-31 is day 2,958,465, and 23:59:59.999 is 86,399,999 of the
/// 86,400,000 milliseconds of its day.
const LATEST_1900: f64 = 2_958_465.0 + 86_399_999.0 / 86_400_000.0;

/// The days from the 1900 systems' serial 0 (1899-12-30) to the 1904
/// system's (1904-01-01).
const DAYS_BETWEEN_SYSTEMS: f64 = 1_462.0;

/// The rows of `system`: dates, times, date-times and elapsed times. The
/// error is a text of this list that `Date::from_iso`, `Time::from_iso` or
/// `DateTime::from_iso` refuses, or a serial date-time that
/// `Elapsed::from_serial` returns `None` for; `cells::tests` fails on it.
pub(super) fn rows(system: DateSystem) -> Result<Vec<Row>, ValueError> {
    let dates = [
        "1899-12-30",
        "1899-12-31",
        "1900-01-01",
        "1900-02-28",
        "1900-03-01",
        "1904-01-01",
        "2024-02-29",
        "9999-12-31",
    ];
    let times = ["00:00:00", "12:34:56.789", "23:59:59.999"];
    let date_times = [
        "1900-01-01T00:00:00",
        "1900-02-28T23:59:59.999",
        "1900-03-01T00:00:00",
        "2024-02-29T12:34:56.789",
        "9999-12-31T23:59:59.999",
    ];
    let latest = match system {
        DateSystem::Base1904 => LATEST_1900 - DAYS_BETWEEN_SYSTEMS,
        _ => LATEST_1900,
    };
    let elapsed = [
        (0.5, "0.5 days"),
        (1.5, "1.5 days"),
        (59.5, "59.5 days"),
        (60.0, "60 days"),
        (61.0, "61 days"),
        (1000.25, "1000.25 days"),
        (latest, "up to 9999-12-31T23:59:59.999"),
    ];
    let mut rows = parsed("date", &dates, Date::from_iso, Sample::Date)?;
    rows.extend(parsed("time", &times, Time::from_iso, Sample::Time)?);
    let date_times = parsed(
        "date-time",
        &date_times,
        DateTime::from_iso,
        Sample::DateTime,
    );
    rows.extend(date_times?);
    for (serial, text) in elapsed {
        let value = Elapsed::from_serial(serial, system).ok_or(ValueError::Range)?;
        rows.push(Row::new(
            format!("elapsed time {text}"),
            Sample::Elapsed(value),
        ));
    }
    Ok(rows)
}

/// A row for each of `texts`, named `<kind> <text>`, from what `parse`
/// reads of the text; the first text `parse` refuses is the error.
fn parsed<T>(
    kind: &str,
    texts: &[&str],
    parse: fn(&str) -> Result<T, ValueError>,
    sample: fn(T) -> Sample,
) -> Result<Vec<Row>, ValueError> {
    let row = |text: &&str| Ok(Row::new(format!("{kind} {text}"), sample(parse(text)?)));
    texts.iter().map(row).collect()
}
