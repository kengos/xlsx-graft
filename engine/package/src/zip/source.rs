//! [`Source`]: the reader a zip is read from, with its length, so every read
//! is checked against the end of the file before it is made.

use std::io::{self, Read, Seek, SeekFrom};

/// The underlying reader and its length.
#[derive(Debug)]
pub(super) struct Source<R> {
    pub(super) reader: R,
    pub(super) len: u64,
}

impl<R: Read + Seek> Source<R> {
    /// Wraps `reader`, seeking to its end to learn its length.
    pub(super) fn new(mut reader: R) -> io::Result<Self> {
        let len = reader.seek(SeekFrom::End(0))?;
        Ok(Self { reader, len })
    }

    /// Whether `len` bytes from `offset` lie within the file.
    pub(super) fn contains(&self, offset: u64, len: u64) -> bool {
        offset.checked_add(len).is_some_and(|end| end <= self.len)
    }

    /// The `len` bytes from `offset`, or `None` when they run past the end.
    pub(super) fn read(&mut self, offset: u64, len: u64) -> io::Result<Option<Vec<u8>>> {
        let Some(size) = usize::try_from(len)
            .ok()
            .filter(|_| self.contains(offset, len))
        else {
            return Ok(None);
        };
        self.reader.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; size];
        self.reader.read_exact(&mut bytes)?;
        Ok(Some(bytes))
    }

    /// The last `len` bytes, or the whole file when it is shorter.
    pub(super) fn tail(&mut self, len: u64) -> io::Result<(u64, Vec<u8>)> {
        let len = self.len.min(len);
        let start = self.len - len;
        let bytes = self.read(start, len)?.unwrap_or_default();
        Ok((start, bytes))
    }
}
