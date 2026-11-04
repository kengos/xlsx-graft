//! Unit tests for one cell: what each version gets, and the builders.

use super::{Added, Entry, Fallback, Status};
use crate::app::Excel;

#[test]
fn added_in_a_version_is_missing_before_it() {
    let entry = Entry::added(Excel::V2021);
    assert_eq!(entry.added, Added::In(Excel::V2021));
    assert_eq!(entry.at(Excel::V2019), Status::Unsupported);
    assert_eq!(entry.at(Excel::V2021), Status::Supported);
    assert_eq!(entry.at(Excel::V365), Status::Supported);
}

#[test]
fn at_most_a_version_is_unverified_before_it() {
    let entry = Entry::at_most(Excel::V2016);
    assert_eq!(entry.added, Added::AtMost(Excel::V2016));
    assert_eq!(entry.at(Excel::V2013), Status::Unverified);
    assert_eq!(entry.at(Excel::V2016), Status::Supported);
    assert_eq!(entry.at(Excel::V365), Status::Supported);
}

#[test]
fn between_is_missing_through_the_first_and_unverified_until_the_second() {
    let entry = Entry::between(Excel::V2021, Excel::V365);
    assert_eq!(entry.added, Added::Between(Excel::V2021, Excel::V365));
    assert_eq!(entry.at(Excel::V2019), Status::Unsupported);
    assert_eq!(entry.at(Excel::V2021), Status::Unsupported);
    assert_eq!(entry.at(Excel::V2024), Status::Unverified);
    assert_eq!(entry.at(Excel::V365), Status::Supported);
}

#[test]
fn never_and_unknown_hold_for_every_version() {
    let never = Entry::<Excel>::never();
    let unknown = Entry::<Excel>::unknown();
    for &version in <Excel as crate::Version>::ALL {
        assert_eq!(never.at(version), Status::Unsupported);
        assert_eq!(unknown.at(version), Status::Unverified);
    }
}

#[test]
fn removed_drops_it_from_that_version_on() {
    let entry = Entry::added(Excel::V2010).removed(Excel::V2021);
    assert_eq!(entry.removed, Some(Excel::V2021));
    assert_eq!(entry.at(Excel::V2007), Status::Unsupported);
    assert_eq!(entry.at(Excel::V2019), Status::Supported);
    assert_eq!(entry.at(Excel::V2021), Status::Unsupported);
    assert_eq!(entry.at(Excel::V365), Status::Unsupported);
    // Removed wins over a version not known to have it.
    assert_eq!(
        Entry::<Excel>::unknown()
            .removed(Excel::V365)
            .at(Excel::V365),
        Status::Unsupported
    );
}

#[test]
fn builders_fill_one_field_each() {
    let plain = Entry::added(Excel::V2010);
    assert_eq!(
        (plain.removed, plain.partial, plain.fallback, plain.notes),
        (None, false, None, None)
    );
    let entry = plain.fallback(Fallback::NameError).note("a note");
    assert_eq!(entry.fallback, Some(Fallback::NameError));
    assert_eq!(entry.notes, Some("a note"));
    assert!(!entry.partial);
    let partial = plain.partial("differs");
    assert!(partial.partial);
    assert_eq!(partial.notes, Some("differs"));
}

#[test]
fn fallbacks_have_the_research_vocabulary() {
    let names: Vec<_> = Fallback::ALL
        .iter()
        .map(|fallback| fallback.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "ignored",
            "value_only",
            "name_error",
            "value_error",
            "legacy_array",
            "as_note",
            "placeholder_shape",
            "degraded",
            "repair_prompt",
            "unopenable"
        ]
    );
}
