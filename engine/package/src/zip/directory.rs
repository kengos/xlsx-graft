//! Reading the central directory (APPNOTE 6.3.10 §4.3.12) into [`Entry`]s,
//! taking a size or offset the header leaves to the zip64 extra field
//! (§4.5.3) from there. Nothing here is checked against an entry's local
//! header yet.

use super::end::End;
use super::source::Source;
use super::{Entry, Method, SATURATED_16, SATURATED_32};
use crate::fields::{Fields, zip64_extra};
use std::io::{self, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, ZipDirectoryReason};

const HEADER_SIGNATURE: u32 = 0x0201_4b50;
/// The digital signature record that may close the directory (§4.3.13).
const DIGITAL_SIGNATURE: u32 = 0x0505_4b50;

/// Why one header cannot be read.
enum Refusal {
    /// The header is not there, or is cut short.
    Missing,
    /// The header leaves a field to a zip64 extra field that does not hold it.
    Zip64Field,
    /// The entry starts on another disk.
    MultiDisk,
}

/// Every entry the central directory that `end` locates lists, or the
/// diagnostic that says why it cannot be read.
pub(super) fn read<R: Read + Seek>(
    source: &mut Source<R>,
    end: &End,
) -> io::Result<Result<Vec<Entry>, Diagnostic>> {
    let directory = source.read(end.offset, end.size)?.unwrap_or_default();
    if end.entries > 0 && !directory.starts_with(&HEADER_SIGNATURE.to_le_bytes()) {
        return Ok(Err(misplaced(source, end)?));
    }
    // The end records follow the directory with nothing between them, so no
    // second directory can hide there for another reader to find.
    if end.offset.saturating_add(end.size) != end.records_offset {
        let reason = ZipDirectoryReason::ExtraBytes;
        return Ok(Err(Diagnostic::ZipDirectoryInvalid { reason }));
    }
    let mut fields = Fields::new(&directory);
    // Bounded by the entry limit, which `end` was checked against, and by
    // the directory's size, since a header takes at least 46 bytes: a limit
    // raised high does not let a lying count allocate.
    let capacity = usize::try_from(end.entries)
        .unwrap_or(0)
        .min(directory.len() / 46);
    let mut entries = Vec::with_capacity(capacity);
    for _ in 0..end.entries {
        match header(&mut fields) {
            Ok(entry) => entries.push(entry),
            Err(refusal) => return Ok(Err(refused(refusal))),
        }
    }
    if !fields.rest().is_empty() && !is_digital_signature(fields.rest()) {
        let reason = ZipDirectoryReason::ExtraBytes;
        return Ok(Err(Diagnostic::ZipDirectoryInvalid { reason }));
    }
    Ok(Ok(entries))
}

/// Why the directory does not start with a header where `end` says it does:
/// it does start with one where it would if the offsets did not count data
/// in front of the zip, or else it is missing.
fn misplaced<R: Read + Seek>(source: &mut Source<R>, end: &End) -> io::Result<Diagnostic> {
    let shifted = end.records_offset.saturating_sub(end.size);
    let at_shifted = source.read(shifted, 4)?;
    Ok(
        if at_shifted.as_deref() == Some(&HEADER_SIGNATURE.to_le_bytes()) {
            Diagnostic::ZipPrependedData {
                size: shifted.saturating_sub(end.offset),
            }
        } else {
            refused(Refusal::Missing)
        },
    )
}

/// The diagnostic for a header refused as `refusal` says.
fn refused(refusal: Refusal) -> Diagnostic {
    let reason = match refusal {
        Refusal::Missing => ZipDirectoryReason::MissingHeader,
        Refusal::Zip64Field => ZipDirectoryReason::Zip64FieldMissing,
        Refusal::MultiDisk => return Diagnostic::ZipMultiDisk {},
    };
    Diagnostic::ZipDirectoryInvalid { reason }
}

/// Whether `rest` is exactly one digital signature record.
fn is_digital_signature(rest: &[u8]) -> bool {
    let mut fields = Fields::new(rest);
    fields.u32() == Some(DIGITAL_SIGNATURE)
        && fields.u16().map(usize::from) == Some(fields.rest().len())
}

/// The entry whose header `fields` starts at, leaving `fields` past it.
fn header(fields: &mut Fields<'_>) -> Result<Entry, Refusal> {
    let mut fixed = Fields::new(fields.take(46).ok_or(Refusal::Missing)?);
    let missing = |field: Option<u32>| field.ok_or(Refusal::Missing);
    if missing(fixed.u32())? != HEADER_SIGNATURE {
        return Err(Refusal::Missing);
    }
    let _versions = fixed.take(4);
    let flags = fixed.u16().ok_or(Refusal::Missing)?;
    let method = fixed.u16().ok_or(Refusal::Missing)?;
    let _modified = fixed.take(4);
    let crc32 = missing(fixed.u32())?;
    let compressed_size = missing(fixed.u32())?;
    let size = missing(fixed.u32())?;
    let name_len = fixed.u16().ok_or(Refusal::Missing)?;
    let extra_len = fixed.u16().ok_or(Refusal::Missing)?;
    let comment_len = fixed.u16().ok_or(Refusal::Missing)?;
    let disk = fixed.u16().ok_or(Refusal::Missing)?;
    let _attributes = fixed.take(6);
    let local_offset = missing(fixed.u32())?;
    let name = fields.take(name_len.into()).ok_or(Refusal::Missing)?;
    let extra = fields.take(extra_len.into()).ok_or(Refusal::Missing)?;
    fields.take(comment_len.into()).ok_or(Refusal::Missing)?;

    let mut zip64 = Fields::new(zip64_extra(extra).unwrap_or_default());
    let mut wide = |value: u32| match value {
        SATURATED_32 => zip64.u64().ok_or(Refusal::Zip64Field),
        value => Ok(u64::from(value)),
    };
    let size = wide(size)?;
    let compressed_size = wide(compressed_size)?;
    let local_offset = wide(local_offset)?;
    let disk = match disk {
        SATURATED_16 => zip64.u32().ok_or(Refusal::Zip64Field)?,
        disk => u32::from(disk),
    };
    if disk != 0 {
        return Err(Refusal::MultiDisk);
    }
    Ok(Entry {
        name: name.to_vec(),
        flags,
        method: Method::from_raw(method),
        crc32,
        compressed_size,
        size,
        local_offset,
        data_offset: 0,
        end_offset: 0,
        readable: true,
    })
}
