//! Tests for `ZipArchive`, driven through `open`, `entries` and
//! `open_entry` on zips the builder lays out byte by byte. Each file under
//! `tests/` covers one aspect of what the reader refuses or admits.

mod builder;
mod directory;
mod end;
mod entries;
mod reading;
mod sizes;

use crate::{ZipArchive, ZipLimits};
use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit};

/// The reader every test reads a zip from: bytes in memory that fail every
/// read and seek while `failing` is set.
///
/// One reader type for every test, the failing ones included: the coverage
/// gate counts a line of code generic over the reader as covered only when
/// one instantiation covers it, so a second reader type would leave the
/// lines only it reaches uncovered.
#[derive(Debug)]
struct Reader {
    inner: Cursor<Vec<u8>>,
    failing: Rc<Cell<bool>>,
    /// How many bytes were read.
    read: Rc<Cell<u64>>,
}

impl Reader {
    /// `bytes`, read without failing.
    fn new(bytes: Vec<u8>) -> Self {
        Self::failing_with(bytes, Rc::default())
    }

    /// `bytes`, failing while `failing` is set.
    fn failing_with(bytes: Vec<u8>, failing: Rc<Cell<bool>>) -> Self {
        Self {
            inner: Cursor::new(bytes),
            failing,
            read: Rc::default(),
        }
    }

    /// `bytes`, counting in `read` how many bytes are read.
    fn counted(bytes: Vec<u8>, read: Rc<Cell<u64>>) -> Self {
        Self {
            read,
            ..Self::new(bytes)
        }
    }

    fn check(&self) -> io::Result<()> {
        if self.failing.get() {
            Err(io::Error::other("the disk went away"))
        } else {
            Ok(())
        }
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.check()?;
        let len = self.inner.read(buf)?;
        self.read.set(self.read.get() + len as u64);
        Ok(len)
    }
}

impl Seek for Reader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.check()?;
        self.inner.seek(pos)
    }
}

/// A zip read by the tests.
type Archive = ZipArchive<Reader>;

/// What opening a zip found: each diagnostic, with the entry it points to
/// (empty for the archive as a whole).
type Found = Vec<(Diagnostic, String)>;

/// Opens `bytes` under `limits`.
fn open_with(bytes: Vec<u8>, limits: &ZipLimits) -> (Option<Archive>, Found) {
    let mut diagnostics = Diagnostics::new();
    let archive = ZipArchive::open(Reader::new(bytes), limits, &mut diagnostics).unwrap();
    (archive, found(diagnostics))
}

/// [`open_with`] under the default limits.
fn open(bytes: Vec<u8>) -> (Option<Archive>, Found) {
    open_with(bytes, &ZipLimits::default())
}

/// The diagnostics in `diagnostics`, each with the entry it points to.
fn found(diagnostics: Diagnostics) -> Found {
    diagnostics
        .finish()
        .into_iter()
        .map(|record| {
            let entry = record.location.entry.map(|e| e.as_str().to_owned());
            (record.diagnostic, entry.unwrap_or_default())
        })
        .collect()
}

/// The one diagnostic that keeps `bytes`' central directory from being read.
fn unreadable(bytes: Vec<u8>) -> Diagnostic {
    unreadable_with(bytes, &ZipLimits::default())
}

/// [`unreadable`] under `limits`.
fn unreadable_with(bytes: Vec<u8>, limits: &ZipLimits) -> Diagnostic {
    match open_with(bytes, limits) {
        (None, mut found) if found.len() == 1 => found.remove(0).0,
        other => panic!("expected one diagnostic and no archive, got {other:?}"),
    }
}

/// Opens `bytes`, asserting the directory is read, and returns which
/// entries are readable and what was found.
fn readable(bytes: Vec<u8>, limits: &ZipLimits) -> (Vec<bool>, Found) {
    let (archive, found) = open_with(bytes, limits);
    let archive = archive.expect("the central directory should be read");
    let readable = archive.entries().iter().map(|e| e.is_readable()).collect();
    (readable, found)
}

/// `value` as a default limit.
fn limit(value: u64) -> Limit {
    Limit::default_of(value)
}

/// `len` bytes that deflate poorly.
fn noise(len: usize) -> Vec<u8> {
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
