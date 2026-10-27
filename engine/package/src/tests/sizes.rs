//! The size limits on what the entries declare: one entry's inflated size,
//! its ratio to the compressed size, and the total over every entry, each at
//! its largest admitted value and one past it.

use super::builder::{Item, Zip};
use super::{limit, readable};
use crate::ZipLimits;
use xlsx_graft_diagnostics::Diagnostic;

#[test]
fn an_entry_at_the_size_limit_is_read_and_one_past_it_is_refused() {
    let limits = ZipLimits {
        max_entry_bytes: limit(10),
        ..ZipLimits::default()
    };
    let zip = Zip::of(vec![
        Item::stored("a", &[1; 10]),
        Item::deflated("b", &[1; 11]),
    ]);
    let refused = Diagnostic::ZipEntryTooLarge {
        size: 11,
        limit: limit(10),
    };
    assert_eq!(
        readable(zip.build(), &limits),
        (vec![true, false], vec![(refused, "b".to_owned())])
    );
}

#[test]
fn an_entry_at_the_ratio_limit_is_read_and_one_past_it_is_refused() {
    let item = Item::deflated("a", &[0; 1_000]);
    let compressed_size = item.central.compressed_size;
    // The smallest ratio that admits it.
    let ratio = 1_000_u64.div_ceil(compressed_size);
    let limits = |ratio| ZipLimits {
        max_compression_ratio: limit(ratio),
        min_ratio_checked_bytes: limit(1_000),
        ..ZipLimits::default()
    };
    let zip = Zip::of(vec![item]);
    assert_eq!(readable(zip.build(), &limits(ratio)), (vec![true], vec![]));
    let refused = Diagnostic::ZipRatioExceeded {
        size: 1_000,
        compressed_size,
        limit: limit(ratio - 1),
    };
    assert_eq!(
        readable(zip.build(), &limits(ratio - 1)),
        (vec![false], vec![(refused, "a".to_owned())])
    );
}

#[test]
fn the_ratio_is_not_checked_below_its_threshold() {
    let limits = ZipLimits {
        max_compression_ratio: limit(1),
        min_ratio_checked_bytes: limit(1_001),
        ..ZipLimits::default()
    };
    let zip = Zip::of(vec![Item::deflated("a", &[0; 1_000])]);
    assert_eq!(readable(zip.build(), &limits), (vec![true], vec![]));
}

#[test]
fn a_run_of_zeros_at_the_default_threshold_deflates_past_the_default_ratio() {
    let item = Item::deflated("bomb", &vec![0; 16 * 1024 * 1024]);
    let compressed_size = item.central.compressed_size;
    let refused = Diagnostic::ZipRatioExceeded {
        size: 16 * 1024 * 1024,
        compressed_size,
        limit: limit(1_000),
    };
    assert_eq!(
        readable(Zip::of(vec![item]).build(), &ZipLimits::default()),
        (vec![false], vec![(refused, "bomb".to_owned())])
    );
}

#[test]
fn entries_at_the_total_limit_are_read() {
    let limits = ZipLimits {
        max_total_bytes: limit(10),
        ..ZipLimits::default()
    };
    let zip = Zip::of(vec![Item::stored("a", &[1; 4]), Item::stored("b", &[1; 6])]);
    assert_eq!(readable(zip.build(), &limits), (vec![true, true], vec![]));
}

#[test]
fn the_entry_that_crosses_the_total_limit_is_reported_and_the_rest_left_unread() {
    let limits = ZipLimits {
        max_entry_bytes: limit(6),
        max_total_bytes: limit(10),
        ..ZipLimits::default()
    };
    let zip = Zip::of(vec![
        Item::stored("a", &[1; 4]),
        // Refused on its own, so it does not count toward the total.
        Item::stored("big", &[1; 7]),
        Item::stored("b", &[1; 6]),
        Item::stored("c", &[1; 1]),
        Item::stored("d", &[1; 2]),
    ]);
    let (readable, found) = readable(zip.build(), &limits);
    assert_eq!(readable, [true, false, true, false, false]);
    let too_large = Diagnostic::ZipEntryTooLarge {
        size: 7,
        limit: limit(6),
    };
    let total = Diagnostic::ZipTotalTooLarge {
        size: 13,
        limit: limit(10),
    };
    assert_eq!(
        found,
        [(too_large, "big".to_owned()), (total, "c".to_owned())]
    );
}
