//! Unit tests for the JSON of a grid's cell: each type, and the text that
//! was cut.

use super::{GridCell, GridValue};
use serde_json::{Value as Json, json};
use xlsx_graft_diagnostics::{CellEcho, MAX_CELL_ECHO};
use xlsx_graft_read::{CellRef, DateSystem, FormatKind, Value};

/// The JSON of the cell at B2 that holds `value`.
fn cell(value: Value<'_>) -> Json {
    let value = GridValue::of(value).unwrap();
    let cell = GridCell {
        reference: CellRef::new(2, 2).unwrap(),
        value,
    };
    serde_json::to_value(&cell).unwrap()
}

/// What a serial number is under `kind`, in the 1900 date system.
fn serial(serial: f64, kind: FormatKind) -> Value<'static> {
    Value::from_serial(serial, kind, DateSystem::Compatibility1900)
}

#[test]
fn each_type_is_written_as_the_clis_read_writes_it() {
    let at = Value::from_iso("2024-01-15T08:30:00").unwrap();
    assert!(matches!(at, Value::DateTime(_)));
    let date = serial(45_306.0, FormatKind::Date);
    let time = serial(0.5, FormatKind::Time);
    let elapsed = serial(1.0 + 2.0 / 24.0 + 3.0 / 1440.0, FormatKind::Duration);
    assert!(matches!(date, Value::Date(_)));
    assert!(matches!(time, Value::Time(_)));
    assert!(matches!(elapsed, Value::Elapsed(_)));
    let cases = [
        (Value::String("text"), json!("string"), json!("text")),
        (Value::Number(1.5), json!("number"), json!(1.5)),
        (Value::Number(3.0), json!("number"), json!(3.0)),
        (Value::Bool(true), json!("boolean"), json!(true)),
        (date, json!("date"), json!("2024-01-15")),
        (time, json!("time"), json!("12:00:00.000")),
        (at, json!("datetime"), json!("2024-01-15T08:30:00.000")),
        (elapsed, json!("elapsed"), json!(93_780_000)),
        (Value::Error("#N/A"), json!("error"), json!("#N/A")),
    ];
    for (value, kind, written) in cases {
        let json = cell(value);
        assert_eq!(
            json,
            json!({"ref": "B2", "type": kind, "value": written}),
            "{value:?}"
        );
    }
}

#[test]
fn a_cell_with_no_value_is_not_a_grid_cell() {
    assert_eq!(GridValue::of(Value::Empty), None);
}

#[test]
fn a_number_is_written_with_a_fraction_so_a_float_reads_as_one() {
    let line = serde_json::to_string(&GridCell {
        reference: CellRef::new(1, 1).unwrap(),
        value: GridValue::Number(3.0),
    })
    .unwrap();
    assert_eq!(line, r#"{"ref":"A1","type":"number","value":3.0}"#);
}

#[test]
fn the_text_is_stripped_and_truncated_is_there_only_when_it_was_cut() {
    let whole = cell(Value::String("a\tb\nc\u{1b}d"));
    assert_eq!(whole["value"], "a\tb\ncd");
    assert!(whole.get("truncated").is_none());
    let exact = "x".repeat(MAX_CELL_ECHO);
    assert!(cell(Value::String(&exact)).get("truncated").is_none());
    let long = "y".repeat(MAX_CELL_ECHO + 1);
    let cut = cell(Value::String(&long));
    assert_eq!(cut["value"], "y".repeat(MAX_CELL_ECHO));
    assert_eq!(cut["truncated"], json!(true));
    let error = cell(Value::Error(&long));
    assert_eq!(error["type"], "error");
    assert_eq!(error["truncated"], json!(true));
    let keys: Vec<_> = cut.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["ref", "truncated", "type", "value"]);
    let line = serde_json::to_string(&GridCell {
        reference: CellRef::new(1, 1).unwrap(),
        value: GridValue::String(CellEcho::new(&long)),
    })
    .unwrap();
    assert!(
        line.starts_with(r#"{"ref":"A1","type":"string","value":"yyy"#),
        "{line}"
    );
    assert!(line.ends_with(r#"yyy","truncated":true}"#), "{line}");
}
