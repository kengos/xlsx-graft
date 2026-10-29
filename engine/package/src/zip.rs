//! [`ZipArchive`]: a zip's entries, read from its central directory under
//! limits and checked against their local headers before any is inflated.
//!
//! [`ZipArchive::open`] finds the end records (`end.rs`), reads the central
//! directory (`directory.rs`), then checks each entry in passes: its own
//! fields (`checks.rs`), its place in the layout of the file (`layout.rs`)
//! with its local record (`local.rs`), and its sizes (`checks.rs`). An entry
//! a pass refuses gets that pass's diagnostic and is skipped by the later
//! passes, so one diagnostic refuses it however much is wrong with it.

mod checks;
mod directory;
mod end;
mod layout;
mod local;
mod reader;
mod source;
mod stream;

pub use reader::EntryReader;
pub use stream::EntryStream;

use crate::{PackageError, ZipLimits};
use source::Source;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};

/// A field the zip64 extra field holds instead (APPNOTE 6.3.10 §4.4.1.4).
const SATURATED_32: u32 = u32::MAX;
/// The disk number field's value when the zip64 extra field holds it.
const SATURATED_16: u16 = u16::MAX;

/// How an entry's data is compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Stored as is (method 0).
    Stored,
    /// Deflated (method 8).
    Deflate,
    /// Any other method, which a package may not use; such an entry is not
    /// readable.
    Other(u16),
}

impl Method {
    fn from_raw(raw: u16) -> Self {
        match raw {
            0 => Self::Stored,
            8 => Self::Deflate,
            other => Self::Other(other),
        }
    }
}

/// One entry of the central directory.
///
/// A readable entry has passed every check: its local header agrees with
/// the central directory, its data lies within the file without overlapping
/// another's, and its sizes are within the limits. Every entry that is not
/// readable has had one diagnostic of its own, except the entries after the
/// one that took the declared total past its limit: that one's
/// `zip_total_too_large` stands for them all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    name: Vec<u8>,
    flags: u16,
    method: Method,
    crc32: u32,
    compressed_size: u64,
    size: u64,
    local_offset: u64,
    /// Where the data starts; known once the local header is read.
    data_offset: u64,
    /// Where the entry ends, past its data descriptor if it has one.
    end_offset: u64,
    readable: bool,
}

impl Entry {
    /// The name as the central directory holds it, which need not be UTF-8.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// How the data is compressed.
    #[must_use]
    pub fn method(&self) -> Method {
        self.method
    }

    /// The CRC-32 of the inflated data, as the central directory gives it.
    #[must_use]
    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    /// The size of the data as stored, in bytes.
    #[must_use]
    pub fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    /// The inflated size the central directory declares, in bytes.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Whether the entry passed every check, so it can be opened.
    #[must_use]
    pub fn is_readable(&self) -> bool {
        self.readable
    }

    /// Where the entry's stored (compressed) data lies in the file, for a
    /// readable entry; empty for one that is not.
    #[must_use]
    pub fn data_range(&self) -> Range<u64> {
        if self.readable {
            self.data_offset..self.data_offset.saturating_add(self.compressed_size)
        } else {
            0..0
        }
    }

    /// Where a diagnostic about this entry points.
    fn location(&self) -> Location {
        Location::default().with_entry_bytes(&self.name)
    }

    /// Records `diagnostic` against this entry, which is not read from then on.
    #[track_caller]
    fn refuse(&mut self, diagnostic: Diagnostic, diagnostics: &mut Diagnostics) {
        diagnostics.push(diagnostic, self.location());
        self.readable = false;
    }
}

/// A zip read from `R`: its entries, in central directory order, and the
/// reader to inflate them from.
#[derive(Debug)]
pub struct ZipArchive<R> {
    source: Source<R>,
    entries: Vec<Entry>,
}

impl<R: Read + Seek> ZipArchive<R> {
    /// Reads the zip's central directory and checks every entry, pushing a
    /// diagnostic for each problem found.
    ///
    /// `Ok(None)` when the central directory cannot be read, which the
    /// diagnostics say why; an entry that cannot be read is still listed,
    /// as not readable.
    ///
    /// # Errors
    ///
    /// [`PackageError::Io`] when reading or seeking `reader` fails.
    pub fn open(
        reader: R,
        limits: &ZipLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, PackageError> {
        let mut source = Source::new(reader)?;
        let end = match end::find(&mut source, limits)? {
            Ok(end) => end,
            Err(diagnostic) => return Ok(Self::unreadable(diagnostic, diagnostics)),
        };
        let mut entries = match directory::read(&mut source, &end)? {
            Ok(entries) => entries,
            Err(diagnostic) => return Ok(Self::unreadable(diagnostic, diagnostics)),
        };
        checks::fields(&mut entries, limits, diagnostics);
        layout::check(&mut source, &mut entries, end.offset, diagnostics)?;
        checks::sizes(&mut entries, limits, diagnostics);
        Ok(Some(Self { source, entries }))
    }

    /// Records why the central directory cannot be read.
    #[track_caller]
    fn unreadable(diagnostic: Diagnostic, diagnostics: &mut Diagnostics) -> Option<Self> {
        diagnostics.push(diagnostic, Location::default());
        None
    }

    /// Every entry, in central directory order, readable or not.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The index of the first entry named exactly `name`, byte for byte.
    #[must_use]
    pub fn position(&self, name: &[u8]) -> Option<usize> {
        self.entries.iter().position(|entry| entry.name == name)
    }

    /// A stream of the inflated data of the entry at `index`, read from
    /// this archive through [`EntryStream::read`], or `None` when there is
    /// no such entry or it is not readable (the diagnostic that says why is
    /// already pushed).
    #[must_use]
    pub fn stream_entry(&self, index: usize) -> Option<EntryStream> {
        let entry = self.entries.get(index).filter(|entry| entry.readable)?;
        Some(EntryStream::new(index, entry))
    }

    /// A reader of the inflated data of the entry at `index`, or `None` when
    /// there is no such entry or it is not readable (the diagnostic that says
    /// why is already pushed).
    ///
    /// # Errors
    ///
    /// [`PackageError::Io`] when seeking to the entry's data fails.
    pub fn open_entry<'a>(
        &'a mut self,
        index: usize,
        diagnostics: &'a mut Diagnostics,
    ) -> Result<Option<EntryReader<'a, R>>, PackageError> {
        let Some(stream) = self.stream_entry(index) else {
            return Ok(None);
        };
        // Seek now, so a host that cannot reach the data fails here rather
        // than at the first read.
        let offset = self.entries.get(index).map_or(0, |entry| entry.data_offset);
        self.source.reader.seek(SeekFrom::Start(offset))?;
        Ok(Some(EntryReader::new(self, diagnostics, stream)))
    }
}
