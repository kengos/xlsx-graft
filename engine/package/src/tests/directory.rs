//! The central directory: headers that are missing, cut short or more than
//! the end record lists, the digital signature that may close it, and the
//! sizes, offsets and disk numbers a header leaves to the zip64 extra field.

use super::builder::{Item, Zip, put16, put32};
use super::{open, readable, unreadable};
use crate::ZipLimits;
use xlsx_graft_diagnostics::{Diagnostic, ZipDirectoryReason};

/// The diagnostic for a directory invalid for `reason`.
fn invalid(reason: ZipDirectoryReason) -> Diagnostic {
    Diagnostic::ZipDirectoryInvalid { reason }
}

/// A zip of two stored entries.
fn two() -> Zip {
    Zip::of(vec![
        Item::stored("a", b"hello"),
        Item::stored("b", b"world"),
    ])
}

#[test]
fn fewer_headers_than_the_end_record_lists_is_invalid() {
    let mut zip = two();
    zip.lies.entries = Some(3);
    assert_eq!(
        unreadable(zip.build()),
        invalid(ZipDirectoryReason::MissingHeader)
    );
}

#[test]
fn a_later_header_without_its_signature_is_invalid() {
    let mut bytes = two().build();
    // Two local records of 36 bytes, then the first header of 47.
    bytes[72 + 47] ^= 0xff;
    assert_eq!(
        unreadable(bytes),
        invalid(ZipDirectoryReason::MissingHeader)
    );
}

#[test]
fn a_header_running_past_the_directory_is_invalid() {
    let mut zip = two().build();
    // The second header's extra field length, 0xffff past the directory.
    zip[72 + 47 + 30..72 + 47 + 32].copy_from_slice(&[0xff, 0xff]);
    assert_eq!(unreadable(zip), invalid(ZipDirectoryReason::MissingHeader));
}

#[test]
fn bytes_between_the_directory_and_the_end_records_are_invalid() {
    let mut zip = two();
    zip.lies.size = Some(93);
    assert_eq!(
        unreadable(zip.build()),
        invalid(ZipDirectoryReason::ExtraBytes)
    );
    // A second directory there would be what another reader reads.
    let mut hidden = two();
    hidden.lies.entries = Some(1);
    hidden.lies.size = Some(47);
    assert_eq!(
        unreadable(hidden.build()),
        invalid(ZipDirectoryReason::ExtraBytes)
    );
}

#[test]
fn more_headers_than_the_end_record_lists_is_invalid() {
    let mut zip = two();
    zip.lies.entries = Some(1);
    assert_eq!(
        unreadable(zip.build()),
        invalid(ZipDirectoryReason::ExtraBytes)
    );
    let mut junk = Zip::of(vec![Item::stored("a", b"hello")]);
    junk.directory_tail = b"junk".to_vec();
    assert_eq!(
        unreadable(junk.build()),
        invalid(ZipDirectoryReason::ExtraBytes)
    );
}

#[test]
fn a_digital_signature_may_close_the_directory() {
    let mut zip = two();
    put32(&mut zip.directory_tail, 0x0505_4b50);
    put16(&mut zip.directory_tail, 3);
    zip.directory_tail.extend(b"sig");
    assert_eq!(
        readable(zip.build(), &ZipLimits::default()),
        (vec![true, true], vec![])
    );
    zip.directory_tail.push(0);
    assert_eq!(
        unreadable(zip.build()),
        invalid(ZipDirectoryReason::ExtraBytes)
    );
}

#[test]
fn sizes_and_offsets_are_read_from_the_zip64_extra_field() {
    let mut big = Item::deflated("b", &[7; 1_000]);
    big.zip64 = true;
    let zip = Zip::of(vec![Item::stored("a", b"hello"), big]);
    let (archive, found) = open(zip.build());
    assert!(found.is_empty());
    let archive = archive.unwrap();
    let entry = &archive.entries()[1];
    assert_eq!((entry.size(), entry.is_readable()), (1_000, true));
    // The second local header starts after the first (30 + 1 + 5).
    assert_eq!(entry.data_range().start, 36 + 30 + 1 + 20);
}

#[test]
fn the_zip64_extra_field_is_found_among_other_extra_fields() {
    let mut item = Item::deflated("a", &[7; 1_000]);
    item.zip64 = true;
    // An NTFS timestamp field (0x000a) of 32 bytes ahead of it.
    put16(&mut item.extra, 0x000a);
    put16(&mut item.extra, 32);
    item.extra.extend([0; 32]);
    let (archive, found) = open(Zip::of(vec![item]).build());
    assert!(found.is_empty());
    assert_eq!(archive.unwrap().entries()[0].size(), 1_000);
}

#[test]
fn a_size_left_to_a_missing_zip64_field_is_invalid() {
    let mut item = Item::stored("a", b"hello");
    item.central.compressed_size = u64::from(u32::MAX);
    assert_eq!(
        unreadable(Zip::of(vec![item]).build()),
        invalid(ZipDirectoryReason::Zip64FieldMissing)
    );
    // A zip64 field that holds the sizes and the offset, but not the disk.
    let mut item = Item::stored("a", b"hello");
    item.zip64 = true;
    item.disk = u16::MAX;
    assert_eq!(
        unreadable(Zip::of(vec![item]).build()),
        invalid(ZipDirectoryReason::Zip64FieldMissing)
    );
}

#[test]
fn an_entry_on_another_disk_is_refused() {
    let mut item = Item::stored("a", b"hello");
    item.disk = 1;
    assert_eq!(
        unreadable(Zip::of(vec![item]).build()),
        Diagnostic::ZipMultiDisk {}
    );
}
