//! Tests for `write_cell`: the bytes of each type of cell, what is refused
//! before anything is written, the cut of a long text, and a failing
//! writer.

mod properties;

use super::{CellValue, Written, write_cell};
use crate::{CellError, CellRef, Date, DateSystem, DateTime, Elapsed, ErrorValue, Time};
use std::io::{self, Write};

/// A writer that accepts `room` bytes and then fails.
pub(super) struct Short {
    /// How many more bytes it accepts.
    pub(super) room: usize,
}

impl Write for Short {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() > self.room {
            return Err(io::Error::other("full"));
        }
        self.room -= buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A1.
fn a1() -> CellRef {
    CellRef::new(1, 1).unwrap()
}

/// `value` at A1 in the style `style` and `system`, written, and whether
/// it was whole.
fn cell_in(
    style: Option<u32>,
    value: &CellValue<'_>,
    system: DateSystem,
) -> Result<(String, Written), CellError> {
    let mut out = Vec::new();
    let written = write_cell(&mut out, a1(), style, value, system)?;
    Ok((String::from_utf8(out).unwrap(), written))
}

/// `value` at A1 with no style, in the 1900 system, which must be written.
fn cell(value: &CellValue<'_>) -> String {
    cell_in(None, value, DateSystem::Compatibility1900)
        .unwrap()
        .0
}

/// The date of the ISO 8601 text `iso`.
fn date(iso: &str) -> Date {
    Date::from_iso(iso).unwrap()
}

/// The date-time of the ISO 8601 text `iso`.
fn date_time(iso: &str) -> DateTime {
    DateTime::from_iso(iso).unwrap()
}

#[test]
fn the_style_is_written_when_it_is_not_the_default() {
    let empty = CellValue::Empty;
    let system = DateSystem::Compatibility1900;
    for (style, expected) in [
        (None, r#"<c r="A1"/>"#),
        (Some(0), r#"<c r="A1"/>"#),
        (Some(3), r#"<c r="A1" s="3"/>"#),
        (Some(u32::MAX), r#"<c r="A1" s="4294967295"/>"#),
    ] {
        assert_eq!(cell_in(style, &empty, system).unwrap().0, expected);
    }
    let value = CellValue::Bool(true);
    assert_eq!(
        cell_in(Some(2), &value, system).unwrap().0,
        r#"<c r="A1" s="2" t="b"><v>1</v></c>"#
    );
}

#[test]
fn the_reference_is_the_cells_position() {
    let at = CellRef::new(16_384, 1_048_576).unwrap();
    let mut out = Vec::new();
    let system = DateSystem::Base1900;
    let written = write_cell(&mut out, at, None, &CellValue::Empty, system);
    assert_eq!(written.unwrap(), Written::Whole);
    assert_eq!(out, br#"<c r="XFD1048576"/>"#);
}

#[test]
fn a_string_that_looks_like_a_formula_number_or_date_is_inline_text() {
    for text in [
        "",
        "a",
        "=SUM(A1)",
        "00123",
        "2024-01-01",
        "TRUE",
        "#N/A",
        "1.5",
    ] {
        assert_eq!(
            cell(&CellValue::String(text)),
            format!(r#"<c r="A1" t="inlineStr"><is><t>{text}</t></is></c>"#)
        );
    }
    assert_eq!(
        cell(&CellValue::String(" a&b<c>\u{1}\r\n_x0041_ ")),
        concat!(
            r#"<c r="A1" t="inlineStr"><is><t xml:space="preserve"> a&amp;b&lt;c&gt;"#,
            "_x0001_&#13;\n_x005F_x0041_ </t></is></c>"
        )
    );
}

#[test]
fn a_number_has_no_type_and_a_boolean_and_an_error_value_have_theirs() {
    let cases = [
        (CellValue::Number(1.5), r#"<c r="A1"><v>1.5</v></c>"#),
        (CellValue::Number(-0.0), r#"<c r="A1"><v>0</v></c>"#),
        (CellValue::Number(1e21), r#"<c r="A1"><v>1E+21</v></c>"#),
        (
            CellValue::Number(1.5e-7),
            r#"<c r="A1"><v>0.00000015</v></c>"#,
        ),
        (CellValue::Number(1e-8), r#"<c r="A1"><v>1E-8</v></c>"#),
        (CellValue::Number(1e300), r#"<c r="A1"><v>1E+300</v></c>"#),
        (CellValue::Integer(-7), r#"<c r="A1"><v>-7</v></c>"#),
        (
            CellValue::Integer(9_007_199_254_740_992),
            r#"<c r="A1"><v>9007199254740992</v></c>"#,
        ),
        (CellValue::Bool(true), r#"<c r="A1" t="b"><v>1</v></c>"#),
        (CellValue::Bool(false), r#"<c r="A1" t="b"><v>0</v></c>"#),
        (
            CellValue::SharedString(12),
            r#"<c r="A1" t="s"><v>12</v></c>"#,
        ),
    ];
    for (value, expected) in cases {
        assert_eq!(cell(&value), expected, "{value:?}");
    }
}

#[test]
fn every_error_value_is_written_as_its_text() {
    for (error, text) in [
        (ErrorValue::Null, "#NULL!"),
        (ErrorValue::DivZero, "#DIV/0!"),
        (ErrorValue::Value, "#VALUE!"),
        (ErrorValue::Ref, "#REF!"),
        (ErrorValue::Name, "#NAME?"),
        (ErrorValue::Num, "#NUM!"),
        (ErrorValue::NotAvailable, "#N/A"),
        (ErrorValue::GettingData, "#GETTING_DATA"),
    ] {
        assert_eq!(
            cell(&CellValue::Error(error)),
            format!(r#"<c r="A1" t="e"><v>{text}</v></c>"#)
        );
    }
}

#[test]
fn a_date_or_time_is_a_number_in_the_date_system() {
    let day = CellValue::Date(date("2024-01-01"));
    let both = CellValue::DateTime(date_time("2024-01-01T18:00:00"));
    let time = CellValue::Time(Time::from_serial(0.75, DateSystem::Base1900).unwrap());
    let elapsed = CellValue::Elapsed(Elapsed::from_ms(129_600_000).unwrap());
    for (system, expected) in [
        (DateSystem::Compatibility1900, ["45292", "45292.75"]),
        (DateSystem::Base1900, ["45292", "45292.75"]),
        (DateSystem::Base1904, ["43830", "43830.75"]),
    ] {
        let written = |value| cell_in(None, value, system).unwrap().0;
        assert_eq!(
            written(&day),
            format!(r#"<c r="A1"><v>{}</v></c>"#, expected[0])
        );
        assert_eq!(
            written(&both),
            format!(r#"<c r="A1"><v>{}</v></c>"#, expected[1])
        );
        assert_eq!(written(&time), r#"<c r="A1"><v>0.75</v></c>"#);
        assert_eq!(written(&elapsed), r#"<c r="A1"><v>1.5</v></c>"#);
    }
}

#[test]
fn the_first_day_of_each_date_system_is_written_and_the_day_before_is_refused() {
    let system_days = [
        (
            DateSystem::Compatibility1900,
            "1900-01-01",
            "1",
            "1899-12-31",
        ),
        (DateSystem::Base1900, "1899-12-30", "0", "1899-12-29"),
        (DateSystem::Base1904, "1904-01-01", "0", "1903-12-31"),
    ];
    for (system, first, serial, before) in system_days {
        let at = |iso: &str| cell_in(None, &CellValue::Date(date(iso)), system);
        assert_eq!(
            at(first).unwrap().0,
            format!(r#"<c r="A1"><v>{serial}</v></c>"#)
        );
        assert!(
            matches!(at(before), Err(CellError::DateOutOfRange)),
            "{before}"
        );
        let at = |iso: &str| cell_in(None, &CellValue::DateTime(date_time(iso)), system);
        assert!(at(&format!("{first}T00:00:00")).is_ok());
        let day_before = format!("{before}T23:59:59");
        assert!(
            matches!(at(&day_before), Err(CellError::DateOutOfRange)),
            "{before}"
        );
    }
}

#[test]
fn compatibility_1900_skips_serial_60() {
    let system = DateSystem::Compatibility1900;
    for (iso, serial) in [
        ("1900-02-28", "59"),
        ("1900-03-01", "61"),
        ("1900-03-02", "62"),
        ("9999-12-31", "2958465"),
    ] {
        let value = CellValue::Date(date(iso));
        let expected = format!(r#"<c r="A1"><v>{serial}</v></c>"#);
        assert_eq!(cell_in(None, &value, system).unwrap().0, expected, "{iso}");
    }
    let value = CellValue::Date(date("9999-12-31"));
    let base = cell_in(None, &value, DateSystem::Base1900).unwrap().0;
    assert_eq!(base, r#"<c r="A1"><v>2958465</v></c>"#);
}

#[test]
fn an_elapsed_time_is_written_up_to_the_longest_one_a_serial_reads() {
    let last = |base: i64| (3_652_059 - base) * 86_400_000;
    for (system, base) in [
        (DateSystem::Compatibility1900, 693_593),
        (DateSystem::Base1900, 693_593),
        (DateSystem::Base1904, 695_055),
    ] {
        let at = |ms| {
            cell_in(
                None,
                &CellValue::Elapsed(Elapsed::from_ms(ms).unwrap()),
                system,
            )
        };
        let written = at(last(base) - 1).unwrap().0;
        let serial = written
            .strip_prefix(r#"<c r="A1"><v>"#)
            .and_then(|rest| rest.strip_suffix("</v></c>"))
            .unwrap();
        let serial: f64 = serial.parse().unwrap();
        assert_eq!(
            Elapsed::from_serial(serial, system).unwrap().milliseconds() as i64,
            last(base) - 1
        );
        assert!(matches!(at(last(base)), Err(CellError::DateOutOfRange)));
        assert!(matches!(at(i64::MAX), Err(CellError::DateOutOfRange)));
    }
}

#[test]
fn a_value_a_cell_cannot_hold_is_refused_and_nothing_is_written() {
    let system = DateSystem::Compatibility1900;
    let refused = [
        (CellValue::Number(f64::NAN), "NotFinite"),
        (CellValue::Number(f64::INFINITY), "NotFinite"),
        (CellValue::Number(f64::NEG_INFINITY), "NotFinite"),
        (CellValue::Integer(9_007_199_254_740_993), "InexactInteger"),
        (CellValue::Integer(i64::MIN), "InexactInteger"),
        (CellValue::Date(date("1899-12-31")), "DateOutOfRange"),
    ];
    for (value, error) in refused {
        let mut out = Vec::new();
        let result = write_cell(&mut out, a1(), Some(3), &value, system);
        assert_eq!(format!("{:?}", result.unwrap_err()), error, "{value:?}");
        assert_eq!(out, b"", "{value:?}");
    }
    // The day before the first day of each date system, and the longest
    // elapsed time plus one millisecond.
    let days = [
        (DateSystem::Compatibility1900, "1899-12-31", 693_593),
        (DateSystem::Base1900, "1899-12-29", 693_593),
        (DateSystem::Base1904, "1903-12-31", 695_055),
    ];
    for (system, before, base) in days {
        let elapsed = (3_652_059 - base) * 86_400_000;
        let refused = [
            CellValue::Date(date(before)),
            CellValue::DateTime(date_time(&format!("{before}T12:00:00"))),
            CellValue::Elapsed(Elapsed::from_ms(elapsed).unwrap()),
        ];
        for value in refused {
            let mut out = Vec::new();
            let result = write_cell(&mut out, a1(), Some(3), &value, system);
            assert!(
                matches!(result, Err(CellError::DateOutOfRange)),
                "{value:?}"
            );
            assert_eq!(out, b"", "{value:?} {system:?}");
        }
    }
}

#[test]
fn a_string_past_32767_units_is_cut_and_returns_truncated() {
    let system = DateSystem::Compatibility1900;
    let long = "a".repeat(32_768);
    let (xml, written) = cell_in(None, &CellValue::String(&long), system).unwrap();
    assert_eq!(written, Written::Truncated);
    assert_eq!(xml.matches('a').count(), 32_767);
    let kept = "a".repeat(32_767);
    let (_, written) = cell_in(None, &CellValue::String(&kept), system).unwrap();
    assert_eq!(written, Written::Whole);
    // The cut string decides `xml:space`. `"a"` x 32,767 + `" b"` is cut
    // after the last `a`, so it has no `xml:space`; `"a"` x 32,766 + `" b"`
    // is cut after the space, so it has.
    let spaced = format!("{} b", "a".repeat(32_767));
    let (xml, written) = cell_in(None, &CellValue::String(&spaced), system).unwrap();
    assert_eq!(
        (xml.contains("preserve"), written),
        (false, Written::Truncated)
    );
    let spaced = format!("{} b", "a".repeat(32_766));
    let (xml, written) = cell_in(None, &CellValue::String(&spaced), system).unwrap();
    assert_eq!(
        (xml.contains("preserve"), written),
        (true, Written::Truncated)
    );
    // A surrogate pair that would end past 32,767 units is left out whole.
    let pair = format!("{}😀", "a".repeat(32_766));
    let (xml, _) = cell_in(None, &CellValue::String(&pair), system).unwrap();
    assert!(!xml.contains('😀'));
    assert_eq!(xml.matches('a').count(), 32_766);
}

#[test]
fn a_failing_writer_is_an_io_error_wherever_it_fails() {
    let system = DateSystem::Compatibility1900;
    let values = [
        CellValue::Empty,
        CellValue::String(" a"),
        CellValue::Number(1.0),
        CellValue::Bool(true),
    ];
    for value in values {
        let (full, _) = cell_in(Some(5), &value, system).unwrap();
        for room in 0..full.len() {
            let mut out = Short { room };
            let result = write_cell(&mut out, a1(), Some(5), &value, system);
            assert!(matches!(result, Err(CellError::Io(_))), "{value:?} {room}");
        }
        let mut out = Short { room: full.len() };
        let written = write_cell(&mut out, a1(), Some(5), &value, system);
        assert_eq!(written.unwrap(), Written::Whole);
    }
}
