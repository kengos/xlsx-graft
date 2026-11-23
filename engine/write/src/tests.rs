//! Tests for the writer, driven through its public API, on zips it writes
//! and reads back with `xlsx_graft_package`. `round_trip` covers what a
//! zip it writes holds, `many_entries` an archive past 65,535 entries,
//! `deflate` the content writer, `failures` what is refused and the
//! failures of the host, and `structure` the rules for the names a
//! structure edit writes. The header bytes are tested on plain values in
//! `zip/header/tests.rs`.

mod deflate;
mod failures;
mod many_entries;
mod round_trip;
mod structure;

use crate::{DeflateWriter, Deflated, ZipWriter};
use std::io::{self, Cursor, Read, Write};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{ZipArchive, ZipLimits};
use xlsx_graft_spool::MemorySpools;

/// The writer every test writes a zip to: bytes in memory that fail once
/// `fail_after` bytes were written.
///
/// One writer type for every test, the failing ones included: the coverage
/// gate counts a line of code generic over the writer as covered only when
/// one instantiation covers it.
#[derive(Debug, Default)]
pub(crate) struct Sink {
    pub(crate) bytes: Vec<u8>,
    pub(crate) fail_after: Option<usize>,
}

impl Sink {
    pub(crate) fn failing_after(fail_after: usize) -> Self {
        Self {
            bytes: Vec::new(),
            fail_after: Some(fail_after),
        }
    }
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let room = self
            .fail_after
            .map_or(buf.len(), |limit| limit.saturating_sub(self.bytes.len()));
        if room == 0 && !buf.is_empty() {
            return Err(io::Error::other("the disk is full"));
        }
        let len = buf.len().min(room);
        self.bytes.extend_from_slice(&buf[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A zip read by the tests.
pub(crate) type Archive = ZipArchive<Cursor<Vec<u8>>>;

/// `len` bytes that deflate poorly.
pub(crate) fn noise(len: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_u32;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()[0]
        })
        .collect()
}

/// `data` deflated, spilling once the output passes `spill_at`.
pub(crate) fn deflated(data: &[u8], spill_at: u64) -> Deflated {
    let mut spools = MemorySpools;
    let mut writer = DeflateWriter::new(&mut spools, spill_at);
    writer.write_all(data).unwrap();
    writer.finish().unwrap()
}

/// A zip of the new entries `entries`, deflated, and read from nothing.
pub(crate) fn written(entries: &[(&[u8], &[u8])]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Sink::default());
    for (name, data) in entries {
        zip.write_deflated(name, deflated(data, 1 << 20)).unwrap();
    }
    zip.finish().unwrap().bytes
}

/// A zip of entries stored as they are, laid out by hand: the source of
/// stored entries for the copy.
pub(crate) fn stored(entries: &[(&[u8], &[u8])]) -> Vec<u8> {
    let (mut out, mut central) = (Vec::new(), Vec::new());
    for (name, data) in entries {
        let crc = crc32fast::hash(data);
        let (len, name_len) = (data.len() as u32, name.len() as u16);
        let fixed = |out: &mut Vec<u8>| {
            out.extend([0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(crc.to_le_bytes());
            out.extend(len.to_le_bytes());
            out.extend(len.to_le_bytes());
            out.extend(name_len.to_le_bytes());
        };
        let offset = out.len() as u32;
        out.extend(b"PK\x03\x04\x0a\x00");
        fixed(&mut out);
        out.extend([0, 0]);
        out.extend(*name);
        out.extend(*data);
        central.extend(b"PK\x01\x02\x14\x00\x0a\x00");
        fixed(&mut central);
        central.extend([0; 2 + 2 + 2 + 2 + 4]);
        central.extend(offset.to_le_bytes());
        central.extend(*name);
    }
    let directory = out.len() as u32;
    let (count, size) = (entries.len() as u16, central.len() as u32);
    out.extend(central);
    out.extend(b"PK\x05\x06\0\0\0\0");
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(directory.to_le_bytes());
    out.extend([0, 0]);
    out
}

/// Opens `bytes` under `limits`, which must give no diagnostic.
pub(crate) fn open_with(bytes: Vec<u8>, limits: &ZipLimits) -> Archive {
    let mut diagnostics = Diagnostics::new();
    let archive = ZipArchive::open(Cursor::new(bytes), limits, &mut diagnostics).unwrap();
    assert!(
        diagnostics.records().is_empty(),
        "{:?}",
        diagnostics.records()
    );
    archive.unwrap()
}

/// [`open_with`] under the default limits.
pub(crate) fn open(bytes: Vec<u8>) -> Archive {
    open_with(bytes, &ZipLimits::default())
}

/// The inflated data of the entry at `index`.
pub(crate) fn read(archive: &mut Archive, index: usize) -> Vec<u8> {
    let mut diagnostics = Diagnostics::new();
    let mut entry = archive
        .open_entry(index, &mut diagnostics)
        .unwrap()
        .unwrap();
    let mut data = Vec::new();
    entry.read_to_end(&mut data).unwrap();
    assert!(diagnostics.records().is_empty());
    data
}
