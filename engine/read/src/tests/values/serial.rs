//! A cell's number made a date, a time of day, a date-time or an elapsed
//! time by its format, in each date system: day 0, 1900's day 60,
//! negative numbers, the last millisecond of 9999 and one past it, and the
//! rounding to the millisecond.

use super::iso;
use crate::{DateSystem, FormatKind, Value};
use DateSystem::{Base1900, Base1904, Compatibility1900};
use FormatKind::{Date, DateTime, Duration, General, Number, Text, Time, Unknown};

/// Milliseconds in a day, as a serial's fraction counts them.
const DAY: f64 = 86_400_000.0;

/// What `serial` is under a format of `kind` in `system`: ISO 8601 text
/// for a date or time, the number for a number.
fn shown(serial: f64, kind: FormatKind, system: DateSystem) -> Result<String, f64> {
    let value = Value::from_serial(serial, kind, system);
    match (iso(value), value) {
        (Some(text), _) => Ok(text),
        (None, Value::Number(number)) => Err(number),
        (None, other) => panic!("{serial} made {other:?}"),
    }
}

#[test]
fn the_days_of_each_system() {
    let cases = [
        // Lotus's 1900: day 1 is 1900-01-01, day 60 never was, and from
        // day 61 it is the standard's 1900.
        (1.0, Compatibility1900, Ok("1900-01-01")),
        (59.0, Compatibility1900, Ok("1900-02-28")),
        (60.0, Compatibility1900, Err(60.0)),
        (61.0, Compatibility1900, Ok("1900-03-01")),
        (0.0, Compatibility1900, Err(0.0)),
        (0.5, Compatibility1900, Err(0.5)),
        (45_000.0, Compatibility1900, Ok("2023-03-15")),
        // The standard's 1900: day 0 is 1899-12-30, and 60 is 1900-02-28.
        (0.0, Base1900, Ok("1899-12-30")),
        (1.0, Base1900, Ok("1899-12-31")),
        (60.0, Base1900, Ok("1900-02-28")),
        (61.0, Base1900, Ok("1900-03-01")),
        (45_000.0, Base1900, Ok("2023-03-15")),
        // 1904: day 0 is 1904-01-01.
        (0.0, Base1904, Ok("1904-01-01")),
        (2225.0, Base1904, Ok("1910-02-03")),
        (45_000.0, Base1904, Ok("2027-03-16")),
        // A date drops the time of day.
        (61.75, Compatibility1900, Ok("1900-03-01")),
    ];
    for (serial, system, expected) in cases {
        let expected = expected.map(str::to_owned);
        assert_eq!(
            shown(serial, Date, system),
            expected,
            "{serial} in {system:?}"
        );
    }
}

#[test]
fn negative_and_not_finite_numbers_stay_numbers() {
    for system in [Compatibility1900, Base1900, Base1904] {
        for kind in [Date, DateTime, Time, Duration] {
            assert_eq!(shown(-1.0, kind, system), Err(-1.0));
            assert_eq!(shown(-0.000_000_1, kind, system), Err(-0.000_000_1));
            assert_eq!(shown(f64::INFINITY, kind, system), Err(f64::INFINITY));
            let nan = Value::from_serial(f64::NAN, kind, system);
            assert!(matches!(nan, Value::Number(n) if n.is_nan()), "{nan:?}");
        }
    }
}

#[test]
fn the_last_millisecond_of_9999_and_one_past_it() {
    let cases = [
        (Compatibility1900, 2_958_465.0),
        (Base1900, 2_958_465.0),
        (Base1904, 2_957_003.0),
    ];
    for (system, last_day) in cases {
        let last = last_day + 86_399_999.4 / DAY;
        let past = last_day + 86_399_999.6 / DAY;
        assert_eq!(shown(last_day, Date, system), Ok("9999-12-31".to_owned()));
        assert_eq!(
            shown(last, DateTime, system),
            Ok("9999-12-31T23:59:59.999".to_owned())
        );
        assert_eq!(shown(last, Time, system), Ok("23:59:59.999".to_owned()));
        for kind in [Date, DateTime, Time, Duration] {
            assert_eq!(
                shown(past, kind, system),
                Err(past),
                "{kind:?} in {system:?}"
            );
            assert_eq!(shown(last_day + 1.0, kind, system), Err(last_day + 1.0));
        }
    }
    // The standard's 9999-12-31T23:59:59, 2958465.9999884 to its seven
    // decimals, is 23:59:58.998 to the millisecond.
    assert_eq!(
        shown(2_958_465.999_988_4, DateTime, Base1900),
        Ok("9999-12-31T23:59:58.998".to_owned())
    );
    assert_eq!(
        shown(2_957_003.0 + 86_399_000.0 / DAY, DateTime, Base1904),
        Ok("9999-12-31T23:59:59.000".to_owned())
    );
}

#[test]
fn date_times_keep_the_time_of_day() {
    let cases = [
        (61.5, Compatibility1900, Ok("1900-03-01T12:00:00.000")),
        (1.5, Compatibility1900, Ok("1900-01-01T12:00:00.000")),
        (0.5, Base1900, Ok("1899-12-30T12:00:00.000")),
        (0.5, Compatibility1900, Err(0.5)),
        (60.5, Compatibility1900, Err(60.5)),
        // 59 and a little less than half a millisecond rounds to day 60.
        (
            59.0 + 86_399_999.6 / DAY,
            Compatibility1900,
            Err(59.0 + 86_399_999.6 / DAY),
        ),
        // The standard's 10:05:54, to its seven decimals, is a millisecond
        // past it.
        (3_687.420_763_9, Base1900, Ok("1910-02-03T10:05:54.001")),
        (
            2225.0 + 36_354_000.0 / DAY,
            Base1904,
            Ok("1910-02-03T10:05:54.000"),
        ),
        (0.5, Base1904, Ok("1904-01-01T12:00:00.000")),
    ];
    for (serial, system, expected) in cases {
        let expected = expected.map(str::to_owned);
        assert_eq!(
            shown(serial, DateTime, system),
            expected,
            "{serial} in {system:?}"
        );
    }
}

#[test]
fn a_time_drops_the_day() {
    let cases = [
        (45_000.75, Compatibility1900, "18:00:00.000"),
        (60.5, Compatibility1900, "12:00:00.000"),
        (0.0, Compatibility1900, "00:00:00.000"),
        (0.5, Compatibility1900, "12:00:00.000"),
        (0.420_763_9, Base1904, "10:05:54.001"),
        (1.25, Base1900, "06:00:00.000"),
    ];
    for (serial, system, expected) in cases {
        assert_eq!(
            shown(serial, Time, system),
            Ok(expected.to_owned()),
            "{serial}"
        );
    }
}

#[test]
fn rounding_to_the_millisecond() {
    let cases = [
        (0.4, "00:00:00.000"),
        (0.6, "00:00:00.001"),
        (1.4, "00:00:00.001"),
        (1.6, "00:00:00.002"),
        (999.6, "00:00:01.000"),
        // Half a millisecond before midnight is midnight.
        (86_399_999.4, "23:59:59.999"),
        (86_399_999.6, "00:00:00.000"),
    ];
    for (ms, expected) in cases {
        assert_eq!(
            shown(ms / DAY, Time, Base1900),
            Ok(expected.to_owned()),
            "{ms}"
        );
    }
    assert_eq!(
        shown(1.0 + 86_399_999.6 / DAY, DateTime, Base1900),
        Ok("1900-01-01T00:00:00.000".to_owned())
    );
}

#[test]
fn elapsed_time_does_not_wrap() {
    let cases = [
        (0.0, Compatibility1900, 0),
        (1.5, Base1900, 129_600_000),
        (60.0, Compatibility1900, 5_184_000_000),
        (1.6 / DAY, Base1904, 2),
        (2_958_465.0, Base1900, 255_611_376_000_000),
    ];
    for (serial, system, expected) in cases {
        let value = Value::from_serial(serial, Duration, system);
        assert!(
            matches!(value, Value::Elapsed(elapsed) if elapsed.milliseconds() == expected),
            "{serial} made {value:?}"
        );
    }
    assert_eq!(shown(2_957_004.0, Duration, Base1904), Err(2_957_004.0));
}

#[test]
fn other_formats_keep_the_number() {
    for kind in [General, Number, Text, Unknown] {
        assert_eq!(shown(45_000.5, kind, Compatibility1900), Err(45_000.5));
    }
}

#[test]
fn the_parts_of_a_date_time() {
    let Value::DateTime(date_time) = Value::from_serial(45_000.5 + 1.0 / DAY, DateTime, Base1900)
    else {
        panic!("not a date-time");
    };
    let (date, time) = (date_time.date(), date_time.time());
    assert_eq!((date.year(), date.month(), date.day()), (2023, 3, 15));
    assert_eq!(
        (
            time.hour(),
            time.minute(),
            time.second(),
            time.millisecond()
        ),
        (12, 0, 0, 1)
    );
}
