//! Unit tests for the cells of the JSON: every form of `added`, and a
//! `removed` version, which no row of the table has yet.

use super::EntryJson;
use serde_json::{Value, json};
use xlsx_graft_compat::{Entry, Excel, Fallback};

fn cell(entry: &Entry<Excel>) -> Value {
    serde_json::to_value(EntryJson::of(entry)).unwrap()
}

#[test]
fn added_is_a_version_a_bound_a_range_false_or_null() {
    let added = |entry| cell(&entry)["added"].clone();
    assert_eq!(added(Entry::added(Excel::V2021)), json!("2021"));
    assert_eq!(added(Entry::at_most(Excel::V2016)), json!("<=2016"));
    assert_eq!(
        added(Entry::between(Excel::V2007, Excel::V2016)),
        json!(">2007,<=2016")
    );
    assert_eq!(added(Entry::never()), json!(false));
    assert_eq!(added(Entry::unknown()), Value::Null);
}

#[test]
fn a_cell_has_its_five_keys_with_removed_partial_fallback_and_notes() {
    let entry = Entry::added(Excel::V2010)
        .removed(Excel::V365)
        .fallback(Fallback::NameError)
        .partial("differs");
    assert_eq!(
        cell(&entry),
        json!({
            "added": "2010",
            "removed": "365",
            "partial": true,
            "fallback": "name_error",
            "notes": "differs",
        })
    );
    assert_eq!(
        cell(&Entry::unknown()),
        json!({
            "added": null,
            "removed": null,
            "partial": false,
            "fallback": null,
            "notes": null,
        })
    );
}
