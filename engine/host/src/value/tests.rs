//! Unit tests for a value written as text and as JSON: each type, the
//! numbers, and the characters a text escapes.

use super::{Escape, serialize_value, write_escaped, write_value};
use serde::ser::{Serialize, SerializeMap, Serializer};
use xlsx_graft_read::{DateSystem, FormatKind, Value};

/// A value serialized as `serialize_value` does, as the entries of a map.
struct Entries<'v>(Value<'v>);

impl Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        serialize_value(&mut map, self.0)?;
        map.end()
    }
}

/// `value` as the JSON of a cell's `type` and `value`.
fn json(value: Value<'_>) -> String {
    serde_json::to_string(&Entries(value)).unwrap()
}

/// `value` as text, escaped as `escape` says.
fn text(value: Value<'_>, escape: Escape) -> String {
    let mut line = String::new();
    write_value(&mut line, value, escape).unwrap();
    line
}

/// Each kind of value, `t="d"` text making the dates and times, and the
/// elapsed time of 1 day, 2 hours and 3 minutes.
fn values() -> [Value<'static>; 9] {
    let elapsed = Value::from_serial(
        1.0 + 2.05 / 24.0,
        FormatKind::Duration,
        DateSystem::Base1900,
    );
    [
        Value::Empty,
        Value::String("a\tb"),
        Value::Number(3.0),
        Value::Bool(true),
        Value::from_iso("2024-01-31").unwrap(),
        Value::from_iso("08:30:00.5").unwrap(),
        Value::from_iso("2024-01-31T08:30:00").unwrap(),
        elapsed,
        Value::Error("#N/A"),
    ]
}

#[test]
fn each_value_is_a_type_and_a_value_in_json() {
    let cells: Vec<String> = values().into_iter().map(json).collect();
    assert_eq!(
        cells,
        [
            r#"{"type":"empty","value":null}"#,
            r#"{"type":"string","value":"a\tb"}"#,
            r#"{"type":"number","value":3.0}"#,
            r#"{"type":"boolean","value":true}"#,
            r#"{"type":"date","value":"2024-01-31"}"#,
            r#"{"type":"time","value":"08:30:00.500"}"#,
            r#"{"type":"datetime","value":"2024-01-31T08:30:00.000"}"#,
            r#"{"type":"elapsed","value":93780000}"#,
            r##"{"type":"error","value":"#N/A"}"##,
        ]
    );
}

#[test]
fn a_number_in_json_always_has_a_fraction_or_an_exponent() {
    let numbers = [
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (-42.0, "-42.0"),
        (0.1, "0.1"),
        (1e15, "1000000000000000.0"),
        (1e16, "1e+16"),
        (1.5e-7, "1.5e-7"),
        (f64::MAX, "1.7976931348623157e+308"),
        (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
    ];
    for (number, written) in numbers {
        let cell = json(Value::Number(number));
        assert_eq!(cell, format!(r#"{{"type":"number","value":{written}}}"#));
        // The JSON number reads back as the same number.
        let read: serde_json::Value = serde_json::from_str(&cell).unwrap();
        assert_eq!(read["value"].as_f64().unwrap().to_bits(), number.to_bits());
    }
}

#[test]
fn each_value_is_written_for_a_person_in_text() {
    let cells: Vec<String> = (values().into_iter())
        .map(|value| text(value, Escape::Line))
        .collect();
    assert_eq!(
        cells,
        [
            "",
            "a\\tb",
            "3",
            "TRUE",
            "2024-01-31",
            "08:30:00.500",
            "2024-01-31T08:30:00.000",
            "26:03:00.000",
            "#N/A",
        ]
    );
    assert_eq!(text(Value::Bool(false), Escape::Line), "FALSE");
}

#[test]
fn a_number_in_text_takes_an_exponent_only_far_from_one() {
    let numbers = [
        (0.0, "0"),
        (-0.0, "-0"),
        (0.1, "0.1"),
        (1e-5, "0.00001"),
        (9.9e-6, "9.9e-6"),
        (1e15, "1000000000000000"),
        (-1e16, "-1e16"),
        (123_456.789, "123456.789"),
    ];
    for (number, written) in numbers {
        assert_eq!(
            text(Value::Number(number), Escape::Line),
            written,
            "{number}"
        );
    }
}

#[test]
fn a_string_in_text_never_splits_its_line_or_its_column() {
    let value = Value::String("a\\b\tc\nd\re é|f");
    assert_eq!(text(value, Escape::Line), "a\\\\b\\tc\\nd\\re é|f");
    assert_eq!(text(Value::Error("#\t"), Escape::Line), "#\\t");
}

#[test]
fn escape_table_writes_a_vertical_bar_as_backslash_bar() {
    let value = Value::String("a|b\\c");
    assert_eq!(text(value, Escape::Table), "a\\|b\\\\c");
    assert_eq!(text(Value::Error("#|"), Escape::Table), "#\\|");
    let mut line = String::new();
    write_escaped(&mut line, "x\ty|z", Escape::Table).unwrap();
    assert_eq!(line, "x\\ty\\|z");
}
