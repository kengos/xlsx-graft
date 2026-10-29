//! How a caller reads a part of a [`Package`]: through a reader that holds
//! the package ([`Package::open_part`]), whole as XML
//! ([`Package::read_xml_part`]), or through a stream that holds nothing of
//! it ([`Package::stream_part`]), handed the package at each read
//! ([`Package::read_stream`]).

use super::{Package, xml_part};
use crate::{EntryReader, EntryStream, PackageError};
use std::io::{self, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostics, Limit, Location};

impl<R: Read + Seek> Package<R> {
    /// A reader of the part at `index` of [`parts`](Self::parts), as
    /// [`ZipArchive::open_entry`](crate::ZipArchive::open_entry) gives one.
    ///
    /// # Errors
    ///
    /// [`PackageError::Io`] when seeking to the part's data fails.
    pub fn open_part<'a>(
        &'a mut self,
        index: usize,
        diagnostics: &'a mut Diagnostics,
    ) -> Result<Option<EntryReader<'a, R>>, PackageError> {
        let entry = self.parts.get(index).map_or(usize::MAX, |part| part.entry);
        self.archive.open_entry(entry, diagnostics)
    }

    /// The bytes of the part at `index` of [`parts`](Self::parts), read
    /// whole as the package's own XML streams are: `None` when there is no
    /// such part, it is not readable (its diagnostic is already pushed), it
    /// declares more than `limit` bytes (`xml_too_large`, pushed against
    /// the part before a byte is inflated), or its data turns out damaged
    /// (pushed by the zip reader).
    ///
    /// # Errors
    ///
    /// [`PackageError::Io`] when reading or seeking the package fails.
    pub fn read_xml_part(
        &mut self,
        index: usize,
        limit: Limit,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Vec<u8>>, PackageError> {
        let part = self.parts.get(index);
        let entry = part.map_or(usize::MAX, |part| part.entry);
        let location = (part.map(|part| Location::default().with_part(part.name.as_str())))
            .unwrap_or_default();
        xml_part::read(&mut self.archive, entry, limit, &location, diagnostics)
    }

    /// A stream of the part at `index` of [`parts`](Self::parts), as
    /// [`ZipArchive::stream_entry`](crate::ZipArchive::stream_entry) gives
    /// one, read through [`read_stream`](Self::read_stream). It holds no
    /// borrow of the package, so a caller may keep it beside the package.
    #[must_use]
    pub fn stream_part(&self, index: usize) -> Option<EntryStream> {
        let entry = self.parts.get(index).map_or(usize::MAX, |part| part.entry);
        self.archive.stream_entry(entry)
    }

    /// Reads the next bytes of `stream`, a stream of one of this package's
    /// parts, into `out`, as [`EntryStream::read`] reads them from the
    /// package's archive.
    ///
    /// # Errors
    ///
    /// As [`EntryStream::read`].
    pub fn read_stream(
        &mut self,
        stream: &mut EntryStream,
        out: &mut [u8],
        diagnostics: &mut Diagnostics,
    ) -> io::Result<usize> {
        stream.read(&mut self.archive, out, diagnostics)
    }
}
