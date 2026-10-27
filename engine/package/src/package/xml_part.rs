//! [`read`]: the bytes of an XML stream's zip entry, held to the XML size
//! limit before a byte of it is inflated.

use crate::{PackageError, ZipArchive};
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit, Location};

/// The inflated bytes of the entry at `index`, or `None` when they are not
/// read: the entry is not readable (its diagnostic is already pushed), it
/// declares more than `limit` bytes (`xml_too_large`, pushed at `location`),
/// or its data turns out damaged as it inflates (pushed by the zip reader).
///
/// The declared size is what the entry reader yields at most, so holding it
/// to the limit holds the bytes read to it. The reader is read to its end,
/// never cut at the limit, so it checks the CRC-32 and the declared size.
pub(super) fn read<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
    limit: Limit,
    location: &Location,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Vec<u8>>, PackageError> {
    let size = (archive.entries().get(index))
        .filter(|entry| entry.is_readable())
        .map(|entry| entry.size());
    if let Some(size) = size
        && size > limit.value
    {
        diagnostics.push(Diagnostic::XmlTooLarge { size, limit }, location.clone());
        return Ok(None);
    }
    let Some(mut reader) = archive.open_entry(index, diagnostics)? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    match reader.read_to_end(&mut bytes) {
        Ok(_) => Ok(Some(bytes)),
        Err(_) if reader.is_stopped() => Ok(None),
        Err(error) => Err(error.into()),
    }
}
