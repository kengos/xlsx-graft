//! Tests for `Date::from_iso`, `Time::from_iso` and `DateTime::from_iso` on
//! their own, without xlsx-graft-read's dispatch that tells the forms apart.

use crate::{Date, DateTime, Time, ValueError};

#[test]
fn a_date_is_read_and_a_date_out_of_range_or_form_is_refused() {
    let date = Date::from_iso("2024-02-29").unwrap();
    assert_eq!((date.year(), date.month(), date.day()), (2024, 2, 29));
    assert_eq!(Date::from_iso("2023-02-29"), Err(ValueError::Range));
    assert_eq!(Date::from_iso("2024-02-29T00:00"), Err(ValueError::Form));
}

#[test]
fn a_time_is_read_and_a_time_out_of_range_or_form_is_refused() {
    let time = Time::from_iso("10:20:30.4").unwrap();
    assert_eq!(
        (
            time.hour(),
            time.minute(),
            time.second(),
            time.millisecond()
        ),
        (10, 20, 30, 400)
    );
    assert_eq!(Time::from_iso("24:00"), Err(ValueError::Range));
    assert_eq!(Time::from_iso("10:20Z"), Err(ValueError::Form));
}

#[test]
fn a_date_time_is_read_and_a_date_time_out_of_range_is_refused() {
    let date_time = DateTime::from_iso("2024-01-02T03:04:05").unwrap();
    assert_eq!(date_time.date().day(), 2);
    assert_eq!(date_time.time().minute(), 4);
    assert_eq!(
        DateTime::from_iso("2024-01-02T24:00"),
        Err(ValueError::Range)
    );
    assert_eq!(
        DateTime::from_iso("9999-12-31T23:59:59.9995"),
        Err(ValueError::Range)
    );
}

#[test]
fn a_date_time_without_t_after_the_date_is_a_form_error() {
    assert_eq!(
        DateTime::from_iso("2024-01-01 10:00"),
        Err(ValueError::Form)
    );
    assert_eq!(DateTime::from_iso("2024-01-01"), Err(ValueError::Form));
    assert_eq!(DateTime::from_iso(""), Err(ValueError::Form));
}

#[test]
fn the_last_day_of_a_month_is_a_date_in_a_common_year_and_a_30_day_month() {
    for (text, year, month, day) in [
        ("2023-02-28", 2023, 2, 28),
        ("1900-02-28", 1900, 2, 28),
        ("2023-04-30", 2023, 4, 30),
    ] {
        let date = Date::from_iso(text).unwrap();
        assert_eq!((date.year(), date.month(), date.day()), (year, month, day));
    }
}
