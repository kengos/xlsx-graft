//! Properties of a date, time, date-time or elapsed time written: over the
//! whole range, in each date system, what `write_cell` writes as the
//! cell's number reads back to the value, through the `f64` the text
//! parses to, and the values outside the range are refused.

use crate::clock::{DAY_MS, LAST_DAY};
use crate::{CellValue, Date, DateSystem, DateTime, Elapsed, Time, Written, write_cell};
use proptest::prelude::*;

/// The three date systems.
fn systems() -> impl Strategy<Value = DateSystem> {
    prop::sample::select(
        &[
            DateSystem::Compatibility1900,
            DateSystem::Base1900,
            DateSystem::Base1904,
        ][..],
    )
}

/// The day of the first date `system` writes.
fn first_day(system: DateSystem) -> i64 {
    let (year, month, day) = match system {
        DateSystem::Compatibility1900 => (1900, 1, 1),
        DateSystem::Base1900 => (1899, 12, 30),
        DateSystem::Base1904 => (1904, 1, 1),
    };
    Date::day_of(year, month, day).unwrap()
}

/// The milliseconds of the longest elapsed time `system` writes, and one.
fn elapsed_limit(system: DateSystem) -> i64 {
    let zero = match system {
        DateSystem::Compatibility1900 | DateSystem::Base1900 => (1899, 12, 30),
        DateSystem::Base1904 => (1904, 1, 1),
    };
    (LAST_DAY + 1 - Date::day_of(zero.0, zero.1, zero.2).unwrap()) * DAY_MS
}

/// The number `write_cell` writes for `value`, read back from its text;
/// `None` when it refuses the value.
fn written(value: &CellValue<'_>, system: DateSystem) -> Option<f64> {
    let mut out = Vec::new();
    let at = crate::CellRef::new(1, 1).unwrap();
    assert_eq!(
        write_cell(&mut out, at, None, value, system).ok()?,
        Written::Whole
    );
    let xml = String::from_utf8(out).unwrap();
    let v = xml
        .strip_prefix(r#"<c r="A1"><v>"#)?
        .strip_suffix("</v></c>")?;
    Some(v.parse().unwrap())
}

proptest! {
    #[test]
    fn a_date_reads_back_from_its_serial_and_from_its_text(
        day in 0..=LAST_DAY,
        system in systems(),
    ) {
        let date = Date::from_day(day).unwrap();
        let serial = date.serial(system);
        prop_assert_eq!(serial.is_some(), day >= first_day(system));
        prop_assert_eq!(written(&CellValue::Date(date), system), serial);
        if let Some(serial) = serial {
            prop_assert_eq!(Date::from_serial(serial, system), Some(date));
        }
    }

    #[test]
    fn a_date_time_reads_back_from_its_serial_and_from_its_text(
        ms in 0..(LAST_DAY + 1) * DAY_MS,
        system in systems(),
    ) {
        let date_time = DateTime::from_ms(ms).unwrap();
        let serial = date_time.serial(system);
        prop_assert_eq!(serial.is_some(), ms.div_euclid(DAY_MS) >= first_day(system));
        prop_assert_eq!(written(&CellValue::DateTime(date_time), system), serial);
        if let Some(serial) = serial {
            prop_assert_eq!(DateTime::from_serial(serial, system), Some(date_time));
        }
    }

    #[test]
    fn a_time_reads_back_from_its_serial_and_from_its_text(
        ms in 0..DAY_MS,
        system in systems(),
    ) {
        let time = Time::from_ms(ms).unwrap();
        let serial = time.serial();
        prop_assert!((0.0..1.0).contains(&serial));
        prop_assert_eq!(written(&CellValue::Time(time), system), Some(serial));
        prop_assert_eq!(Time::from_serial(serial, system), Some(time));
    }

    #[test]
    fn an_elapsed_time_reads_back_from_its_serial_and_from_its_text(
        system in systems(),
        past in 0..2 * DAY_MS,
        before in any::<bool>(),
        ms in 0..i64::MAX,
    ) {
        let limit = elapsed_limit(system);
        // Any ms below the limit, or one from the day before the limit to the
        // day after it.
        let ms = if before { ms % limit } else { limit - DAY_MS + past };
        let elapsed = Elapsed::from_ms(ms).unwrap();
        let serial = elapsed.serial(system);
        prop_assert_eq!(serial.is_some(), ms < limit);
        prop_assert_eq!(written(&CellValue::Elapsed(elapsed), system), serial);
        if let Some(serial) = serial {
            prop_assert_eq!(Elapsed::from_serial(serial, system), Some(elapsed));
        }
    }
}
