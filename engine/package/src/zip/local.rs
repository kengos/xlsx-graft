//! One entry's local record: its local header (APPNOTE 6.3.10 §4.3.7), name,
//! extra field, data and, when bit 3 of its flags is set, data descriptor
//! (§4.3.9). Where the record ends comes from the local header's lengths,
//! the central directory's compressed size and, for the descriptor, where
//! the next record starts; then the values written twice are compared with
//! the central directory's (ECMA-376-2:2021 Annex B.2).

use super::source::Source;
use super::{Entry, Method, SATURATED_32};
use crate::fields::{Fields, zip64_extra};
use std::io::{self, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, ZipHeaderField};

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const LOCAL_LEN: u64 = 30;
/// The signature a data descriptor may start with (§4.3.9.3).
const DESCRIPTOR_SIGNATURE: u32 = 0x0807_4b50;
/// Bit 3 of the general purpose flags: the local header leaves the CRC-32
/// and sizes as zero, and a data descriptor after the data holds them.
const HAS_DESCRIPTOR: u16 = 0x0008;

/// A local header's fixed fields.
struct Local {
    flags: u16,
    method: u16,
    crc32: u32,
    compressed_size: u32,
    size: u32,
    name_len: u16,
    extra_len: u16,
}

/// What locating one record found.
pub(super) struct Located {
    /// Whether the record's end, `Entry::end_offset`, is known; else it is
    /// as far as the record was made out.
    pub(super) complete: bool,
    /// Why the entry cannot be trusted, if it cannot.
    pub(super) problem: Option<Diagnostic>,
}

/// Finds where `entry`'s data starts and where its record ends, then, for a
/// readable entry, compares the record with the central directory. `starts`
/// are where every record starts, in order, and `directory` is where the
/// central directory starts: the next of them after the data is where a data
/// descriptor ends.
pub(super) fn locate<R: Read + Seek>(
    source: &mut Source<R>,
    entry: &mut Entry,
    starts: &[u64],
    directory: u64,
) -> io::Result<Located> {
    let partial = |problem| Located {
        complete: false,
        problem: Some(problem),
    };
    // The record starts before the central directory, which takes at least
    // one header, so all 30 bytes of the local header lie within the file;
    // bytes that do not read as one are a signature mismatch.
    let header = source
        .read(entry.local_offset, LOCAL_LEN)?
        .unwrap_or_default();
    let variable_offset = entry.local_offset.saturating_add(LOCAL_LEN);
    entry.end_offset = variable_offset;
    let Some(local) = local_header(&header) else {
        let field = ZipHeaderField::Signature;
        return Ok(partial(Diagnostic::ZipHeaderMismatch { field }));
    };
    let data_offset = variable_offset
        .saturating_add(local.name_len.into())
        .saturating_add(local.extra_len.into());
    entry.end_offset = data_offset;
    if !source.contains(data_offset, entry.compressed_size) {
        return Ok(partial(Diagnostic::ZipTruncated {}));
    }
    let data_end = data_offset.saturating_add(entry.compressed_size);
    entry.data_offset = data_offset;
    entry.end_offset = data_end;
    if data_end > directory {
        return Ok(partial(Diagnostic::ZipEntryOverlapsDirectory {}));
    }
    if local.flags & HAS_DESCRIPTOR != 0 {
        let next = starts.partition_point(|&start| start < data_end);
        let next = starts
            .get(next)
            .map_or(directory, |&start| start.min(directory));
        entry.end_offset = next;
    }
    let problem = if entry.readable {
        compare(source, entry, &local, variable_offset, data_end)?.err()
    } else {
        None
    };
    Ok(Located {
        complete: true,
        problem,
    })
}

/// Compares the record of `entry`, whose local header is `local`, with the
/// central directory: the name and extra field at `variable_offset`, the
/// fixed fields, and the data descriptor from `data_end` to the record's end.
fn compare<R: Read + Seek>(
    source: &mut Source<R>,
    entry: &Entry,
    local: &Local,
    variable_offset: u64,
    data_end: u64,
) -> io::Result<Result<(), Diagnostic>> {
    let mismatch = |field| Ok(Err(Diagnostic::ZipHeaderMismatch { field }));
    // Compared before the name is read, so a long name costs nothing.
    if usize::from(local.name_len) != entry.name.len() {
        return mismatch(ZipHeaderField::Name);
    }
    let variable_len = u64::from(local.name_len).saturating_add(local.extra_len.into());
    let variable = source
        .read(variable_offset, variable_len)?
        .unwrap_or_default();
    let (name, extra) = variable
        .split_at_checked(usize::from(local.name_len))
        .unwrap_or_default();
    if name != entry.name {
        return mismatch(ZipHeaderField::Name);
    }
    if local.flags != entry.flags {
        return mismatch(ZipHeaderField::Flags);
    }
    if Method::from_raw(local.method) != entry.method {
        return mismatch(ZipHeaderField::Method);
    }
    let has_descriptor = local.flags & HAS_DESCRIPTOR != 0;
    let agrees = |local: Option<u64>, central: u64| {
        local == Some(central) || (has_descriptor && local == Some(0))
    };
    // A local header that leaves either size to the zip64 extra field holds
    // both there (§4.5.3).
    let saturated = local.size == SATURATED_32 || local.compressed_size == SATURATED_32;
    let mut zip64 = Fields::new(zip64_extra(extra).unwrap_or_default());
    let mut wide = |value: u32| {
        if saturated {
            zip64.u64()
        } else {
            Some(u64::from(value))
        }
    };
    let size = wide(local.size);
    let compressed_size = wide(local.compressed_size);
    if !agrees(Some(local.crc32.into()), entry.crc32.into()) {
        return mismatch(ZipHeaderField::Crc32);
    }
    if !agrees(compressed_size, entry.compressed_size) {
        return mismatch(ZipHeaderField::CompressedSize);
    }
    if !agrees(size, entry.size) {
        return mismatch(ZipHeaderField::UncompressedSize);
    }
    if !has_descriptor {
        return Ok(Ok(()));
    }
    // Checked before it is read: the bytes up to the next record can be as
    // many as the file holds.
    let len = entry.end_offset.saturating_sub(data_end);
    if !matches!(len, 12 | 16 | 20 | 24) {
        return mismatch(ZipHeaderField::Descriptor);
    }
    let bytes = source.read(data_end, len)?.unwrap_or_default();
    let Some((crc32, compressed_size, size)) = descriptor(&bytes) else {
        return mismatch(ZipHeaderField::Descriptor);
    };
    if crc32 != entry.crc32 {
        return mismatch(ZipHeaderField::Crc32);
    }
    if compressed_size != entry.compressed_size {
        return mismatch(ZipHeaderField::CompressedSize);
    }
    if size != entry.size {
        return mismatch(ZipHeaderField::UncompressedSize);
    }
    Ok(Ok(()))
}

/// The fixed fields of the local header in `header`, or `None` when it does
/// not start with the local header signature.
fn local_header(header: &[u8]) -> Option<Local> {
    let mut fields = Fields::new(header);
    if fields.u32()? != LOCAL_SIGNATURE {
        return None;
    }
    fields.take(2)?;
    let flags = fields.u16()?;
    let method = fields.u16()?;
    fields.take(4)?;
    Some(Local {
        flags,
        method,
        crc32: fields.u32()?,
        compressed_size: fields.u32()?,
        size: fields.u32()?,
        name_len: fields.u16()?,
        extra_len: fields.u16()?,
    })
}

/// The CRC-32, compressed size and size in `bytes`, the whole data
/// descriptor, 12, 16, 20 or 24 bytes long. Its length tells its shape,
/// since a record ends where the next starts: 12 or 16 bytes with 4-byte
/// sizes, 20 or 24 with 8-byte sizes, the longer of each starting with the
/// signature.
fn descriptor(bytes: &[u8]) -> Option<(u32, u64, u64)> {
    let mut fields = Fields::new(bytes);
    if matches!(bytes.len(), 16 | 24) && fields.u32()? != DESCRIPTOR_SIGNATURE {
        return None;
    }
    let crc32 = fields.u32()?;
    if bytes.len() < 20 {
        Some((crc32, fields.u32()?.into(), fields.u32()?.into()))
    } else {
        Some((crc32, fields.u64()?, fields.u64()?))
    }
}
