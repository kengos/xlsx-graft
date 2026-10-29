//! The ISO 8601 text of a `t="d"` cell: a date, a time of day and both,
//! their ranges, the rounding of a second's decimals, and the forms that
//! are not one.

use super::iso;
use crate::{Value, ValueError};

/// What `text` reads as: ISO 8601 text of the value made of it.
fn read(text: &str) -> Result<String, ValueError> {
    Value::from_iso(text).map(|value| iso(value).unwrap_or_default())
}

#[test]
fn dates_times_and_both() {
    let cases = [
        ("1975-10-05", "1975-10-05"),
        ("0001-01-01", "0001-01-01"),
        ("9999-12-31", "9999-12-31"),
        ("2024-02-29", "2024-02-29"),
        ("2000-02-29", "2000-02-29"),
        ("08:30", "08:30:00.000"),
        ("08:30:00", "08:30:00.000"),
        ("08:30:00.000", "08:30:00.000"),
        ("23:59:59.5", "23:59:59.500"),
        ("1976-11-22T08:30", "1976-11-22T08:30:00.000"),
        ("1976-11-22T08:30:15", "1976-11-22T08:30:15.000"),
        ("1582-10-15T00:00:00.000", "1582-10-15T00:00:00.000"),
        ("9999-12-31T23:59:59.999", "9999-12-31T23:59:59.999"),
    ];
    for (text, expected) in cases {
        assert_eq!(read(text), Ok(expected.to_owned()), "{text}");
    }
    assert!(matches!(Value::from_iso("1975-10-05"), Ok(Value::Date(_))));
    assert!(matches!(Value::from_iso("08:30"), Ok(Value::Time(_))));
    assert!(matches!(
        Value::from_iso("1976-11-22T08:30"),
        Ok(Value::DateTime(_))
    ));
}

#[test]
fn decimals_round_to_the_millisecond() {
    let cases = [
        ("00:00:00.0004", "00:00:00.000"),
        ("00:00:00.0005", "00:00:00.001"),
        ("00:00:00.00049999", "00:00:00.000"),
        ("00:00:00.1234567", "00:00:00.123"),
        ("00:00:59.9996", "00:01:00.000"),
        // A time of day rounded to midnight wraps; a date-time moves on.
        ("23:59:59.9995", "00:00:00.000"),
        ("1999-12-31T23:59:59.9995", "2000-01-01T00:00:00.000"),
        ("9999-12-31T23:59:59.9994", "9999-12-31T23:59:59.999"),
    ];
    for (text, expected) in cases {
        assert_eq!(read(text), Ok(expected.to_owned()), "{text}");
    }
    assert_eq!(read("9999-12-31T23:59:59.9995"), Err(ValueError::Range));
}

#[test]
fn fields_out_of_range() {
    for text in [
        "0000-01-01",
        "2023-00-01",
        "2023-13-01",
        "2023-01-00",
        "2023-01-32",
        "2023-02-29",
        "1900-02-29",
        "2023-04-31",
        "24:00",
        "23:60",
        "23:59:60",
        "2023-02-29T00:00",
        "2023-01-01T24:00:00",
    ] {
        assert_eq!(read(text), Err(ValueError::Range), "{text}");
    }
}

#[test]
fn text_in_none_of_the_forms() {
    for text in [
        "",
        "1975",
        "1975-10",
        "1975-1-05",
        "75-10-05",
        "+1975-10-05",
        "19751005",
        "1975-10-05 08:30",
        "1975-10-05T",
        "1975-10-05T08",
        "1975-10-05T08:30Z",
        "1975-10-05T08:30:00+09:00",
        "1975-10-05Z",
        " 1975-10-05",
        "1975-10-05 ",
        "1975/10/05",
        "197x-10-05",
        "1975-10-0x",
        "8:30",
        "08:3",
        "08:30.5",
        "08:30:00.",
        "08:30:00.5x",
        "08:30:0x",
        "08-30-00",
        "T08:30",
        "45000",
    ] {
        assert_eq!(read(text), Err(ValueError::Form), "{text:?}");
    }
}

#[test]
fn the_errors_say_what_they_are() {
    assert_eq!(ValueError::Form.to_string(), "not in the form of the value");
    assert_eq!(ValueError::Range.to_string(), "out of the value's range");
}
