//! Reading an entry through `open_entry`: stored and deflated data, data
//! that inflates to more or less than declared, corrupt deflate, a CRC-32
//! that differs, and failures of the host, told apart from problems in the
//! entry.

use super::builder::{Item, Zip};
use super::{Archive, Found, Reader, found, noise, open};
use crate::{Method, PackageError, ZipArchive, ZipLimits};
use std::io::{self, Read};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, InflateReason};

/// The archive of `items`, which must open without a diagnostic.
fn archive(items: Vec<Item>) -> Archive {
    let (archive, found) = open(Zip::of(items).build());
    assert!(found.is_empty(), "{found:?}");
    archive.unwrap()
}

/// Reads the entry at `index` to its end: the bytes, or whether the error
/// was the reader stopping; and the diagnostics pushed.
fn read(archive: &mut Archive, index: usize) -> (Result<Vec<u8>, bool>, Found) {
    let mut diagnostics = Diagnostics::new();
    let mut entry = archive
        .open_entry(index, &mut diagnostics)
        .unwrap()
        .unwrap();
    let mut bytes = Vec::new();
    let result = match entry.read_to_end(&mut bytes) {
        Ok(_) => Ok(bytes),
        Err(error) => {
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            // A stopped reader keeps failing, without a second diagnostic.
            assert!(entry.read(&mut [0; 8]).is_err());
            Err(entry.is_stopped())
        }
    };
    (result, found(diagnostics))
}

/// Reads the only entry of a zip of `item`.
fn read_one(item: Item) -> (Result<Vec<u8>, bool>, Found) {
    read(&mut archive(vec![item]), 0)
}

/// What reading an entry `a` that fails with `diagnostic` returns.
fn failed(diagnostic: Diagnostic) -> (Result<Vec<u8>, bool>, Found) {
    (Err(true), vec![(diagnostic, "a".to_owned())])
}

/// The diagnostic for data that does not inflate, for `reason`.
fn inflate_failed(reason: InflateReason) -> Diagnostic {
    Diagnostic::ZipInflateFailed { reason }
}

#[test]
fn stored_and_deflated_data_read_back() {
    let large = noise(200_000);
    let mut archive = archive(vec![
        Item::stored("a", b"hello"),
        Item::deflated("b", &large),
        Item::deflated("c", &[b'x'; 100_000]),
        Item::stored("empty", b""),
        Item::deflated("deflated empty", b""),
    ]);
    assert_eq!(read(&mut archive, 0), (Ok(b"hello".to_vec()), vec![]));
    assert_eq!(read(&mut archive, 1), (Ok(large), vec![]));
    assert_eq!(read(&mut archive, 2), (Ok(vec![b'x'; 100_000]), vec![]));
    assert_eq!(read(&mut archive, 3), (Ok(Vec::new()), vec![]));
    assert_eq!(read(&mut archive, 4), (Ok(Vec::new()), vec![]));
    // An entry can be read again.
    assert_eq!(read(&mut archive, 0), (Ok(b"hello".to_vec()), vec![]));
}

#[test]
fn a_finished_reader_keeps_returning_the_end_and_an_empty_read_reads_nothing() {
    let mut archive = archive(vec![Item::deflated("a", b"hello")]);
    let mut diagnostics = Diagnostics::new();
    let mut entry = archive.open_entry(0, &mut diagnostics).unwrap().unwrap();
    assert_eq!(entry.read(&mut []).unwrap(), 0);
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    assert_eq!(entry.read(&mut [0; 8]).unwrap(), 0);
    assert_eq!(bytes, b"hello");
    assert!(!entry.is_stopped());
}

#[test]
fn data_with_another_crc32_is_refused_at_its_end() {
    for mut item in [Item::stored("a", b"hello"), Item::deflated("a", b"hello")] {
        let expected = item.central.crc32 ^ 1;
        item.central.crc32 = expected;
        item.local.crc32 = expected;
        let actual = crc32fast::hash(b"hello");
        assert_eq!(
            read_one(item),
            failed(Diagnostic::ZipCrcMismatch {
                expected: expected.into(),
                actual: actual.into(),
            })
        );
    }
}

#[test]
fn an_entry_found_broken_does_not_open_again() {
    let mut item = Item::stored("a", b"hello");
    item.central.crc32 ^= 1;
    item.local.crc32 ^= 1;
    let mut archive = archive(vec![item]);
    let (result, found) = read(&mut archive, 0);
    assert_eq!((result, found.len()), (Err(true), 1));
    assert!(!archive.entries()[0].is_readable());
    let mut diagnostics = Diagnostics::new();
    assert!(archive.open_entry(0, &mut diagnostics).unwrap().is_none());
    assert!(diagnostics.records().is_empty());
}

#[test]
fn data_that_ends_before_its_declared_size_is_refused() {
    let longer = |mut item: Item| {
        item.central.size += 1;
        item.local.size += 1;
        item
    };
    for item in [Item::stored("a", b"hello"), Item::deflated("a", b"hello")] {
        assert_eq!(
            read_one(longer(item)),
            failed(inflate_failed(InflateReason::EndedEarly))
        );
    }
    // A deflate stream cut in half.
    let mut item = Item::deflated("a", &noise(1_000));
    item.data.truncate(500);
    item.central.compressed_size = 500;
    item.local.compressed_size = 500;
    assert_eq!(
        read_one(item),
        failed(inflate_failed(InflateReason::EndedEarly))
    );
}

#[test]
fn data_that_runs_past_its_declared_size_is_refused() {
    let shorter = |mut item: Item| {
        item.central.size -= 1;
        item.local.size -= 1;
        item
    };
    for item in [Item::stored("a", b"hello"), Item::deflated("a", b"hello")] {
        let reason = InflateReason::LongerThanDeclared;
        assert_eq!(read_one(shorter(item)), failed(inflate_failed(reason)));
    }
}

#[test]
fn data_that_is_not_deflate_is_refused() {
    // The block type 3 is reserved (RFC 1951 §3.2.3).
    let item = Item::stored("a", &[0xff; 8]).with_method(8);
    assert_eq!(
        read_one(item),
        failed(inflate_failed(InflateReason::CorruptData))
    );
}

#[test]
fn an_entry_that_is_not_readable_or_not_there_does_not_open() {
    let (archive, _) = open(Zip::of(vec![Item::stored("a", b"x").with_method(12)]).build());
    let mut archive = archive.unwrap();
    let entry = &archive.entries()[0];
    assert_eq!(entry.method(), Method::Other(12));
    assert!(!entry.is_readable());
    assert_eq!(entry.data_range(), 0..0);
    let mut diagnostics = Diagnostics::new();
    assert!(archive.open_entry(0, &mut diagnostics).unwrap().is_none());
    assert!(archive.open_entry(1, &mut diagnostics).unwrap().is_none());
    assert!(diagnostics.records().is_empty());
}

#[test]
fn an_entry_describes_itself() {
    let archive = archive(vec![
        Item::stored("a", b"hello"),
        Item::deflated("b", &[0; 100]),
    ]);
    let [a, b] = archive.entries() else {
        panic!("expected two entries");
    };
    assert_eq!(
        (a.name(), a.method(), a.size(), a.compressed_size()),
        (&b"a"[..], Method::Stored, 5, 5)
    );
    assert_eq!(a.crc32(), crc32fast::hash(b"hello"));
    assert_eq!(a.data_range(), 31..36);
    assert_eq!((b.method(), b.size()), (Method::Deflate, 100));
    assert_eq!(b.data_range().start, 36 + 31);
    assert_eq!(a.clone(), *a);
}

#[test]
fn a_failure_of_the_host_is_an_error_not_a_diagnostic() {
    let failing = std::rc::Rc::new(std::cell::Cell::new(true));
    let flaky = |bytes| Reader::failing_with(bytes, failing.clone());
    let zip = || Zip::of(vec![Item::deflated("a", b"hello")]).build();
    let mut diagnostics = Diagnostics::new();
    let error =
        ZipArchive::open(flaky(zip()), &ZipLimits::default(), &mut diagnostics).unwrap_err();
    assert!(matches!(error, PackageError::Io(_)));
    assert_eq!(
        error.to_string(),
        "reading the package failed: the disk went away"
    );

    failing.set(false);
    let mut archive = ZipArchive::open(flaky(zip()), &ZipLimits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    failing.set(true);
    assert!(archive.open_entry(0, &mut diagnostics).is_err());
    failing.set(false);
    let mut entry = archive.open_entry(0, &mut diagnostics).unwrap().unwrap();
    failing.set(true);
    let error = entry.read(&mut [0; 8]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
    // Where the reader is in the data is not known after that.
    failing.set(false);
    let error = entry.read(&mut [0; 8]).unwrap_err();
    assert_eq!(error.to_string(), "an earlier read of the zip entry failed");
    assert!(!entry.is_stopped());
    assert!(diagnostics.records().is_empty());
}
