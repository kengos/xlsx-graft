//! Unit tests for reading a setting key by key.

use super::{KeyPath, Reporter, Side, camel_case};
use crate::support::{faults, json};
use serde_json::Value;
use xlsx_graft_diagnostics::Diagnostics;

/// What `read` reports of `value` read as the policy's, each fault a string.
fn reported<R>(read: impl FnOnce(&mut Reporter<'_>, &Value) -> R, value: &str) -> Vec<String> {
    let mut diagnostics = Diagnostics::new();
    let mut reporter = Reporter::new(Side::Policy, &mut diagnostics);
    read(&mut reporter, &json(value));
    assert_eq!(reporter.failed(), !diagnostics.records().is_empty());
    faults(&diagnostics.finish())
}

#[test]
fn a_key_path_names_the_keys_and_positions_from_the_top() {
    let top = KeyPath::default();
    assert_eq!(top.key("forbid").0, "forbid");
    assert_eq!(
        top.key("forbid").key("parts").index(1).key("kind").0,
        "forbid.parts[1].kind"
    );
}

#[test]
fn an_object_reports_each_key_it_does_not_know() {
    let said = reported(
        |r, v| {
            let map = r.object(v, &KeyPath::default(), &["maxEntries", "failOn"]);
            map.is_none()
        },
        r#"{"max_entries": 1, "fail_on": 1, "other": 1, "failOn": "warn"}"#,
    );
    assert_eq!(
        said,
        [
            "policy_invalid fail_on|unknown_key|failOn",
            "policy_invalid max_entries|unknown_key|maxEntries",
            "policy_invalid other|unknown_key|",
        ]
    );
}

#[test]
fn something_that_is_no_object_is_reported_where_it_is() {
    let at = KeyPath::default().key("forbid");
    let said = reported(|r, v| r.object(v, &at, &[]).is_none(), "[]");
    assert_eq!(said, ["policy_invalid forbid|not_object|"]);
}

#[test]
fn a_snake_case_key_has_a_camel_case_candidate() {
    assert_eq!(camel_case("max_xml_depth").as_deref(), Some("maxXmlDepth"));
    assert_eq!(camel_case("fail_on").as_deref(), Some("failOn"));
    assert_eq!(camel_case("a__b").as_deref(), Some("aB"));
    assert_eq!(camel_case("_x").as_deref(), Some("X"));
    assert_eq!(camel_case("maxXmlDepth"), None);
}

#[test]
fn a_limit_is_a_whole_number_from_one_to_its_hard_max() {
    let said = |value: &str| {
        let mut read = None;
        let said = reported(
            |r, v| {
                read = r.limit(v, &KeyPath::default().key("maxEntries"), 10);
                read.is_none()
            },
            value,
        );
        (read, said)
    };
    assert_eq!(said("1"), (Some(1), vec![]));
    assert_eq!(said("10"), (Some(10), vec![]));
    let out_of_range = vec!["policy_invalid maxEntries|out_of_range|1 to 10".to_owned()];
    for value in ["0", "11", "-1", "18446744073709551615"] {
        assert_eq!(said(value), (None, out_of_range.clone()), "{value}");
    }
    let wrong_type = vec!["policy_invalid maxEntries|wrong_type|a positive integer".to_owned()];
    for value in ["1.5", "\"1\"", "true", "null", "[]"] {
        assert_eq!(said(value), (None, wrong_type.clone()), "{value}");
    }
}

#[test]
fn a_boolean_is_true_or_false_and_nothing_else() {
    let at = KeyPath::default().key("denyScheduled");
    let mut read = Vec::new();
    let said = reported(
        |r, v| {
            let items = v.as_array().unwrap();
            read = items.iter().map(|item| r.boolean(item, &at)).collect();
            false
        },
        r#"[true, false, "yes"]"#,
    );
    assert_eq!(read, [Some(true), Some(false), None]);
    assert_eq!(
        said,
        ["policy_invalid denyScheduled|wrong_type|true or false"]
    );
}

#[test]
fn a_text_is_a_string_that_is_not_empty() {
    let at = KeyPath::default().key("name");
    let said = reported(
        |r, v| {
            let items = v.as_array().unwrap();
            let read: Vec<_> = items.iter().map(|item| r.text(item, &at)).collect();
            assert_eq!(read, [Some("a"), None, None]);
            false
        },
        r#"["a", "", 1]"#,
    );
    assert_eq!(
        said,
        [
            "policy_invalid name|empty|",
            "policy_invalid name|wrong_type|a string"
        ]
    );
}

#[test]
fn texts_report_every_element_that_is_not_a_string() {
    let at = KeyPath::default().key("targets");
    let said = reported(|r, v| r.texts(v, &at).is_none(), r#"["a", 2, "", "b"]"#);
    assert_eq!(
        said,
        [
            "policy_invalid targets[1]|wrong_type|a string",
            "policy_invalid targets[2]|empty|"
        ]
    );
    let said = reported(|r, v| r.texts(v, &at).is_none(), r#"{}"#);
    assert_eq!(
        said,
        ["policy_invalid targets|wrong_type|an array of strings"]
    );
    let said = reported(
        |r, v| r.texts(v, &at) != Some(vec!["a", "b"]),
        r#"["a", "b"]"#,
    );
    assert!(said.is_empty());
}

#[test]
fn an_array_is_read_as_it_is_or_reported() {
    let at = KeyPath::default().key("forbid");
    let said = reported(|r, v| r.array(v, &at).map(<[Value]>::len) != Some(1), "[1]");
    assert!(said.is_empty());
    let said = reported(|r, v| r.array(v, &at).is_none(), "1");
    assert_eq!(
        said,
        ["policy_invalid forbid|wrong_type|an array of objects"]
    );
}

#[test]
fn a_name_not_among_the_values_lists_them() {
    let at = KeyPath::default().key("kind");
    let said = reported(
        |r, _| {
            assert_eq!(r.named("a", &at, &[1, 2], |n| n.to_string()), None);
            assert_eq!(r.named("2", &at, &[1, 2], |n| n.to_string()), Some(2));
            true
        },
        "null",
    );
    assert_eq!(said, ["policy_invalid kind|unknown_value|1, 2"]);
}

#[test]
fn a_part_kind_is_named_by_its_wire_string() {
    let at = KeyPath::default().key("kind");
    let said = reported(
        |r, _| {
            assert!(r.part_kind("external_link", &at).is_some());
            r.part_kind("Macro", &at).is_none()
        },
        "null",
    );
    assert_eq!(
        said,
        [
            "policy_invalid kind|unknown_value|macro, external_link, dde, ole, activex, data_connection, web_publish"
        ]
    );
}

#[test]
fn the_rules_side_reports_rules_invalid() {
    let mut diagnostics = Diagnostics::new();
    Reporter::new(Side::Rules, &mut diagnostics).push(
        &KeyPath::default(),
        super::SettingReason::NotObject,
        "",
    );
    assert_eq!(
        faults(&diagnostics.finish()),
        ["rules_invalid |not_object|"]
    );
}

#[test]
fn a_function_name_with_the_prefix_a_workbook_writes_is_reported_with_the_prefix() {
    let at = KeyPath::default().key("name");
    let mut names = Vec::new();
    let said = reported(
        |r, _| {
            for name in [
                "WEBSERVICE",
                "STDEV.S",
                "_xl",
                "_XLFN.FILTER",
                "_xlws.SORT",
                "_xludf.MyFn",
                "x_xlfn.A",
            ] {
                names.push(r.function_name(name, &at));
            }
        },
        "null",
    );
    assert_eq!(
        names,
        [
            Some("WEBSERVICE"),
            Some("STDEV.S"),
            Some("_xl"),
            None,
            None,
            None,
            Some("x_xlfn.A")
        ]
    );
    assert_eq!(
        said,
        [
            "policy_invalid name|prefixed|_XLFN.",
            "policy_invalid name|prefixed|_xlws.",
            "policy_invalid name|prefixed|_xludf.",
        ]
    );
}
