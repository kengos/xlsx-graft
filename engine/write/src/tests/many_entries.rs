//! An archive with 65,535 entries, the count at which the end record is
//! saturated and the zip64 end records are written, read back by
//! `ZipArchive` with the entry limit raised.

use super::{Sink, open, open_with, stored};
use crate::ZipWriter;
use xlsx_graft_diagnostics::{Diagnostics, Limit};
use xlsx_graft_package::ZipLimits;

/// A zip of `count` entries copied from one stored entry.
fn zip_of(count: usize) -> Vec<u8> {
    let mut source = open(stored(&[(b"a", b"x")]));
    let mut zip = ZipWriter::new(Sink::default());
    let mut diagnostics = Diagnostics::new();
    for _ in 0..count {
        assert!(zip.copy_entry(&mut source, 0, &mut diagnostics).unwrap());
    }
    assert!(diagnostics.records().is_empty());
    zip.finish().unwrap().bytes
}

#[test]
fn the_count_before_the_saturated_one_needs_no_zip64() {
    let bytes = zip_of(0xFFFE);
    assert!(!bytes.windows(4).any(|w| w == b"PK\x06\x06"));
    let limits = ZipLimits {
        max_entries: Limit::policy_of(0xFFFF),
        ..ZipLimits::default()
    };
    assert_eq!(open_with(bytes, &limits).entries().len(), 0xFFFE);
}

#[test]
fn the_saturated_count_is_written_in_the_zip64_end_records_and_read_back() {
    let bytes = zip_of(0xFFFF);
    assert!(bytes.windows(4).any(|w| w == b"PK\x06\x06"));
    let limits = ZipLimits {
        max_entries: Limit::policy_of(0x1_0000),
        ..ZipLimits::default()
    };
    let archive = open_with(bytes, &limits);
    assert_eq!(archive.entries().len(), 0xFFFF);
    assert!(archive.entries().iter().all(|e| e.is_readable()));
}
