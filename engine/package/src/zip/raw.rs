//! [`ZipArchive::copy_raw`]: an entry's stored bytes copied as they are,
//! while the same bytes are inflated to check them against the entry's
//! declared size and CRC-32.

use super::{EntryStream, ZipArchive};
use crate::PackageError;
use std::io::{Read, Seek, Write};
use xlsx_graft_diagnostics::Diagnostics;

/// The size of the buffer `copy_raw` inflates into to check the entry; the
/// inflated bytes are discarded.
const SCRATCH: usize = 32_768;

impl<R: Read + Seek> ZipArchive<R> {
    /// Writes the stored (compressed) bytes of the entry at `index`, all of
    /// [`data_range`](super::Entry::data_range), to `out` as they are, and
    /// inflates the same bytes to check the size and CRC-32 the entry
    /// declares, as [`EntryStream`] does. What the entry inflates to is not
    /// kept.
    ///
    /// `Ok(true)`: the bytes were copied and the entry is as declared.
    /// `Ok(false)`: there is no such entry or it is not readable (the
    /// diagnostic that says why is already pushed), or the entry was refused
    /// partway (`zip_inflate_failed` or `zip_crc_mismatch` is pushed, and
    /// the entry is not readable from then on). `out` may hold some of the
    /// bytes then, and the caller drops them.
    ///
    /// # Errors
    ///
    /// [`PackageError::Io`] when reading or seeking the archive, or writing
    /// `out`, fails. `out` may hold some of the stored bytes then.
    pub fn copy_raw(
        &mut self,
        index: usize,
        out: &mut dyn Write,
        diagnostics: &mut Diagnostics,
    ) -> Result<bool, PackageError> {
        let found = self.entries.get(index).filter(|entry| entry.readable);
        let Some(entry) = found.cloned() else {
            return Ok(false);
        };
        let mut stream = EntryStream::new(index, &entry);
        let mut scratch = vec![0; SCRATCH];
        loop {
            // The stored bytes go to `out` as they are read, so what the
            // copy holds in memory does not depend on the entry.
            let read = match stream.read_inner(self, &mut scratch, diagnostics, Some(&mut *out)) {
                Ok(read) => read,
                Err(_) if stream.is_stopped() => return Ok(false),
                Err(error) => return Err(error.into()),
            };
            if read == 0 {
                break;
            }
        }
        stream.copy_rest(&mut self.source.reader, &entry, out)?;
        Ok(true)
    }
}
