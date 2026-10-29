//! Unit tests for what each kind of value becomes.

use super::*;
use xlsx_graft_read::{DateSystem, FormatKind};

/// The fields of `value` a test reads: kind, year, month, day, hour,
/// minute, second, millisecond, boolean, number, elapsed and text.
fn fields(value: Value<'_>) -> (u32, [u16; 8], f64, u64, Option<String>) {
    let out = XlsxGraftValue::of(value);
    assert_eq!(out.reserved, 0);
    let text = (!out.text.is_null()).then(|| {
        // SAFETY: a non-null `text` points at `text_len` bytes of the
        // value, which is alive for the call.
        let bytes = unsafe { std::slice::from_raw_parts(out.text, out.text_len) };
        String::from_utf8(bytes.to_vec()).unwrap()
    });
    let parts = [
        out.year,
        out.month.into(),
        out.day.into(),
        out.hour.into(),
        out.minute.into(),
        out.second.into(),
        out.millisecond,
        out.boolean.into(),
    ];
    (out.kind, parts, out.number, out.elapsed, text)
}

#[test]
fn each_kind_fills_only_its_own_fields() {
    let elapsed = Value::from_serial(
        1.0 + 2.05 / 24.0,
        FormatKind::Duration,
        DateSystem::Base1900,
    );
    let none = [0; 8];
    let cases = [
        (Value::Empty, (XLSX_GRAFT_VALUE_EMPTY, none, 0.0, 0, None)),
        (
            Value::String("a\tb"),
            (XLSX_GRAFT_VALUE_STRING, none, 0.0, 0, Some("a\tb".into())),
        ),
        (
            Value::Number(-0.5),
            (XLSX_GRAFT_VALUE_NUMBER, none, -0.5, 0, None),
        ),
        (
            Value::Bool(true),
            (
                XLSX_GRAFT_VALUE_BOOLEAN,
                [0, 0, 0, 0, 0, 0, 0, 1],
                0.0,
                0,
                None,
            ),
        ),
        (
            Value::Bool(false),
            (XLSX_GRAFT_VALUE_BOOLEAN, none, 0.0, 0, None),
        ),
        (
            Value::from_iso("2024-01-31").unwrap(),
            (
                XLSX_GRAFT_VALUE_DATE,
                [2024, 1, 31, 0, 0, 0, 0, 0],
                0.0,
                0,
                None,
            ),
        ),
        (
            Value::from_iso("08:30:59.5").unwrap(),
            (
                XLSX_GRAFT_VALUE_TIME,
                [0, 0, 0, 8, 30, 59, 500, 0],
                0.0,
                0,
                None,
            ),
        ),
        (
            Value::from_iso("9999-12-31T23:59:59.999").unwrap(),
            (
                XLSX_GRAFT_VALUE_DATETIME,
                [9999, 12, 31, 23, 59, 59, 999, 0],
                0.0,
                0,
                None,
            ),
        ),
        (
            elapsed,
            (XLSX_GRAFT_VALUE_ELAPSED, none, 0.0, 93_780_000, None),
        ),
        (
            Value::Error("#N/A"),
            (XLSX_GRAFT_VALUE_ERROR, none, 0.0, 0, Some("#N/A".into())),
        ),
    ];
    for (value, expected) in cases {
        assert_eq!(fields(value), expected, "{value:?}");
    }
}

#[test]
fn an_empty_string_has_a_text_of_no_bytes() {
    let out = XlsxGraftValue::of(Value::String(""));
    assert!(!out.text.is_null());
    assert_eq!((out.kind, out.text_len), (XLSX_GRAFT_VALUE_STRING, 0));
}

#[test]
fn the_text_is_lent_from_the_value_not_copied() {
    let text = String::from("lent");
    let out = XlsxGraftValue::of(Value::String(&text));
    assert_eq!((out.text, out.text_len), (text.as_ptr(), 4));
}
