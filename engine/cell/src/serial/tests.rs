//! Tests for `Date::from_serial`, `DateTime::from_serial`,
//! `Time::from_serial` and `Elapsed::from_serial` on their own, without
//! xlsx-graft-read's dispatch on the format.

use crate::{Date, DateSystem, DateTime, Elapsed, Time};

#[test]
fn a_serial_is_a_date_a_date_time_a_time_and_an_elapsed_time() {
    let system = DateSystem::Base1900;
    let date = Date::from_serial(45_292.75, system).unwrap();
    assert_eq!((date.year(), date.month(), date.day()), (2024, 1, 1));
    let date_time = DateTime::from_serial(45_292.75, system).unwrap();
    assert_eq!((date_time.date(), date_time.time().hour()), (date, 18));
    let time = Time::from_serial(45_292.75, system).unwrap();
    assert_eq!((time.hour(), time.minute()), (18, 0));
    let elapsed = Elapsed::from_serial(1.5, system).unwrap();
    assert_eq!(elapsed.milliseconds(), 129_600_000);
}

#[test]
fn a_serial_that_is_no_date_time_is_none() {
    let system = DateSystem::Compatibility1900;
    for serial in [-1.0, f64::NAN, f64::INFINITY, 3_000_000.0, 2_958_466.0] {
        assert_eq!(Date::from_serial(serial, system), None);
        assert_eq!(DateTime::from_serial(serial, system), None);
        assert_eq!(Time::from_serial(serial, system), None);
        assert_eq!(Elapsed::from_serial(serial, system), None);
    }
    // In `Compatibility1900`, serial 60 (1900-02-29, which the calendar does
    // not have) and serial 0.5 (day 0) are no date, and serial 0.5 is still
    // a time of day.
    assert_eq!(Date::from_serial(60.0, system), None);
    assert_eq!(Date::from_serial(0.5, system), None);
    assert!(Time::from_serial(0.5, system).is_some());
}
