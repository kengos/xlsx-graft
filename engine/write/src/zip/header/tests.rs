//! The header functions on plain values: the fields every header holds, the
//! flags for a name, and where each size, offset and count turns zip64 on,
//! and a zip with zip64 entries read back by `ZipArchive`.

use super::{Record, central, end, local};
use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};
use xlsx_graft_diagnostics::{Diagnostics, Limit};
use xlsx_graft_package::{ZipArchive, ZipLimits};

const MAX: u64 = u32::MAX as u64;

fn record(size: u64, compressed_size: u64, offset: u64) -> Record {
    Record {
        name: b"a".to_vec(),
        method: 8,
        crc32: 0x1234_5678,
        compressed_size,
        size,
        offset,
    }
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

#[test]
fn a_local_header_holds_the_fixed_fields() {
    let header = local(&record(10, 7, 99));
    assert_eq!(header.len(), 31);
    assert_eq!(u32_at(&header, 0), 0x0403_4b50);
    assert_eq!(u16_at(&header, 4), 20);
    assert_eq!(u16_at(&header, 6), 0);
    assert_eq!(u16_at(&header, 8), 8);
    // 1980-01-01 00:00:00.
    assert_eq!((u16_at(&header, 10), u16_at(&header, 12)), (0, 0x0021));
    assert_eq!(u32_at(&header, 14), 0x1234_5678);
    assert_eq!((u32_at(&header, 18), u32_at(&header, 22)), (7, 10));
    assert_eq!((u16_at(&header, 26), u16_at(&header, 28)), (1, 0));
    assert_eq!(header[30], b'a');
}

#[test]
fn a_central_header_holds_the_fixed_fields() {
    let header = central(&record(10, 7, 99));
    assert_eq!(header.len(), 47);
    assert_eq!(u32_at(&header, 0), 0x0201_4b50);
    // Version made by 4.5, MS-DOS; version needed 2.0.
    assert_eq!((u16_at(&header, 4), u16_at(&header, 6)), (45, 20));
    assert_eq!((u16_at(&header, 8), u16_at(&header, 10)), (0, 8));
    assert_eq!((u16_at(&header, 12), u16_at(&header, 14)), (0, 0x0021));
    assert_eq!(u32_at(&header, 16), 0x1234_5678);
    assert_eq!((u32_at(&header, 20), u32_at(&header, 24)), (7, 10));
    // Name length, then extra, comment, disk, internal, external: 0.
    assert_eq!(u16_at(&header, 28), 1);
    assert_eq!(header[30..42], [0; 12]);
    assert_eq!(u32_at(&header, 42), 99);
    assert_eq!(header[46], b'a');
}

#[test]
fn bit_11_is_set_for_a_utf8_name_with_a_non_ascii_byte_only() {
    let flags = |name: &[u8]| {
        let record = Record {
            name: name.to_vec(),
            ..record(0, 0, 0)
        };
        (u16_at(&local(&record), 6), u16_at(&central(&record), 8))
    };
    assert_eq!(flags(b"xl/a.xml"), (0, 0));
    assert_eq!(flags(b""), (0, 0));
    assert_eq!(flags("xl/é.xml".as_bytes()), (0x0800, 0x0800));
    // A lone continuation byte, a cut sequence, and Latin-1.
    assert_eq!(flags(&[0x80]), (0, 0));
    assert_eq!(flags(&[b'a', 0xc3]), (0, 0));
    assert_eq!(flags(&[0xe9]), (0, 0));
}

#[test]
fn a_size_below_the_saturated_value_needs_no_zip64() {
    for record in [
        record(MAX - 1, 0, 0),
        record(0, MAX - 1, 0),
        record(0, 0, MAX - 1),
    ] {
        let (local, central) = (local(&record), central(&record));
        assert_eq!((local.len(), central.len()), (31, 47));
        assert_eq!((u16_at(&local, 4), u16_at(&central, 6)), (20, 20));
        assert_eq!((u16_at(&local, 28), u16_at(&central, 30)), (0, 0));
    }
}

#[test]
fn a_wide_size_puts_both_sizes_in_the_local_header_and_the_wide_ones_in_the_central() {
    // Size wide: the local header holds both sizes, the central one the size.
    let size = record(MAX, 5, 7);
    let first = local(&size);
    assert_eq!(u16_at(&first, 4), 45);
    assert_eq!(
        (u32_at(&first, 18), u32_at(&first, 22)),
        (u32::MAX, u32::MAX)
    );
    assert_eq!(u16_at(&first, 28), 20);
    assert_eq!(first[31..35], [1, 0, 16, 0]);
    assert_eq!((u64_at(&first, 35), u64_at(&first, 43)), (MAX, 5));
    let second = central(&size);
    assert_eq!((u16_at(&second, 6), u16_at(&second, 30)), (45, 12));
    assert_eq!((u32_at(&second, 20), u32_at(&second, 24)), (5, u32::MAX));
    assert_eq!(u32_at(&second, 42), 7);
    assert_eq!(second[47..51], [1, 0, 8, 0]);
    assert_eq!(u64_at(&second, 51), MAX);

    // Compressed size wide: in the local header the same two fields, in
    // the central one the compressed size alone.
    let compressed = record(5, MAX + 3, 7);
    let first = local(&compressed);
    assert_eq!(
        (u32_at(&first, 18), u32_at(&first, 22)),
        (u32::MAX, u32::MAX)
    );
    assert_eq!((u64_at(&first, 35), u64_at(&first, 43)), (5, MAX + 3));
    let second = central(&compressed);
    assert_eq!((u32_at(&second, 20), u32_at(&second, 24)), (u32::MAX, 5));
    assert_eq!(u64_at(&second, 51), MAX + 3);
}

#[test]
fn a_wide_offset_is_in_the_central_header_only() {
    let wide = record(5, 6, MAX);
    let first = local(&wide);
    // The local header has no offset, so no extra field, but it needs 4.5
    // as the central header does.
    assert_eq!(
        (first.len(), u16_at(&first, 4), u16_at(&first, 28)),
        (31, 45, 0)
    );
    let second = central(&wide);
    assert_eq!((u16_at(&second, 6), u16_at(&second, 30)), (45, 12));
    assert_eq!(u32_at(&second, 42), u32::MAX);
    assert_eq!(u64_at(&second, 51), MAX);
}

#[test]
fn the_central_extra_field_holds_the_wide_values_in_the_order_of_the_spec() {
    let wide = record(MAX + 1, MAX + 2, MAX + 3);
    let second = central(&wide);
    assert_eq!(u16_at(&second, 30), 4 + 24);
    assert_eq!(second[47..51], [1, 0, 24, 0]);
    assert_eq!(
        [
            u64_at(&second, 51),
            u64_at(&second, 59),
            u64_at(&second, 67)
        ],
        [MAX + 1, MAX + 2, MAX + 3]
    );
    assert_eq!(
        [
            u32_at(&second, 20),
            u32_at(&second, 24),
            u32_at(&second, 42)
        ],
        [u32::MAX; 3]
    );
}

#[test]
fn the_end_record_alone_ends_a_small_zip() {
    for count in [0, 1, 0xFFFE] {
        let end = end(count, MAX - 1, 0);
        assert_eq!(end.len(), 22);
        assert_eq!(u32_at(&end, 0), 0x0605_4b50);
        assert_eq!(end[4..8], [0; 4]);
        assert_eq!(
            (u16_at(&end, 8), u16_at(&end, 10)),
            (count as u16, count as u16)
        );
        assert_eq!(u32_at(&end, 12), 0);
        assert_eq!(u32_at(&end, 16), (MAX - 1) as u32);
        assert_eq!(u16_at(&end, 20), 0);
    }
    let end = end(2, 10, MAX - 1);
    assert_eq!(end.len(), 22);
    assert_eq!((u32_at(&end, 12), u32_at(&end, 16)), ((MAX - 1) as u32, 10));
}

#[test]
fn a_count_a_size_or_an_offset_at_the_saturated_value_adds_the_zip64_records() {
    for (count, offset, size) in [(0xFFFF, 10, 20), (3, MAX, 20), (3, 10, MAX)] {
        let end = end(count, offset, size);
        assert_eq!(end.len(), 56 + 20 + 22);
        // The zip64 end record.
        assert_eq!(u32_at(&end, 0), 0x0606_4b50);
        assert_eq!(u64_at(&end, 4), 44);
        assert_eq!((u16_at(&end, 12), u16_at(&end, 14)), (45, 45));
        assert_eq!(end[16..24], [0; 8]);
        assert_eq!((u64_at(&end, 24), u64_at(&end, 32)), (count, count));
        assert_eq!((u64_at(&end, 40), u64_at(&end, 48)), (size, offset));
        // The locator: the zip64 record follows the directory.
        assert_eq!(u32_at(&end, 56), 0x0706_4b50);
        assert_eq!(u32_at(&end, 60), 0);
        assert_eq!(u64_at(&end, 64), offset + size);
        assert_eq!(u32_at(&end, 72), 1);
        // The end record: saturated where the value does not fit.
        assert_eq!(u32_at(&end, 76), 0x0605_4b50);
        assert_eq!(end[80..84], [0; 4]);
        let fits = |value: u64| u16::try_from(value).unwrap_or(u16::MAX);
        assert_eq!(
            (u16_at(&end, 84), u16_at(&end, 86)),
            (fits(count), fits(count))
        );
        let fits = |value: u64| u32::try_from(value).unwrap_or(u32::MAX);
        assert_eq!(
            (u32_at(&end, 88), u32_at(&end, 92)),
            (fits(size), fits(offset))
        );
        assert_eq!(u16_at(&end, 96), 0);
    }
}

#[test]
fn the_end_record_keeps_the_values_that_fit_beside_the_zip64_records() {
    let big = end(0x10000, 10, 20);
    assert_eq!((u16_at(&big, 84), u16_at(&big, 86)), (0xFFFF, 0xFFFF));
    assert_eq!((u32_at(&big, 88), u32_at(&big, 92)), (20, 10));
    let few = end(3, MAX + 5, 20);
    assert_eq!((u16_at(&few, 84), u16_at(&few, 86)), (3, 3));
}

/// A file that is zeros but for the byte ranges in `map`, as large as
/// `len`: a zip of entries of 4 GiB without 4 GiB in memory.
struct Sparse {
    len: u64,
    map: BTreeMap<u64, Vec<u8>>,
    position: u64,
}

impl Read for Sparse {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.len.saturating_sub(self.position);
        let len = usize::try_from(left).map_or(buf.len(), |left| left.min(buf.len()));
        let (buf, _) = buf.split_at_mut(len);
        buf.fill(0);
        let stop = self.position + len as u64;
        for (&start, bytes) in &self.map {
            let from = start.max(self.position);
            let to = (start + bytes.len() as u64).min(stop);
            if from < to {
                let into = (from - self.position) as usize..(to - self.position) as usize;
                let source = (from - start) as usize..(to - start) as usize;
                buf[into].copy_from_slice(&bytes[source]);
            }
        }
        self.position = stop;
        Ok(len)
    }
}

impl Seek for Sparse {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.position = match from {
            SeekFrom::Start(at) => at,
            SeekFrom::End(delta) => self.len.checked_add_signed(delta).unwrap(),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta).unwrap(),
        };
        Ok(self.position)
    }
}

#[test]
fn entries_at_4_gib_are_read_back_with_their_zip64_fields() {
    let gib4 = 1_u64 << 32;
    // A stored entry of 4 GiB (zip64 sizes in its headers), then a small one
    // that starts past 4 GiB (a zip64 offset in its central header).
    let big = Record {
        name: b"big".to_vec(),
        method: 0,
        crc32: 0,
        compressed_size: gib4,
        size: gib4,
        offset: 0,
    };
    let big_local = local(&big);
    let small = Record {
        name: b"small".to_vec(),
        method: 0,
        crc32: crc32fast::hash(b"hi"),
        compressed_size: 2,
        size: 2,
        offset: big_local.len() as u64 + gib4,
    };
    let small_local = [local(&small), b"hi".to_vec()].concat();
    let directory = small.offset + small_local.len() as u64;
    let mut tail = [central(&big), central(&small)].concat();
    let directory_size = tail.len() as u64;
    tail.extend(end(2, directory, directory_size));
    let len = directory + tail.len() as u64;
    let map = BTreeMap::from([
        (0, big_local.clone()),
        (small.offset, small_local),
        (directory, tail),
    ]);
    let reader = Sparse {
        len,
        map,
        position: 0,
    };
    let limits = ZipLimits {
        max_entry_bytes: Limit::policy_of(gib4 * 2),
        max_total_bytes: Limit::policy_of(gib4 * 4),
        ..ZipLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let archive = ZipArchive::open(reader, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    assert!(
        diagnostics.records().is_empty(),
        "{:?}",
        diagnostics.records()
    );
    let entries = archive.entries();
    assert!(entries.iter().all(|entry| entry.is_readable()));
    let first = big_local.len() as u64;
    assert_eq!(entries[0].data_range(), first..first + gib4);
    assert_eq!(
        (entries[0].size(), entries[0].compressed_size()),
        (gib4, gib4)
    );
    let second = small.offset + small_local_len(&small);
    assert_eq!(entries[1].data_range(), second..second + 2);
}

/// The length of `record`'s local header.
fn small_local_len(record: &Record) -> u64 {
    local(record).len() as u64
}
