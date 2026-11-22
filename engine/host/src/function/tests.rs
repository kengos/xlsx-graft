//! Tests for `describe_function`: the JSON of each state a name is in, taken
//! from real rows of the table, and the order of its keys.

use super::describe_function;
use serde_json::{Value, json};
use xlsx_graft_compat::{Feature, function};

fn described(name: &str) -> Value {
    let text = describe_function(name).to_json().unwrap();
    serde_json::from_str(&text).unwrap()
}

/// A cell of `support` written as the JSON gives it.
fn cell(added: Value, fallback: Option<&str>, notes: Option<&str>) -> Value {
    json!({
        "added": added,
        "removed": null,
        "partial": false,
        "fallback": fallback,
        "notes": notes,
    })
}

#[test]
fn a_function_with_a_row_and_a_required_feature_has_every_key() {
    let name = "SEQUENCE";
    let name_error = Some("name_error");
    let legacy_array = Some("legacy_array");
    assert_eq!(
        described("_xlfn.sequence"),
        json!({
            "name": name,
            "known": "versioned",
            "fileName": "_xlfn.SEQUENCE",
            "prefix": "_xlfn.",
            "support": {
                "excel": cell(json!("2021"), name_error, None),
                "excel-mac": cell(json!("2021"), name_error, None),
                "libreoffice": cell(json!("24.8"), None, None),
                "google-sheets": cell(json!("current"), None, None),
                "numbers": cell(Value::Null, None, None),
                "onlyoffice": cell(json!("8.0"), None, None),
            },
            "requires": [{
                "feature": "dynamic_array",
                "support": {
                    "excel": cell(json!("2021"), legacy_array, None),
                    "excel-mac": cell(json!("2021"), legacy_array, None),
                    "libreoffice": cell(
                        Value::Null,
                        None,
                        Some("24.8 added the array functions; whether a result spills by itself is not known"),
                    ),
                    "google-sheets": cell(
                        json!("current"),
                        None,
                        Some("arrays spill by a mechanism of Sheets' own; how it reads `cm` from an xlsx is not known"),
                    ),
                    "numbers": cell(json!("14.4"), None, None),
                    "onlyoffice": cell(json!("9.3"), None, None),
                },
                "sources": Feature::DynamicArray.row().sources,
            }],
            "safety": {"class": "reviewed_safe", "danger": null},
            "sources": function(name).unwrap().sources,
        })
    );
}

#[test]
fn a_function_whose_prefix_is_not_known_has_a_null_prefix_and_file_name() {
    // GROUPBY: LibreOffice never has it, Google Sheets' column is unknown.
    let value = described("GROUPBY");
    assert_eq!(value["known"], "versioned");
    assert_eq!(value["prefix"], Value::Null);
    assert_eq!(value["fileName"], Value::Null);
    assert_eq!(
        value["support"]["libreoffice"],
        cell(json!(false), None, None)
    );
    assert_eq!(
        value["support"]["google-sheets"],
        cell(Value::Null, None, None)
    );
    assert_eq!(value["requires"][0]["feature"], "dynamic_array");
}

#[test]
fn added_has_a_bound_and_a_range_with_the_notes_the_row_gives() {
    // XLOOKUP: Numbers has it in 14.4 at most.
    assert_eq!(
        described("XLOOKUP")["support"]["numbers"],
        cell(
            json!("<=14.4"),
            None,
            Some("Numbers for iCloud added it in 2020; the Mac version is not known"),
        )
    );
    // AGGREGATE: Excel lacked it in 2007 and has it in 2016.
    assert_eq!(
        described("AGGREGATE")["support"]["excel"],
        cell(json!(">2007,<=2016"), Some("name_error"), None)
    );
    // FILTERXML: Excel for Mac never has it.
    assert_eq!(
        described("FILTERXML")["support"]["excel-mac"],
        cell(
            json!(false),
            None,
            Some("relies on Windows; Excel for Mac does not return its result"),
        )
    );
}

#[test]
fn a_baseline_function_ecma_376_defines_has_no_support_and_no_requires() {
    assert_eq!(
        described("sum"),
        json!({
            "name": "SUM",
            "known": "baseline",
            "fileName": "SUM",
            "prefix": "",
            "safety": {"class": "reviewed_safe", "danger": null},
            "sources": ["ECMA-376-1:2016 §18.17.7"],
        })
    );
}

#[test]
fn a_baseline_function_ecma_376_does_not_define_has_a_null_prefix_and_no_source() {
    assert_eq!(
        described("ANCHORARRAY"),
        json!({
            "name": "ANCHORARRAY",
            "known": "baseline",
            "fileName": null,
            "prefix": null,
            "safety": {"class": "reviewed_safe", "danger": null},
            "sources": [],
        })
    );
}

#[test]
fn safety_names_the_class_and_the_danger() {
    assert_eq!(
        described("WEBSERVICE")["safety"],
        json!({"class": "dangerous", "danger": "network"})
    );
    assert_eq!(
        described("FILTERXML")["safety"],
        json!({"class": "unreviewed", "danger": null})
    );
}

#[test]
fn a_name_the_table_does_not_list_is_unknown() {
    assert_eq!(
        described("NOPE"),
        json!({"name": "NOPE", "known": "unknown"})
    );
}

#[test]
fn keys_are_written_in_the_order_of_the_contract() {
    let text = |name: &str| describe_function(name).to_json().unwrap();
    assert_eq!(text("NOPE"), r#"{"name":"NOPE","known":"unknown"}"#);
    assert_eq!(
        text("SUM"),
        concat!(
            r#"{"name":"SUM","known":"baseline","fileName":"SUM","prefix":"","#,
            r#""safety":{"class":"reviewed_safe","danger":null},"#,
            r#""sources":["ECMA-376-1:2016 §18.17.7"]}"#
        )
    );
    let versioned = text("SEQUENCE");
    let mut at = 0;
    for key in [
        r#""name":"#,
        r#""known":"#,
        r#""fileName":"#,
        r#""prefix":"#,
        r#""support":"#,
        r#""excel":"#,
        r#""added":"#,
        r#""removed":"#,
        r#""partial":"#,
        r#""fallback":"#,
        r#""notes":"#,
        r#""excel-mac":"#,
        r#""libreoffice":"#,
        r#""google-sheets":"#,
        r#""numbers":"#,
        r#""onlyoffice":"#,
        r#""requires":"#,
        r#""feature":"#,
        r#""safety":"#,
        r#""class":"#,
        r#""danger":"#,
        r#""sources":"#,
    ] {
        at += versioned[at..]
            .find(key)
            .unwrap_or_else(|| panic!("{key} is missing or out of order"));
    }
}
