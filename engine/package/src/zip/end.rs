//! Finding the end records: the end of central directory record (APPNOTE
//! 6.3.10 §4.3.16) and, in a zip64 archive, the zip64 end of central
//! directory locator and record (§4.3.15, §4.3.14). They say where the
//! central directory is, how large it is and how many entries it lists,
//! which are checked against the limits before a byte of it is read.

use super::source::Source;
use crate::ZipLimits;
use crate::fields::Fields;
use std::cmp::Ordering;
use std::io::{self, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, ZipDirectoryReason};

const END_SIGNATURE: u32 = 0x0605_4b50;
const END_LEN: u64 = 22;
/// The longest comment the end record can carry, so the end record starts
/// within the last `END_LEN + MAX_COMMENT` bytes.
const MAX_COMMENT: u64 = 65_535;
const LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const LOCATOR_LEN: u64 = 20;
const ZIP64_SIGNATURE: u32 = 0x0606_4b50;
/// The zip64 record's fixed fields; the extensible data after them is not
/// read.
const ZIP64_LEN: u64 = 56;
/// The first bytes of an OLE compound file.
const OLE_MAGIC: &[u8] = &[0xd0, 0xcf, 0x11, 0xe0];
/// The first bytes of a zip that starts with a local header.
const LOCAL_MAGIC: &[u8] = b"PK\x03\x04";

/// Where the central directory is, as the end records give it.
pub(super) struct End {
    /// How many entries it lists.
    pub(super) entries: u64,
    /// Where it starts.
    pub(super) offset: u64,
    /// How many bytes it takes.
    pub(super) size: u64,
    /// Where the end records after it start: the zip64 record, or else the
    /// end record.
    pub(super) records_offset: u64,
    /// What the record says, field by field: this disk, the directory's
    /// disk, the entries on this disk, the entries, the size and the offset.
    written: [u64; 6],
}

/// The value of each of [`End::written`]'s fields in an end record that
/// leaves it to the zip64 record (APPNOTE 6.3.10 §4.4.1.4).
const SATURATED: [u64; 6] = [0xffff, 0xffff, 0xffff, 0xffff, 0xffff_ffff, 0xffff_ffff];

impl End {
    /// The end records at `records_offset` that wrote `written`.
    fn new(written: [u64; 6], records_offset: u64) -> Self {
        let [_, _, _, entries, size, offset] = written;
        Self {
            entries,
            offset,
            size,
            records_offset,
            written,
        }
    }

    /// Whether the records say the zip spans more than one disk.
    fn multi_disk(&self) -> bool {
        let [disk, directory_disk, entries_on_disk, entries, _, _] = self.written;
        disk != 0 || directory_disk != 0 || entries_on_disk != entries
    }

    /// Whether every field of this end record either leaves its value to
    /// `zip64` or holds the same one, so no reader that reads the zip64
    /// record only when it must sees another zip.
    fn agrees_with(&self, zip64: &End) -> bool {
        let fields = self.written.iter().zip(SATURATED).zip(zip64.written);
        fields
            .into_iter()
            .all(|((&own, saturated), other)| own == saturated || own == other)
    }
}

/// The end records of the zip in `source`, checked against `limits`, or
/// the diagnostic that says why its central directory cannot be read.
pub(super) fn find<R: Read + Seek>(
    source: &mut Source<R>,
    limits: &ZipLimits,
) -> io::Result<Result<End, Diagnostic>> {
    let head = source.read(0, 4)?;
    if head.as_deref() == Some(OLE_MAGIC) {
        return Ok(Err(Diagnostic::OleCompoundFile {}));
    }
    // The last end record signature, as most readers take it; its comment
    // must then run exactly to the end of the file, as the others require.
    let (tail_start, tail) = source.tail(END_LEN + MAX_COMMENT)?;
    let signature = END_SIGNATURE.to_le_bytes();
    let Some(at) = tail.windows(4).rposition(|window| window == signature) else {
        let starts_as_zip = head.as_deref() == Some(LOCAL_MAGIC);
        return Ok(Err(if starts_as_zip {
            Diagnostic::ZipTruncated {}
        } else {
            Diagnostic::NotAZip {}
        }));
    };
    let record = tail.get(at..).unwrap_or_default();
    let end = match end_record(record, tail_start + at as u64) {
        Ok(end) => end,
        Err(diagnostic) => return Ok(Err(diagnostic)),
    };
    let locator_offset = end.records_offset.checked_sub(LOCATOR_LEN);
    let locator = match locator_offset {
        Some(at) => source.read(at, LOCATOR_LEN)?,
        None => None,
    };
    let end = match locator.as_deref().and_then(zip64_locator) {
        None => end,
        Some((_, true)) => return Ok(Err(Diagnostic::ZipMultiDisk {})),
        Some((at, false)) => match source
            .read(at, ZIP64_LEN)?
            .as_deref()
            .and_then(|record| zip64_record(record, at, locator_offset))
        {
            None => return Ok(Err(invalid(ZipDirectoryReason::Zip64RecordMissing))),
            Some(zip64) if zip64.multi_disk() => {
                return Ok(Err(Diagnostic::ZipMultiDisk {}));
            }
            Some(zip64) if !end.agrees_with(&zip64) => {
                return Ok(Err(invalid(ZipDirectoryReason::EndRecordsDisagree)));
            }
            Some(zip64) => zip64,
        },
    };
    Ok(check(end, limits))
}

/// `zip_directory_invalid` for `reason`.
fn invalid(reason: ZipDirectoryReason) -> Diagnostic {
    Diagnostic::ZipDirectoryInvalid { reason }
}

/// `end`, unless it spans disks, breaks a limit or runs past its records.
fn check(end: End, limits: &ZipLimits) -> Result<End, Diagnostic> {
    if end.multi_disk() {
        return Err(Diagnostic::ZipMultiDisk {});
    }
    let limit = limits.max_entries;
    if end.entries > limit.value {
        return Err(Diagnostic::ZipEntryCountExceeded {
            count: end.entries,
            limit,
        });
    }
    let limit = limits.max_directory_bytes;
    if end.size > limit.value {
        return Err(Diagnostic::ZipDirectoryTooLarge {
            size: end.size,
            limit,
        });
    }
    if end
        .offset
        .checked_add(end.size)
        .is_none_or(|stop| stop > end.records_offset)
    {
        return Err(Diagnostic::ZipTruncated {});
    }
    Ok(end)
}

/// The end record at the start of `record`, which sits at `offset` in the
/// file and runs to its end: it is cut short when its comment runs past the
/// end, and followed by other bytes when the comment stops before it.
fn end_record(record: &[u8], offset: u64) -> Result<End, Diagnostic> {
    let Some((written, comment)) = end_fields(record) else {
        return Err(Diagnostic::ZipTruncated {});
    };
    let rest = record.len().saturating_sub(END_LEN as usize);
    match comment.cmp(&rest) {
        Ordering::Equal => Ok(End::new(written, offset)),
        Ordering::Greater => Err(Diagnostic::ZipTruncated {}),
        Ordering::Less => Err(invalid(ZipDirectoryReason::TrailingBytes)),
    }
}

/// The fields of the end record at the start of `record`, and its comment's
/// length.
fn end_fields(record: &[u8]) -> Option<([u64; 6], usize)> {
    let mut fields = Fields::new(record);
    fields.take(4)?;
    let disk = fields.u16()?;
    let directory_disk = fields.u16()?;
    let entries_on_disk = fields.u16()?;
    let entries = fields.u16()?;
    let size = fields.u32()?;
    let directory_offset = fields.u32()?;
    let comment = fields.u16()?;
    let written = [
        disk.into(),
        directory_disk.into(),
        entries_on_disk.into(),
        entries.into(),
        size.into(),
        directory_offset.into(),
    ];
    Some((written, comment.into()))
}

/// Where the zip64 locator in `locator` says the zip64 record is, and
/// whether it says the zip spans more than one disk; `None` when `locator`
/// is not a locator, so the zip is not zip64.
fn zip64_locator(locator: &[u8]) -> Option<(u64, bool)> {
    let mut fields = Fields::new(locator);
    if fields.u32()? != LOCATOR_SIGNATURE {
        return None;
    }
    let record_disk = fields.u32()?;
    let offset = fields.u64()?;
    let disks = fields.u32()?;
    Some((offset, record_disk != 0 || disks > 1))
}

/// The zip64 record in `record`, which sits at `offset` in the file, if it
/// is one that ends where the locator, at `locator_offset`, starts.
fn zip64_record(record: &[u8], offset: u64, locator_offset: Option<u64>) -> Option<End> {
    let mut fields = Fields::new(record);
    if fields.u32()? != ZIP64_SIGNATURE {
        return None;
    }
    // The record's size, not counting its signature and this field (§4.3.14.1).
    let size_of_record = fields.u64()?;
    let ends = offset.checked_add(12)?.checked_add(size_of_record);
    if size_of_record < ZIP64_LEN - 12 || ends != locator_offset {
        return None;
    }
    fields.take(4)?;
    let disk = fields.u32()?;
    let directory_disk = fields.u32()?;
    let entries_on_disk = fields.u64()?;
    let entries = fields.u64()?;
    let size = fields.u64()?;
    let directory_offset = fields.u64()?;
    let written = [
        disk.into(),
        directory_disk.into(),
        entries_on_disk,
        entries,
        size,
        directory_offset,
    ];
    Some(End::new(written, offset))
}
