//! The end records: telling a zip from other files, finding the end record
//! and the zip64 records, refusing multi-disk zips, and holding the entry
//! count and the directory's size to their limits before reading it.

use super::builder::{Item, Zip};
use super::{limit, open, open_with, readable, unreadable, unreadable_with};
use crate::ZipLimits;
use xlsx_graft_diagnostics::{Diagnostic, ZipDirectoryReason};

/// A zip of one stored entry named `a`.
fn one() -> Zip {
    Zip::of(vec![Item::stored("a", b"hello")])
}

#[test]
fn an_empty_zip_opens_with_no_entries() {
    let (archive, found) = open(Zip::default().build());
    assert!(archive.unwrap().entries().is_empty());
    assert!(found.is_empty());
}

#[test]
fn a_file_that_is_not_a_zip_is_not_a_zip() {
    for bytes in [
        Vec::new(),
        b"PK".to_vec(),
        b"hello, world".to_vec(),
        vec![0; 70_000],
    ] {
        assert_eq!(unreadable(bytes), Diagnostic::NotAZip {});
    }
}

#[test]
fn an_ole_compound_file_is_reported_alone_even_with_a_zip_inside() {
    let mut bytes = vec![0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
    bytes.extend(one().build());
    assert_eq!(unreadable(bytes), Diagnostic::OleCompoundFile {});
}

#[test]
fn a_zip_cut_short_in_or_before_its_end_record_is_truncated() {
    for cut in [1, 22] {
        let mut bytes = one().build();
        bytes.truncate(bytes.len() - cut);
        assert_eq!(unreadable(bytes), Diagnostic::ZipTruncated {});
    }
}

#[test]
fn the_end_record_is_found_behind_the_longest_comment() {
    let mut zip = one();
    zip.comment = vec![b'x'; 65_535];
    assert_eq!(
        readable(zip.build(), &ZipLimits::default()),
        (vec![true], vec![])
    );
}

#[test]
fn bytes_after_the_comment_are_refused() {
    let mut bytes = one().build();
    bytes.push(0);
    assert_eq!(
        unreadable(bytes),
        Diagnostic::ZipDirectoryInvalid {
            reason: ZipDirectoryReason::TrailingBytes
        }
    );
}

#[test]
fn a_comment_cut_short_is_truncated() {
    let mut zip = one();
    zip.comment = b"hello".to_vec();
    let mut bytes = zip.build();
    bytes.truncate(bytes.len() - 2);
    assert_eq!(unreadable(bytes), Diagnostic::ZipTruncated {});
}

#[test]
fn the_last_end_record_signature_is_taken_even_inside_a_comment() {
    // A whole end record in the comment: it lists no directory where it
    // sits, so the bytes before it are left over.
    let mut fake = one();
    fake.comment = b"PK\x05\x06".to_vec();
    fake.comment.extend([0; 18]);
    assert_eq!(
        unreadable(fake.build()),
        Diagnostic::ZipDirectoryInvalid {
            reason: ZipDirectoryReason::ExtraBytes
        }
    );
    // A signature too close to the end to start one.
    let mut cut = one();
    cut.comment = b"xxPK\x05\x06".to_vec();
    assert_eq!(unreadable(cut.build()), Diagnostic::ZipTruncated {});
}

#[test]
fn a_zip_on_more_than_one_disk_is_refused() {
    let mut on_disk = one();
    on_disk.lies.disk = 1;
    let mut directory_disk = one();
    directory_disk.lies.directory_disk = 1;
    let mut entries_on_disk = one();
    entries_on_disk.lies.entries_on_disk = Some(0);
    let mut zip64_disk = one();
    zip64_disk.zip64 = true;
    zip64_disk.lies.disk = 1;
    let mut zip64_disks = one();
    zip64_disks.zip64 = true;
    zip64_disks.lies.disks = 2;
    let mut locator_disk = one();
    locator_disk.zip64 = true;
    locator_disk.lies.locator_disk = 1;
    for zip in [
        on_disk,
        directory_disk,
        entries_on_disk,
        zip64_disk,
        zip64_disks,
        locator_disk,
    ] {
        assert_eq!(unreadable(zip.build()), Diagnostic::ZipMultiDisk {});
    }
}

#[test]
fn a_zip64_end_record_is_read() {
    let mut zip = Zip::of(vec![
        Item::stored("a", b"hello"),
        Item::deflated("b", b"world"),
    ]);
    zip.zip64 = true;
    let (archive, found) = open(zip.build());
    assert!(found.is_empty());
    let names: Vec<&[u8]> = archive
        .as_ref()
        .unwrap()
        .entries()
        .iter()
        .map(|e| e.name())
        .collect();
    assert_eq!(names, [b"a", b"b"]);
}

#[test]
fn a_zip64_locator_pointing_at_no_record_is_refused() {
    for offset in [0, 1 << 40] {
        let mut zip = one();
        zip.zip64 = true;
        zip.lies.zip64_offset = Some(offset);
        assert_eq!(
            unreadable(zip.build()),
            Diagnostic::ZipDirectoryInvalid {
                reason: ZipDirectoryReason::Zip64RecordMissing
            }
        );
    }
}

#[test]
fn the_entry_limit_admits_exactly_the_default() {
    let items = (0..10_000)
        .map(|i| Item::stored(&format!("{i}"), b""))
        .collect();
    let (archive, found) = open(Zip::of(items).build());
    assert_eq!(archive.unwrap().entries().len(), 10_000);
    assert!(found.is_empty());
}

#[test]
fn one_entry_past_the_limit_is_refused_before_the_directory_is_read() {
    let items = (0..10_001)
        .map(|i| Item::stored(&format!("{i}"), b""))
        .collect();
    let refused = |count| Diagnostic::ZipEntryCountExceeded {
        count,
        limit: limit(10_000),
    };
    assert_eq!(unreadable(Zip::of(items).build()), refused(10_001));
    // The count alone decides: nothing is allocated for a count that lies.
    let mut lie = Zip::default();
    lie.lies.entries = Some(u64::from(u16::MAX));
    assert_eq!(unreadable(lie.build()), refused(65_535));
    lie.zip64 = true;
    lie.lies.entries = Some(u64::MAX);
    assert_eq!(unreadable(lie.build()), refused(u64::MAX));
}

#[test]
fn the_directory_size_limit_admits_exactly_its_value() {
    // One header of 46 bytes and the name `a`.
    let limits = |value| ZipLimits {
        max_directory_bytes: limit(value),
        ..ZipLimits::default()
    };
    assert_eq!(readable(one().build(), &limits(47)), (vec![true], vec![]));
    assert_eq!(
        unreadable_with(one().build(), &limits(46)),
        Diagnostic::ZipDirectoryTooLarge {
            size: 47,
            limit: limit(46)
        }
    );
}

#[test]
fn a_directory_running_past_its_end_records_is_truncated() {
    let mut past = one();
    past.lies.offset = Some(1_000);
    let mut longer = one();
    longer.lies.size = Some(48);
    let mut overflowing = one();
    overflowing.zip64 = true;
    overflowing.lies.offset = Some(u64::MAX);
    for zip in [past, longer, overflowing] {
        assert_eq!(unreadable(zip.build()), Diagnostic::ZipTruncated {});
    }
}

#[test]
fn offsets_that_do_not_count_data_in_front_are_prepended_data() {
    let mut bytes = b"#!/bin/sh\n".repeat(10);
    bytes.extend(one().build());
    assert_eq!(
        unreadable(bytes),
        Diagnostic::ZipPrependedData { size: 100 }
    );
}

#[test]
fn offsets_that_count_data_in_front_still_report_it() {
    let mut zip = one();
    zip.prefix = vec![b'x'; 100];
    let (readable, found) = readable(zip.build(), &ZipLimits::default());
    assert_eq!(readable, [true]);
    assert_eq!(
        found,
        [(Diagnostic::ZipPrependedData { size: 100 }, String::new())]
    );
}

#[test]
fn a_directory_that_does_not_start_with_a_header_is_invalid() {
    let zip = one();
    let mut bytes = zip.build();
    // The directory starts after the local header (30 + 1) and the data (5).
    bytes[36] ^= 0xff;
    assert_eq!(
        unreadable(bytes),
        Diagnostic::ZipDirectoryInvalid {
            reason: ZipDirectoryReason::MissingHeader
        }
    );
    let (archive, _) = open_with(zip.build(), &ZipLimits::default());
    assert!(archive.is_some());
}

#[test]
fn a_zip64_record_that_does_not_end_at_its_locator_is_refused() {
    let mut zip = one();
    zip.zip64 = true;
    let mut bytes = zip.build();
    // The record's size field: the record, locator and end record are the
    // last 56 + 20 + 22 bytes.
    let size_of_record = bytes.len() - 98 + 4;
    bytes[size_of_record] = 45;
    assert_eq!(
        unreadable(bytes),
        Diagnostic::ZipDirectoryInvalid {
            reason: ZipDirectoryReason::Zip64RecordMissing
        }
    );
}

#[test]
fn an_end_record_disagreeing_with_its_zip64_record_is_refused() {
    let mut zip = Zip::of(vec![
        Item::stored("a", b"hello"),
        Item::stored("b", b"world"),
    ]);
    zip.zip64 = true;
    // Equal to the zip64 record's value: read.
    zip.lies.plain_entries = Some(2);
    assert_eq!(
        readable(zip.build(), &ZipLimits::default()),
        (vec![true, true], vec![])
    );
    // Another value: a reader that reads the zip64 record only when it must
    // would see one entry.
    zip.lies.plain_entries = Some(1);
    assert_eq!(
        unreadable(zip.build()),
        Diagnostic::ZipDirectoryInvalid {
            reason: ZipDirectoryReason::EndRecordsDisagree
        }
    );
}
