//! [`FileSpool`]: a spool in a file, and [`TempFiles`], which makes them in
//! unnamed temporary files.

use crate::spool::{Spool, SpoolFactory, past_end};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// A [`Spool`] in a file. It counts the bytes the file holds itself, so a
/// read past them is refused without asking the file, and it seeks before
/// every append and every read, so the two never disturb each other.
#[derive(Debug)]
pub struct FileSpool {
    pub(crate) file: File,
    /// Where the next append goes: the bytes the file holds.
    end: u64,
}

impl FileSpool {
    /// A spool in `file`, open for reading and writing: it appends after
    /// what the file holds already, which it can read back too. Not in
    /// append mode (`OpenOptions::append`): that writes at the file's end
    /// whatever the spool counts, and after a failed append the two part.
    ///
    /// # Errors
    ///
    /// When seeking to the file's end fails.
    pub fn new(mut file: File) -> io::Result<Self> {
        let end = file.seek(SeekFrom::End(0))?;
        Ok(Self { file, end })
    }
}

impl Spool for FileSpool {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(self.end))?;
        self.file.write_all(bytes)?;
        // No file reaches 2^64 bytes: the sum never saturates.
        self.end = self.end.saturating_add(bytes.len() as u64);
        Ok(())
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let stop = offset.checked_add(buf.len() as u64);
        if stop.is_none_or(|stop| stop > self.end) {
            return Err(past_end());
        }
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(buf)
    }
}

/// Makes [`FileSpool`]s in unnamed temporary files (the `tempfile` crate's
/// `tempfile_in`), in one directory: `std::env::temp_dir()`, as it is when
/// [`TempFiles::new`] is called, or one the caller names. The OS removes
/// each file however the process ends, so nothing is left to clean up.
#[derive(Debug, Clone)]
pub struct TempFiles {
    dir: PathBuf,
}

impl TempFiles {
    /// Temporary files in `std::env::temp_dir()`.
    #[must_use]
    pub fn new() -> Self {
        Self::in_dir(std::env::temp_dir())
    }

    /// Temporary files in `dir`, which must exist when a file is made.
    #[must_use]
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The directory the files are made in.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// A new, empty spool in a temporary file.
    ///
    /// # Errors
    ///
    /// When the file cannot be made: the directory is missing or not
    /// writable, or the disk is full.
    pub fn create_file(&self) -> io::Result<FileSpool> {
        let file = tempfile::tempfile_in(&self.dir)?;
        Ok(FileSpool { file, end: 0 })
    }
}

impl Default for TempFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl SpoolFactory for TempFiles {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Ok(Box::new(self.create_file()?))
    }
}
