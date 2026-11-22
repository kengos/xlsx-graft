//! [`ZipWriter`]: the entries of a zip written one after another, then its
//! central directory. The headers' bytes are `zip/header.rs`.

mod header;

use crate::deflate::Deflated;
use crate::error::WriteError;
use header::Record;
use std::io::{Read, Seek, Write};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Method, ZipArchive};

/// How many bytes of deflated content are read from the buffer at a time.
const COPY_CHUNK: usize = 65_536;

/// A zip written to `W`, entry by entry in the order the calls come, and
/// ended by [`finish`](Self::finish). The crate root says what it writes.
///
/// Once a call fails (`Err`), or [`copy_entry`](Self::copy_entry) refuses an
/// entry after its local header was written (`Ok(false)`), what was written
/// ends in a partial entry and is not a zip: every later call returns
/// [`WriteError::Unusable`], and the caller drops the output.
#[derive(Debug)]
pub struct ZipWriter<W> {
    out: W,
    /// How many bytes were written to `out`.
    offset: u64,
    records: Vec<Record>,
    /// Whether what was written is not a zip.
    failed: bool,
}

impl<W: Write> ZipWriter<W> {
    /// A writer of a zip that starts at the first byte written to `out`.
    #[must_use]
    pub fn new(out: W) -> Self {
        Self {
            out,
            offset: 0,
            records: Vec::new(),
            failed: false,
        }
    }

    /// Writes the entry at `index` of `archive` as the next entry, its
    /// stored bytes as they are and its method kept, checking them as
    /// [`ZipArchive::copy_raw`] does.
    ///
    /// `Ok(false)` in two cases. There is no such entry or it is not
    /// readable: nothing was written, the writer is as it was, and the
    /// diagnostic that says why was pushed by the archive already. Or the
    /// entry was refused while it was copied (`zip_inflate_failed` or
    /// `zip_crc_mismatch` is pushed): a partial entry was written, and the
    /// writer is unusable from then on.
    ///
    /// # Errors
    ///
    /// [`WriteError::Io`] when reading `archive` or writing the zip fails;
    /// [`WriteError::Unusable`] after an earlier failure. When the
    /// [`std::io::Error`] that `out` returns carries a [`WriteError`] (as
    /// [`DeflateWriter`](crate::DeflateWriter)'s do), that `WriteError`.
    pub fn copy_entry<R: Read + Seek>(
        &mut self,
        archive: &mut ZipArchive<R>,
        index: usize,
        diagnostics: &mut Diagnostics,
    ) -> Result<bool, WriteError> {
        self.check_usable()?;
        let Some(entry) = archive.entries().get(index).filter(|e| e.is_readable()) else {
            return Ok(false);
        };
        let record = Record {
            name: entry.name().to_vec(),
            // A readable entry is stored or deflated.
            method: if entry.method() == Method::Stored {
                0
            } else {
                8
            },
            crc32: entry.crc32(),
            compressed_size: entry.compressed_size(),
            size: entry.size(),
            offset: self.offset,
        };
        self.write_local(&record)?;
        let copied = archive.copy_raw(index, &mut self.out, diagnostics);
        let copied = self.track(copied.map_err(WriteError::from))?;
        self.failed = !copied;
        if copied {
            self.offset = self.offset.saturating_add(record.compressed_size);
            self.records.push(record);
        }
        Ok(copied)
    }

    /// Writes `deflated`, the result of [`DeflateWriter::finish`], as the
    /// next entry, named `name`.
    ///
    /// # Errors
    ///
    /// [`WriteError::EntryNameTooLong`] for a name of more than 65,535
    /// bytes (nothing was written, the writer is as it was, and `deflated`
    /// is dropped); [`WriteError::Io`] when writing the zip fails;
    /// [`WriteError::Spool`] when reading `deflated` back from its scratch
    /// storage fails; [`WriteError::Unusable`] after an earlier failure. When
    /// the [`std::io::Error`] that `out` returns carries a [`WriteError`] (as
    /// [`DeflateWriter`](crate::DeflateWriter)'s do), that `WriteError`.
    ///
    /// [`DeflateWriter::finish`]: crate::DeflateWriter::finish
    pub fn write_deflated(
        &mut self,
        name: &[u8],
        mut deflated: Deflated,
    ) -> Result<(), WriteError> {
        self.check_usable()?;
        if name.len() > usize::from(u16::MAX) {
            return Err(WriteError::EntryNameTooLong { length: name.len() });
        }
        let record = Record {
            name: name.to_vec(),
            method: 8,
            crc32: deflated.crc32,
            compressed_size: deflated.buffer.len(),
            size: deflated.size,
            offset: self.offset,
        };
        self.write_local(&record)?;
        // `compressed_size` is `deflated.buffer.len()`, so reading until a
        // read returns 0 reads every byte of it.
        let mut chunk = vec![0; COPY_CHUNK];
        let mut at = 0_u64;
        loop {
            let read = deflated.buffer.read_at(at, &mut chunk);
            let read = self.track(read.map_err(WriteError::from))?;
            if read == 0 {
                break;
            }
            let written = self.out.write_all(chunk.get(..read).unwrap_or_default());
            self.track(written.map_err(WriteError::from))?;
            at = at.saturating_add(read as u64);
        }
        self.offset = self.offset.saturating_add(record.compressed_size);
        self.records.push(record);
        Ok(())
    }

    /// Writes the central directory and the end records, and returns the
    /// writer `out` the zip was written to.
    ///
    /// # Errors
    ///
    /// [`WriteError::Io`] when writing the zip fails;
    /// [`WriteError::Unusable`] after an earlier failure. When the
    /// [`std::io::Error`] that `out` returns carries a [`WriteError`] (as
    /// [`DeflateWriter`](crate::DeflateWriter)'s do), that `WriteError`.
    pub fn finish(mut self) -> Result<W, WriteError> {
        self.check_usable()?;
        let mut size = 0_u64;
        for record in &self.records {
            let header = header::central(record);
            self.out.write_all(&header)?;
            size = size.saturating_add(header.len() as u64);
        }
        let end = header::end(self.records.len() as u64, self.offset, size);
        self.out.write_all(&end)?;
        Ok(self.out)
    }

    /// `Err` once what was written is not a zip.
    fn check_usable(&self) -> Result<(), WriteError> {
        if self.failed {
            return Err(WriteError::Unusable);
        }
        Ok(())
    }

    /// `result`, with the writer unusable if it is an `Err`.
    fn track<T>(&mut self, result: Result<T, WriteError>) -> Result<T, WriteError> {
        self.failed |= result.is_err();
        result
    }

    /// Writes `record`'s local header.
    fn write_local(&mut self, record: &Record) -> Result<(), WriteError> {
        let header = header::local(record);
        let written = self.out.write_all(&header);
        self.track(written.map_err(WriteError::from))?;
        self.offset = self.offset.saturating_add(header.len() as u64);
        Ok(())
    }
}
