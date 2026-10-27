//! Unit tests for `Limit`: it prints and serializes as its bare number.

use super::{Limit, LimitSource};

#[test]
fn a_default_limit_names_the_default_source() {
    let limit = Limit::default_of(1_000);
    assert_eq!(limit.value, 1_000);
    assert_eq!(limit.source, LimitSource::Default);
}

#[test]
fn a_limit_prints_and_serializes_as_its_value() {
    let limit = Limit::default_of(1_000);
    assert_eq!(limit.to_string(), "1000");
    assert_eq!(serde_json::to_string(&limit).unwrap(), "1000");
}

#[test]
fn the_largest_limit_serializes_as_every_digit() {
    let wire = serde_json::to_string(&Limit::default_of(u64::MAX)).unwrap();
    assert_eq!(wire, "18446744073709551615");
}
