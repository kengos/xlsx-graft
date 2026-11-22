//! `copy_raw`: an entry's stored bytes copied as they are while the same
//! bytes are inflated and checked, for stored and deflated entries, entries
//! past the size one read takes, bytes after the end of the deflate stream,
//! entries refused partway, entries that are not readable, and failures of
//! the host (the archive's reader and the writer the bytes go to).

use super::builder::{Item, Zip};
use super::{Archive, Reader, found, noise, open};
use crate::{PackageError, ZipArchive, ZipLimits};
use std::io::{self, Write};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, InflateReason};

/// A writer that fails every write.
struct Broken;

impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("the disk is full"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A writer that accepts `limit` bytes in all and fails after that.
struct Limited {
    bytes: Vec<u8>,
    limit: usize,
}

impl Limited {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }
}

impl Write for Limited {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let room = self.limit - self.bytes.len();
        if room == 0 {
            return Err(io::Error::other("the disk is full"));
        }
        let len = buf.len().min(room);
        self.bytes.extend(&buf[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A writer that keeps the bytes and the size of the largest write.
#[derive(Default)]
struct Recorder {
    bytes: Vec<u8>,
    largest: usize,
    writes: usize,
}

impl Write for Recorder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes.extend(buf);
        self.largest = self.largest.max(buf.len());
        self.writes += 1;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The bytes of the zip of `items`.
fn bytes(items: Vec<Item>) -> Vec<u8> {
    Zip::of(items).build()
}

/// The archive of `bytes`, which must open without a diagnostic.
fn archive(bytes: Vec<u8>) -> Archive {
    let (archive, found) = open(bytes);
    assert!(found.is_empty(), "{found:?}");
    archive.unwrap()
}

/// What `copy_raw` of the entry at `index` returned, the bytes it wrote
/// and the diagnostics it pushed, each with its entry.
fn copy(archive: &mut Archive, index: usize) -> (bool, Vec<u8>, Vec<(Diagnostic, String)>) {
    let mut out = Vec::new();
    let mut diagnostics = Diagnostics::new();
    let copied = archive.copy_raw(index, &mut out, &mut diagnostics).unwrap();
    (copied, out, found(diagnostics))
}

#[test]
fn the_stored_bytes_of_each_entry_are_copied_as_they_are() {
    let zip = bytes(vec![
        Item::stored("a", b"hello"),
        Item::deflated("b", &[7; 5_000]),
        Item::stored("empty", b""),
        Item::deflated("empty deflated", b""),
    ]);
    let mut archive = archive(zip.clone());
    for index in 0..4 {
        let range = archive.entries()[index].data_range();
        let range = usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap();
        let (copied, out, found) = copy(&mut archive, index);
        assert_eq!((copied, &out[..], found.len()), (true, &zip[range], 0));
        assert!(archive.entries()[index].is_readable());
    }
}

#[test]
fn entries_larger_than_one_read_are_copied_whole() {
    let large = noise(100_000);
    let items = vec![Item::stored("a", &large), Item::deflated("b", &large)];
    let zip = bytes(items.clone());
    let mut archive = archive(zip.clone());
    for (index, item) in items.iter().enumerate() {
        let (copied, out, found) = copy(&mut archive, index);
        assert_eq!((copied, found.len()), (true, 0));
        assert!(out.len() > 32 * 1024);
        assert_eq!(out, item.data);
        let range = archive.entries()[index].data_range();
        assert_eq!(out, zip[range.start as usize..range.end as usize]);
    }
}

#[test]
fn bytes_after_the_end_of_the_deflate_stream_are_copied_too() {
    // Within the compressed size: the first, a few bytes; the second, more
    // than the input buffer, so the rest takes several reads.
    for extra in [3, 100_000] {
        let mut item = Item::deflated("a", b"hello hello hello");
        item.data.extend(noise(extra));
        item.local.compressed_size = item.data.len() as u64;
        item.central.compressed_size = item.data.len() as u64;
        let data = item.data.clone();
        let mut archive = archive(bytes(vec![item]));
        let (copied, out, found) = copy(&mut archive, 0);
        assert_eq!((copied, out, found.len()), (true, data, 0));
    }
}

#[test]
fn a_crc32_that_differs_stops_the_copy_with_a_diagnostic() {
    let mut item = Item::deflated("a", b"hello");
    item.central.crc32 ^= 1;
    item.local.crc32 ^= 1;
    let mut archive = archive(bytes(vec![item, Item::stored("b", b"x")]));
    let (copied, _, found) = copy(&mut archive, 0);
    assert!(!copied);
    assert!(matches!(&found[..], [(Diagnostic::ZipCrcMismatch { .. }, name)] if name == "a"));
    assert!(!archive.entries()[0].is_readable());
    // It does not copy again, and the other entry is not affected.
    let (copied, out, found) = copy(&mut archive, 0);
    assert_eq!((copied, out.len(), found.len()), (false, 0, 0));
    assert!(copy(&mut archive, 1).0);
}

#[test]
fn data_that_is_not_deflate_stops_the_copy_with_a_diagnostic() {
    let item = Item::stored("a", &[0xff; 8]).with_method(8);
    let mut archive = archive(bytes(vec![item]));
    let (copied, _, found) = copy(&mut archive, 0);
    let reason = InflateReason::CorruptData;
    assert!(!copied);
    assert_eq!(
        found,
        [(Diagnostic::ZipInflateFailed { reason }, "a".to_owned())]
    );
}

#[test]
fn an_entry_that_is_not_readable_or_not_there_copies_nothing() {
    let zip = bytes(vec![Item::stored("a", b"x").with_method(12)]);
    let (archive, found_open) = open(zip);
    let mut archive = archive.unwrap();
    assert_eq!(found_open.len(), 1);
    for index in [0, 1] {
        let (copied, out, found) = copy(&mut archive, index);
        assert_eq!((copied, out.len(), found.len()), (false, 0, 0));
    }
}

#[test]
fn a_writer_that_fails_is_an_error_not_a_diagnostic() {
    let mut archive = archive(bytes(vec![Item::stored("a", b"hello")]));
    let mut diagnostics = Diagnostics::new();
    let error = archive
        .copy_raw(0, &mut Broken, &mut diagnostics)
        .unwrap_err();
    assert!(matches!(error, PackageError::Io(_)));
    assert!(diagnostics.records().is_empty());
    assert!(archive.entries()[0].is_readable());
}

/// An item of the deflated "a" with `extra` bytes of noise after the end of
/// its deflate stream, within its compressed size.
fn with_trailing(extra: usize) -> Item {
    let mut item = Item::deflated("a", b"hello");
    item.data.extend(noise(extra));
    item.local.compressed_size = item.data.len() as u64;
    item.central.compressed_size = item.data.len() as u64;
    item
}

#[test]
fn a_writer_that_fails_on_the_bytes_after_the_stream_is_an_error() {
    // The first 32,768 bytes, which one read of the input takes, are
    // written; the write of the next ones, in the copy of the bytes after
    // the end of the deflate stream, fails.
    let mut archive = archive(bytes(vec![with_trailing(100_000)]));
    let mut out = Limited::new(32_768);
    let mut diagnostics = Diagnostics::new();
    let error = archive.copy_raw(0, &mut out, &mut diagnostics);
    assert!(matches!(error, Err(PackageError::Io(_))));
    assert_eq!(out.bytes.len(), 32_768);
    assert!(diagnostics.records().is_empty());
}

#[test]
fn a_reader_that_fails_on_the_bytes_after_the_stream_is_an_error() {
    // 300,000 bytes after the stream: opening the zip reads only the end of
    // the file, so the poisoned bytes are first read by the copy.
    let zip = bytes(vec![with_trailing(300_000)]);
    let start = archive(zip.clone()).entries()[0].data_range().start;
    let reader = Reader::poisoned(zip, start + 32_768..start + 65_536);
    let mut diagnostics = Diagnostics::new();
    let limits = ZipLimits::default();
    let mut archive = ZipArchive::open(reader, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    let mut out = Vec::new();
    let error = archive.copy_raw(0, &mut out, &mut diagnostics);
    assert!(matches!(error, Err(PackageError::Io(_))));
    assert_eq!(out.len(), 32_768);
    assert!(diagnostics.records().is_empty());
}

#[test]
fn an_entry_inflating_to_nothing_for_most_of_its_bytes_is_copied_in_reads_of_the_input_size() {
    // 200,000 empty non-final stored blocks (5 bytes each), then "hi": one
    // read of the entry takes all of them without yielding a byte.
    let mut data = Vec::new();
    for _ in 0..200_000 {
        data.extend([0x00, 0x00, 0x00, 0xff, 0xff]);
    }
    data.extend([0x01, 0x02, 0x00, 0xfd, 0xff, b'h', b'i']);
    let mut item = Item::stored("a", b"hi").with_method(8);
    item.local.compressed_size = data.len() as u64;
    item.central.compressed_size = data.len() as u64;
    item.data = data.clone();
    let mut archive = archive(bytes(vec![item]));
    let mut out = Recorder::default();
    let mut diagnostics = Diagnostics::new();
    assert!(archive.copy_raw(0, &mut out, &mut diagnostics).unwrap());
    assert!(diagnostics.records().is_empty());
    assert_eq!(out.bytes, data);
    // Every write is one read of the input: the whole size of it, but the
    // last.
    assert_eq!(out.largest, 32 * 1024);
    assert!(out.writes > 30);
}

#[test]
fn a_reader_that_fails_is_an_error_not_a_diagnostic() {
    let zip = bytes(vec![
        Item::stored("a", b"hello"),
        Item::deflated("b", b"hi"),
    ]);
    let mut diagnostics = Diagnostics::new();
    for index in 0..2 {
        let range = archive(zip.clone()).entries()[index].data_range();
        let reader = Reader::poisoned(zip.clone(), range);
        let limits = ZipLimits::default();
        let mut archive = ZipArchive::open(reader, &limits, &mut diagnostics)
            .unwrap()
            .unwrap();
        let mut out = Vec::new();
        let error = archive.copy_raw(index, &mut out, &mut diagnostics);
        assert!(matches!(error, Err(PackageError::Io(_))));
        assert!(out.is_empty());
    }
    assert!(diagnostics.records().is_empty());
}
