//! The write context of [`PackageEdit::finish`](super::PackageEdit::finish):
//! where to spill, the limits and the diagnostics, with the two ways it
//! writes an entry (a part rewritten, a part added).

use crate::deflate::DeflateWriter;
use crate::error::WriteError;
use crate::limits::WriteLimits;
use crate::zip::ZipWriter;
use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use xlsx_graft_diagnostics::{Diagnostics, Location};
use xlsx_graft_package::ZipArchive;
use xlsx_graft_spool::SpoolFactory;

/// What a write needs besides the zip: where to spill, the limits, and
/// where the diagnostics go.
pub(super) struct Context<'a> {
    pub(super) spools: &'a mut dyn SpoolFactory,
    pub(super) limits: &'a WriteLimits,
    pub(super) diagnostics: &'a mut Diagnostics,
    /// The name of the part of each zip entry, by the entry's index.
    pub(super) parts: BTreeMap<usize, String>,
}

impl Context<'_> {
    /// Writes the entry at `index` of `archive` as the next entry of `zip`,
    /// its new content made by `rewrite` from its inflated content. Returns
    /// whether the entry was written: `false` after a diagnostic is pushed
    /// (the content refused, or the entry found damaged).
    pub(super) fn rewritten<R: Read + Seek, W: Write>(
        &mut self,
        zip: &mut ZipWriter<W>,
        archive: &mut ZipArchive<R>,
        index: usize,
        rewrite: impl FnOnce(&mut dyn BufRead, &mut dyn Write) -> Result<(), WriteError>,
    ) -> Result<bool, WriteError> {
        let name = archive
            .entries()
            .get(index)
            .map_or(Vec::new(), |e| e.name().to_vec());
        // A diagnostic about a part is at the part. The content types stream
        // is not a part, so a diagnostic about the stream is at its zip entry,
        // as the package pushes its own diagnostics about the stream.
        let stream = Location::default().with_entry_bytes(&name);
        let location =
            (self.parts.get(&index)).map_or(stream, |part| Location::default().with_part(part));
        let Some(entry) = archive.open_entry(index, self.diagnostics)? else { return Ok(false) };
        let mut original = BufReader::new(entry);
        let mut deflate = DeflateWriter::new(&mut *self.spools, self.limits.part_spill_bytes);
        // Whatever `rewrite` leaves unread is read to the end, so the
        // entry's size and CRC-32 are checked in every case.
        let result = rewrite(&mut original, &mut deflate).and_then(|()| drain(&mut original));
        let stopped = original.get_ref().is_stopped();
        drop(original);
        if !accepted(self.diagnostics, result, stopped, location)? {
            return Ok(false);
        }
        zip.write_deflated(&name, deflate.finish()?)?;
        Ok(true)
    }

    /// Writes a new entry for the part `part` (its name with a `/` in front)
    /// as the next entry of `zip`, its content made by `write`. Returns
    /// whether it was written, as [`rewritten`](Self::rewritten) does.
    pub(super) fn added<W: Write>(
        &mut self,
        zip: &mut ZipWriter<W>,
        part: &str,
        write: impl FnOnce(&mut dyn Write) -> Result<(), WriteError>,
    ) -> Result<bool, WriteError> {
        let name = part.get(1..).unwrap_or_default().as_bytes();
        let mut deflate = DeflateWriter::new(&mut *self.spools, self.limits.part_spill_bytes);
        let result = write(&mut deflate);
        if !accepted(
            self.diagnostics,
            result,
            false,
            Location::default().with_part(part),
        )? {
            return Ok(false);
        }
        zip.write_deflated(name, deflate.finish()?)?;
        Ok(true)
    }
}

/// Reads `original` to its end and drops what it reads.
fn drain(original: &mut dyn Read) -> Result<(), WriteError> {
    io::copy(original, &mut io::sink())?;
    Ok(())
}

/// Whether the content of an entry is accepted: `result` is `Ok`.
/// `Ok(false)` when it is refused, with a diagnostic pushed already or now
/// at `location`: `result` is [`WriteError::Invalid`], or an error that comes
/// from the entry being read, which `stopped` says, with its own
/// diagnostic. Any other error is the host's, and is returned.
fn accepted(
    diagnostics: &mut Diagnostics,
    result: Result<(), WriteError>,
    stopped: bool,
    location: Location,
) -> Result<bool, WriteError> {
    match result {
        Ok(()) => Ok(true),
        Err(WriteError::Invalid(diagnostic)) => {
            diagnostics.push(diagnostic, location);
            Ok(false)
        }
        Err(_) if stopped => Ok(false),
        Err(error) => Err(error),
    }
}
