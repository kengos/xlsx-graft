//! Unit tests for the kinds of danger: each has a stable name and a
//! reason in English of its own.

use super::Danger;
use std::collections::HashSet;

#[test]
fn every_kind_of_danger_has_a_snake_case_name_of_its_own() {
    let names: Vec<_> = Danger::ALL.iter().map(|danger| danger.as_str()).collect();
    assert_eq!(
        names,
        [
            "network",
            "external_data",
            "external_code",
            "machine_info",
            "opens_link"
        ]
    );
}

#[test]
fn every_kind_of_danger_has_a_reason_of_its_own() {
    let mut seen = HashSet::new();
    for &danger in Danger::ALL {
        let reason = danger.reason();
        assert!(
            reason.starts_with(|c: char| c.is_ascii_lowercase()),
            "{reason}"
        );
        assert!(!reason.ends_with('.'), "{reason}");
        assert!(seen.insert(reason), "{reason} is given twice");
    }
}

#[test]
fn every_kind_of_danger_is_in_the_list_of_all_once() {
    // The match below names every kind: a kind added to Danger stops this
    // test from compiling until the match has an arm for the kind, and
    // Danger::ALL must hold the kind at the index the arm gives.
    let place = |danger: Danger| match danger {
        Danger::Network => 0,
        Danger::ExternalData => 1,
        Danger::ExternalCode => 2,
        Danger::MachineInfo => 3,
        Danger::OpensLink => 4,
    };
    let places: Vec<usize> = Danger::ALL.iter().map(|&danger| place(danger)).collect();
    assert_eq!(places, [0, 1, 2, 3, 4]);
}
