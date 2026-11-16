//! Unit tests for the limits a policy may set: the table, and the table the
//! module's documentation holds.

use super::{HostLimits, LIMITS, MAX_SAFE};
use std::collections::BTreeSet;
use xlsx_graft_diagnostics::{Limit, LimitSource};
use xlsx_graft_read::ReadLimits;

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
    assert_eq!(LIMITS.len(), 25);
    let names: BTreeSet<&str> = LIMITS.iter().map(|key| key.name).collect();
    assert_eq!(names.len(), LIMITS.len());
    for key in LIMITS {
        // The spill threshold is a size at which the table moves, not a
        // most or a least.
        assert!(
            key.name.starts_with("max")
                || key.name.starts_with("min")
                || key.name == "sharedStringsSpillBytes",
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

#[test]
fn the_limits_cells_are_read_under_are_the_defaults_until_a_policy_sets_them() {
    assert_eq!(HostLimits::default().read_limits(), ReadLimits::default());
}

#[test]
fn the_limits_cells_are_read_under_come_from_verify_and_from_read() {
    let mut limits = HostLimits::default();
    for (name, value) in [
        ("maxXmlDepth", 7),
        ("maxMergedCells", 8),
        ("maxCellFormats", 9),
        ("maxNumberFormats", 10),
        ("maxSharedStrings", 11),
        ("maxSharedStringsBytes", 12),
        ("sharedStringsSpillBytes", 13),
        ("maxRowBytes", 14),
    ] {
        let key = LIMITS.iter().find(|key| key.name == name).unwrap();
        key.set(&mut limits, value);
    }
    let policy = Limit::policy_of;
    let read = limits.read_limits();
    assert_eq!(read.xml.max_depth, policy(7));
    assert_eq!(read.max_merged_cells, policy(8));
    assert_eq!(read.max_cell_formats, policy(9));
    assert_eq!(read.max_number_formats, policy(10));
    assert_eq!(read.max_shared_strings, policy(11));
    assert_eq!(read.max_shared_strings_bytes, policy(12));
    assert_eq!(read.shared_strings_spill_bytes, 13);
    assert_eq!(read.max_row_bytes, policy(14));
}
