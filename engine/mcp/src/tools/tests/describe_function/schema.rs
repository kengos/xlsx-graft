//! The result of `describe_function` against its `outputSchema`: a small
//! checker of the keywords the schema uses (`type`, `enum`, `const`,
//! `required`, `properties`, `items`, `oneOf`), and, with that checker, the
//! check of the result of `describe_function` for a function of each kind of
//! row against the output schema.

use super::{described, descriptor};
use serde_json::{Value, json};

/// Whether the JSON `value` conforms to `schema`, by the keywords the output
/// schema uses. A `schema` that is not an object conforms to nothing.
fn conforms(value: &Value, schema: &Value) -> bool {
    let Some(schema) = schema.as_object() else {
        return false;
    };
    schema
        .iter()
        .all(|(keyword, argument)| match keyword.as_str() {
            "type" => type_matches(value, argument),
            "enum" => argument.as_array().is_some_and(|all| all.contains(value)),
            "const" => argument == value,
            "required" => required_present(value, argument),
            "properties" => properties_conform(value, argument),
            "items" => value
                .as_array()
                .is_none_or(|items| items.iter().all(|item| conforms(item, argument))),
            "oneOf" => {
                let shapes = argument.as_array().map_or(&[][..], Vec::as_slice);
                shapes.iter().filter(|shape| conforms(value, shape)).count() == 1
            }
            other => panic!("the checker does not know the keyword {other}"),
        })
}

/// Whether `value` is of the JSON type `types` names, or of one of them when
/// it is an array.
fn type_matches(value: &Value, types: &Value) -> bool {
    let names: Vec<&str> = match types {
        Value::String(name) => vec![name.as_str()],
        other => other
            .as_array()
            .map(|all| all.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default(),
    };
    names.iter().any(|name| match *name {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        other => panic!("the checker does not know the type {other}"),
    })
}

/// Whether the object `value` has every key `required` lists.
fn required_present(value: &Value, required: &Value) -> bool {
    let keys = required.as_array().map_or(&[][..], Vec::as_slice);
    value.as_object().is_none_or(|object| {
        keys.iter()
            .all(|key| key.as_str().is_some_and(|k| object.contains_key(k)))
    })
}

/// Whether each key of the object `value` that `properties` describes
/// conforms to its schema.
fn properties_conform(value: &Value, properties: &Value) -> bool {
    let Some(described) = properties.as_object() else {
        return false;
    };
    value.as_object().is_none_or(|object| {
        object.iter().all(|(key, found)| {
            described
                .get(key)
                .is_none_or(|schema| conforms(found, schema))
        })
    })
}

#[test]
fn the_checker_refuses_a_value_that_does_not_match_the_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "added": { "type": ["string", "boolean", "null"] },
            "list": { "type": "array", "items": { "type": "string" } },
            "kind": { "enum": ["a"] },
        },
        "required": ["added"],
    });
    assert!(conforms(
        &json!({ "added": false, "list": ["x"], "kind": "a" }),
        &schema
    ));
    for value in [
        json!({}),
        json!({ "added": 5 }),
        json!({ "added": "x", "list": [1] }),
        json!({ "added": "x", "kind": "b" }),
        json!([]),
    ] {
        assert!(!conforms(&value, &schema), "{value}");
    }
    assert!(!conforms(
        &json!({ "a": 1 }),
        &json!({ "properties": { "a": null } })
    ));
    let one_of = json!({ "oneOf": [{ "const": 1 }, { "const": 1 }] });
    assert!(!conforms(&json!(1), &one_of));
    assert!(!conforms(&json!(2), &one_of));
}

#[test]
fn the_result_for_each_kind_of_row_conforms_to_the_output_schema() {
    let schema = descriptor()["outputSchema"].clone();
    let names = [
        // A versioned function with a required feature.
        "SEQUENCE",
        // A versioned function with no known prefix.
        "XMATCH",
        "GROUPBY",
        // `added` is `<=` a version, a range of versions and `false`.
        "XLOOKUP",
        "AGGREGATE",
        "FILTERXML",
        // A baseline function ECMA-376 defines and one it does not.
        "SUM",
        "ANCHORARRAY",
        // A dangerous function.
        "WEBSERVICE",
        // A name the table does not list.
        "NOPE",
    ];
    for name in names {
        let value = &described(name)["structuredContent"];
        assert!(conforms(value, &schema), "{name}: {value}");
    }
    let xlookup = described("XLOOKUP")["structuredContent"]["support"]["numbers"]["added"].clone();
    let aggregate =
        described("AGGREGATE")["structuredContent"]["support"]["excel"]["added"].clone();
    let filterxml =
        described("FILTERXML")["structuredContent"]["support"]["excel-mac"]["added"].clone();
    assert_eq!(
        [xlookup, aggregate, filterxml],
        [json!("<=14.4"), json!(">2007,<=2016"), json!(false)]
    );
}

#[test]
fn a_result_that_leaves_the_output_schema_is_refused() {
    let schema = descriptor()["outputSchema"].clone();
    let mut value = described("SEQUENCE")["structuredContent"].clone();
    value["requires"] = json!(null);
    assert!(!conforms(&value, &schema));
    let mut value = described("WEBSERVICE")["structuredContent"].clone();
    value["safety"] = json!(null);
    assert!(!conforms(&value, &schema));
    let mut value = described("SEQUENCE")["structuredContent"].clone();
    value["support"]["excel"]["added"] = json!(5);
    assert!(!conforms(&value, &schema));
    let mut value = described("WEBSERVICE")["structuredContent"].clone();
    value["safety"].as_object_mut().unwrap().remove("danger");
    assert!(!conforms(&value, &schema));
    let mut value = described("SEQUENCE")["structuredContent"].clone();
    value["requires"][0]
        .as_object_mut()
        .unwrap()
        .remove("sources");
    assert!(!conforms(&value, &schema));
    let mut value = described("SUM")["structuredContent"].clone();
    value["known"] = json!("other");
    assert!(!conforms(&value, &schema));
}
