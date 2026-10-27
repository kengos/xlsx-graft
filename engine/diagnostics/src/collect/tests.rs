//! Unit tests for `Diagnostics`: the order it keeps, and the limit at exactly
//! its value and one past it.

use super::Diagnostics;
use crate::{Diagnostic, Limit, Location};

/// Any diagnostic, to fill a collector with: `n` tells them apart.
fn filler(n: u64) -> Diagnostic {
    Diagnostic::DiagnosticsTruncated {
        dropped: n,
        dropped_errors: 0,
        limit: Limit::default_of(0),
    }
}

fn filled(mut diagnostics: Diagnostics, count: u64) -> Diagnostics {
    (0..count).for_each(|n| diagnostics.push(filler(n), Location::default()));
    diagnostics
}

#[test]
fn a_new_collector_is_empty() {
    let diagnostics = Diagnostics::default();
    assert_eq!(diagnostics, Diagnostics::new());
    assert!(diagnostics.records().is_empty());
    assert_eq!(diagnostics.dropped(), 0);
    assert!(!diagnostics.has_errors());
    assert!(diagnostics.finish().is_empty());
}

#[test]
fn records_keep_the_order_they_were_pushed_in() {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(filler(1), Location::default().with_part("/a"));
    diagnostics.push(filler(2), Location::default().with_part("/b"));
    let records = diagnostics.finish();
    assert_eq!(
        records
            .iter()
            .map(|r| r.diagnostic.clone())
            .collect::<Vec<_>>(),
        [filler(1), filler(2)]
    );
    assert_eq!(records[1].location, Location::default().with_part("/b"));
}

#[test]
fn the_origin_is_the_line_that_pushed() {
    let mut diagnostics = Diagnostics::new();
    let line = line!() + 1;
    diagnostics.push(filler(1), Location::default());
    assert_eq!(
        diagnostics.records()[0].origin(),
        format!("{}:{line}", file!())
    );
}

#[test]
fn exactly_the_default_limit_is_kept_without_a_notice() {
    let diagnostics = filled(Diagnostics::new(), 1_000);
    assert_eq!(diagnostics.records().len(), 1_000);
    assert_eq!(diagnostics.dropped(), 0);
    let records = diagnostics.finish();
    assert_eq!(records.len(), 1_000);
    assert_eq!(records[999].diagnostic, filler(999));
}

#[test]
fn one_past_the_default_limit_is_dropped_and_noticed_last() {
    let diagnostics = filled(Diagnostics::new(), 1_001);
    assert_eq!(diagnostics.records().len(), 1_000);
    assert_eq!(diagnostics.dropped(), 1);
    let records = diagnostics.finish();
    assert_eq!(records.len(), 1_001);
    assert_eq!(records[999].diagnostic, filler(999));
    assert_eq!(
        records[1_000].diagnostic,
        Diagnostic::DiagnosticsTruncated {
            dropped: 1,
            dropped_errors: 0,
            limit: Limit::default_of(1_000)
        }
    );
    assert_eq!(records[1_000].location, Location::default());
}

#[test]
fn the_notice_counts_every_dropped_diagnostic() {
    let records = filled(Diagnostics::with_limit(Limit::default_of(2)), 7).finish();
    assert_eq!(records.len(), 3);
    assert_eq!(
        records[2].diagnostic,
        Diagnostic::DiagnosticsTruncated {
            dropped: 5,
            dropped_errors: 0,
            limit: Limit::default_of(2)
        }
    );
}

#[test]
fn the_notice_origin_is_the_line_that_finished() {
    let diagnostics = filled(Diagnostics::with_limit(Limit::default_of(0)), 1);
    let line = line!() + 1;
    let records = diagnostics.finish();
    assert_eq!(records[0].origin(), format!("{}:{line}", file!()));
}
