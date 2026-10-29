//! [`EntryReader`]: an [`EntryStream`] lent its archive and the diagnostics
//! for as long as it lives, so it reads as any [`Read`] does.

use super::{EntryStream, ZipArchive};
use std::io::{self, Read, Seek};
use xlsx_graft_diagnostics::Diagnostics;

/// The inflated data of one readable entry, from
/// [`ZipArchive::open_entry`]: an [`EntryStream`] that holds the archive
/// and the diagnostics while it is read, and reads as that stream does.
pub struct EntryReader<'a, R> {
    archive: &'a mut ZipArchive<R>,
    diagnostics: &'a mut Diagnostics,
    stream: EntryStream,
}

impl<'a, R> EntryReader<'a, R> {
    /// `stream`, read from `archive`, pushing into `diagnostics`.
    pub(super) fn new(
        archive: &'a mut ZipArchive<R>,
        diagnostics: &'a mut Diagnostics,
        stream: EntryStream,
    ) -> Self {
        Self {
            archive,
            diagnostics,
            stream,
        }
    }

    /// Whether the reader stopped because the entry is refused, so an error
    /// it returned stands for a problem in the entry, which a diagnostic
    /// says, rather than a failure of the host.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.stream.is_stopped()
    }
}

impl<R: Read + Seek> Read for EntryReader<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.stream.read(self.archive, out, self.diagnostics)
    }
}
