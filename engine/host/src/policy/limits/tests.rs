//! Unit tests for the limits a policy may set: the table, and the table the
//! module's documentation holds.

use super::{HostLimits, LIMITS, MAX_SAFE};
use std::collections::BTreeSet;
use xlsx_graft_diagnostics::LimitSource;

/// `n` written with a comma between thousands, as the table writes it.
fn with_commas(n: u64) -> String {
    let digits = n.to_string();
    let mut written = String::new();
    for (position, digit) in digits.chars().enumerate() {
        if position > 0 && (digits.len() - position).is_multiple_of(3) {
            written.push(',');
        }
        written.push(digit);
    }
    written
}

/// Every way the table may write the byte count or count `n`.
fn spellings(n: u64) -> Vec<String> {
    let mut spellings = vec![with_commas(n)];
    for (unit, size) in [("GiB", 1 << 30), ("MiB", 1 << 20), ("KiB", 1 << 10)] {
        if n.is_multiple_of(size) {
            spellings.push(format!("{} {unit}", n / size));
        }
    }
    if n == MAX_SAFE {
        spellings.push("2^53 − 1".to_owned());
    }
    spellings
}

#[test]
fn every_limit_has_its_own_name_and_a_default_inside_its_hard_max() {
    assert_eq!(LIMITS.len(), 21);
    let names: BTreeSet<&str> = LIMITS.iter().map(|key| key.name).collect();
    assert_eq!(names.len(), LIMITS.len());
    for key in LIMITS {
        assert!(
            key.name.starts_with("max") || key.name.starts_with("min"),
            "{}",
            key.name
        );
        assert!(
            key.name.chars().all(|c| c.is_ascii_alphanumeric()),
            "{}",
            key.name
        );
        assert!(
            (1..=key.hard_max).contains(&key.default_value()),
            "{}",
            key.name
        );
        assert!(key.hard_max <= MAX_SAFE, "{}", key.name);
    }
}

#[test]
fn a_limit_set_by_the_policy_names_the_policy_and_no_other_changes() {
    for key in LIMITS {
        let mut limits = HostLimits::default();
        key.set(&mut limits, 7);
        assert_eq!(
            limits.max_diagnostics.source == LimitSource::Policy,
            key.name == "maxDiagnostics"
        );
        assert_ne!(limits, HostLimits::default(), "{}", key.name);
        let mut again = HostLimits::default();
        key.set(&mut again, key.default_value());
        assert_ne!(
            again,
            HostLimits::default(),
            "{} keeps its default but names the policy",
            key.name
        );
    }
}

#[test]
fn the_documentation_holds_a_row_for_each_limit_with_its_default_and_hard_max() {
    let source = include_str!("../limits.rs");
    let rows: Vec<&str> = source
        .lines()
        .filter(|line| line.starts_with("//! | `"))
        .collect();
    assert_eq!(rows.len(), LIMITS.len());
    for (row, key) in rows.iter().zip(LIMITS) {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert_eq!(cells[1], format!("`{}`", key.name));
        assert!(
            spellings(key.default_value()).contains(&cells[2].to_owned()),
            "{row}"
        );
        assert!(
            spellings(key.hard_max).contains(&cells[3].to_owned()),
            "{row}"
        );
    }
}
