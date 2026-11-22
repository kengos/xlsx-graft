//! What a zip the writer finishes holds, read back by `ZipArchive`: the
//! entries copied and written, in the order of the calls, with the bytes and
//! the method they had, the same bytes on every run, and the fixed fields.

use super::{Archive, Sink, deflated, open, read, stored, written};
use crate::ZipWriter;
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Method, ZipArchive, ZipLimits};

/// The bytes of `archive` in `range` of the file `bytes`.
fn stored_bytes(bytes: &[u8], archive: &Archive, index: usize) -> Vec<u8> {
    let range = archive.entries()[index].data_range();
    bytes[range.start as usize..range.end as usize].to_vec()
}

/// A zip of what the two sources hold, then one entry of new content.
fn rewrite(stored_zip: &[u8], deflated_zip: &[u8]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Sink::default());
    let mut diagnostics = Diagnostics::new();
    for bytes in [stored_zip, deflated_zip] {
        let mut archive = open(bytes.to_vec());
        for index in 0..archive.entries().len() {
            assert!(
                zip.copy_entry(&mut archive, index, &mut diagnostics)
                    .unwrap()
            );
        }
    }
    zip.write_deflated(b"new.xml", deflated(&[b'x'; 5_000], 1 << 20))
        .unwrap();
    assert!(diagnostics.records().is_empty());
    zip.finish().unwrap().bytes
}

fn sources() -> (Vec<u8>, Vec<u8>) {
    let noise: Vec<u8> = (0..40_000_u32).map(|n| ((n * n) >> 3) as u8).collect();
    let stored_zip = stored(&[(b"a.txt", b"hello"), (b"empty", b""), (b"b/n.bin", &noise)]);
    let deflated_zip = written(&[(b"c.xml", &[b'c'; 9_000]), (b"d.xml", b"")]);
    (stored_zip, deflated_zip)
}

#[test]
fn copied_and_new_entries_are_read_back_in_order_with_their_bytes_and_methods() {
    let (stored_zip, deflated_zip) = sources();
    let out = rewrite(&stored_zip, &deflated_zip);
    let mut archive = open(out.clone());
    let names: Vec<&[u8]> = archive.entries().iter().map(|e| e.name()).collect();
    assert_eq!(
        names,
        [
            &b"a.txt"[..],
            b"empty",
            b"b/n.bin",
            b"c.xml",
            b"d.xml",
            b"new.xml"
        ]
    );
    let methods: Vec<Method> = archive.entries().iter().map(|e| e.method()).collect();
    use Method::{Deflate, Stored};
    assert_eq!(methods, [Stored, Stored, Stored, Deflate, Deflate, Deflate]);
    assert_eq!(read(&mut archive, 0), b"hello");
    assert_eq!(read(&mut archive, 1), b"");
    assert_eq!(read(&mut archive, 3), [b'c'; 9_000]);
    assert_eq!(read(&mut archive, 4), b"");
    assert_eq!(read(&mut archive, 5), [b'x'; 5_000]);

    // The stored bytes are the sources', as they are.
    let sources = [open(stored_zip.clone()), open(deflated_zip.clone())];
    let from = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1)];
    for (index, (source, at)) in from.into_iter().enumerate() {
        let bytes = if source == 0 {
            &stored_zip
        } else {
            &deflated_zip
        };
        let theirs = stored_bytes(bytes, &sources[source], at);
        let ours = stored_bytes(&out, &archive, index);
        assert_eq!(ours, theirs);
        let (a, b) = (&archive.entries()[index], &sources[source].entries()[at]);
        assert_eq!((a.crc32(), a.size()), (b.crc32(), b.size()));
        assert_eq!(a.compressed_size(), b.compressed_size());
    }
}

#[test]
fn the_same_calls_write_the_same_bytes() {
    let (stored_zip, deflated_zip) = sources();
    let first = rewrite(&stored_zip, &deflated_zip);
    assert_eq!(first, rewrite(&stored_zip, &deflated_zip));
    assert_eq!(
        written(&[(b"a", b"hello hello")]),
        written(&[(b"a", b"hello hello")])
    );
}

#[test]
fn the_fixed_fields_are_in_every_header() {
    let (stored_zip, deflated_zip) = sources();
    let out = rewrite(&stored_zip, &deflated_zip);
    let archive = open(out.clone());
    assert_eq!(archive.entries().len(), 6);
    let field = |at: usize, len: usize| &out[at..at + len];
    // The end record: the last 22 bytes, with no comment.
    let end = out.len() - 22;
    assert_eq!(field(end, 4), b"PK\x05\x06");
    assert_eq!(field(end + 20, 2), [0, 0]);
    let directory = u32::from_le_bytes(field(end + 16, 4).try_into().unwrap()) as usize;
    let mut at = directory;
    let mut locals = Vec::new();
    while at < end {
        assert_eq!(field(at, 4), b"PK\x01\x02");
        assert_eq!(field(at + 4, 4), [45, 0, 20, 0]);
        // Time, date.
        assert_eq!(field(at + 12, 4), [0, 0, 0x21, 0]);
        let len =
            |from: usize| usize::from(u16::from_le_bytes(field(at + from, 2).try_into().unwrap()));
        assert_eq!((len(30), len(32)), (0, 0));
        assert_eq!(field(at + 34, 8), [0; 8]);
        locals.push(u32::from_le_bytes(field(at + 42, 4).try_into().unwrap()) as usize);
        at += 46 + len(28);
    }
    assert_eq!((at, locals.len()), (end, 6));
    for local in locals {
        assert_eq!(field(local, 4), b"PK\x03\x04");
        assert_eq!(field(local + 4, 4), [20, 0, 0, 0]);
        assert_eq!(field(local + 10, 4), [0, 0, 0x21, 0]);
        assert_eq!(field(local + 28, 2), [0, 0]);
    }
}

#[test]
fn a_name_is_written_as_given_and_utf8_is_flagged() {
    let names: [&[u8]; 3] = ["é.xml".as_bytes(), b"plain", &[0xe9, b'.']];
    let out = written(&[(names[0], b"1"), (names[1], b"2"), (names[2], b"3")]);
    let mut diagnostics = Diagnostics::new();
    let limits = ZipLimits::default();
    let archive = ZipArchive::open(Cursor::new(out.clone()), &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    let read: Vec<&[u8]> = archive.entries().iter().map(|e| e.name()).collect();
    assert_eq!(read, names);
    // The flags of the three local headers, in order.
    let flags: Vec<u8> = out
        .windows(4)
        .enumerate()
        .filter(|(_, w)| w == b"PK\x03\x04")
        .map(|(at, _)| out[at + 7])
        .collect();
    assert_eq!(flags, [0x08, 0, 0]);
}

#[test]
fn an_empty_zip_is_its_end_record() {
    let out = ZipWriter::new(Sink::default()).finish().unwrap().bytes;
    assert_eq!(out, b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
}
