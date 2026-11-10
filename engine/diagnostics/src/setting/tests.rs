//! Unit tests for the message of `policy_invalid` and `rules_invalid`: one
//! sentence for each reason, with and without a key.

use super::message;
use crate::{Echo, SettingReason};

fn said(key: &str, reason: SettingReason, hint: &str) -> String {
    message("the policy", &Echo::new(key), reason, &Echo::new(hint))
}

#[test]
fn every_reason_has_a_sentence_that_names_the_key() {
    use SettingReason::*;
    let said = |reason, hint| said("maxEntries", reason, hint);
    assert_eq!(
        said(TooLarge, "1048576"),
        "the policy `maxEntries` is larger than the 1048576 bytes it may take"
    );
    assert_eq!(
        said(Malformed, "line 1 column 2"),
        "the policy `maxEntries` is not JSON (line 1 column 2)"
    );
    assert_eq!(
        said(NotObject, ""),
        "the policy `maxEntries` is not a JSON object"
    );
    assert_eq!(
        said(UnknownKey, ""),
        "the policy `maxEntries` is not a key it has"
    );
    assert_eq!(
        said(UnknownKey, "maxEntries"),
        "the policy `maxEntries` is not a key it has; did you mean `maxEntries`?"
    );
    assert_eq!(
        said(WrongType, "a string"),
        "the policy `maxEntries` is not a string"
    );
    assert_eq!(
        said(OutOfRange, "1 to 9"),
        "the policy `maxEntries` is out of range; it must be 1 to 9"
    );
    assert_eq!(
        said(UnknownValue, "error, warn"),
        "the policy `maxEntries` is not a value it takes; it takes error, warn"
    );
    assert_eq!(
        said(Required, "name"),
        "the policy `maxEntries` has no `name`"
    );
    assert_eq!(said(Empty, ""), "the policy `maxEntries` is empty");
    assert_eq!(
        said(Duplicate, ""),
        "the policy `maxEntries` names what an earlier entry already names"
    );
}

#[test]
fn a_fault_of_the_whole_setting_names_no_key() {
    assert_eq!(
        said("", SettingReason::NotObject, ""),
        "the policy is not a JSON object"
    );
}

#[test]
fn a_function_named_with_its_workbook_prefix_says_to_write_it_without() {
    assert_eq!(
        said(
            "forbid.functions[0].name",
            SettingReason::Prefixed,
            "_xlfn."
        ),
        "the policy `forbid.functions[0].name` names a function with its `_xlfn.` prefix, which is removed before a function is ruled on; write the name without it"
    );
}
