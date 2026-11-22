//! The bytes of the zip's headers, built from plain values: the local file
//! header (APPNOTE 6.3.10 §4.3.7), the central directory header (§4.3.12)
//! and the end records (§4.3.14 to §4.3.16). The crate root says which
//! fields hold what.

/// What the headers say of one entry.
#[derive(Debug)]
pub(super) struct Record {
    pub(super) name: Vec<u8>,
    /// 0 (stored) or 8 (deflate), §4.4.5.
    pub(super) method: u16,
    pub(super) crc32: u32,
    pub(super) compressed_size: u64,
    pub(super) size: u64,
    /// Where the local header starts.
    pub(super) offset: u64,
}

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const END_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_SIGNATURE: u32 = 0x0606_4b50;
const LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
/// The zip64 extended information extra field's id (§4.5.3).
const ZIP64_EXTRA_ID: u16 = 0x0001;
/// Version made by: 4.5, MS-DOS (§4.4.2).
const VERSION_MADE_BY: u16 = 45;
const VERSION_DEFAULT: u16 = 20;
const VERSION_ZIP64: u16 = 45;
/// Bit 11 of the flags: the name is UTF-8 (§4.4.4).
const FLAG_UTF8: u16 = 0x0800;
/// 00:00:00 in the DOS time of §4.4.6.
const DOS_TIME: u16 = 0x0000;
/// 1980-01-01 in the DOS date of §4.4.6: (0 << 9) | (1 << 5) | 1.
const DOS_DATE: u16 = 0x0021;

/// The value a 32-bit field holds when the zip64 extra field holds the real
/// one (§4.4.8).
const SATURATED_32: u32 = u32::MAX;
/// The same for a 16-bit count (§4.4.22).
const SATURATED_16: u16 = u16::MAX;

/// Whether `value` does not fit a 32-bit field short of the saturated value.
fn wide(value: u64) -> bool {
    value >= u64::from(SATURATED_32)
}

/// A 32-bit field's value: `value`, or the saturated one when it is wide.
fn field32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(SATURATED_32)
}

/// The length of a name: at most 65,535 bytes, since `write_deflated`
/// refuses a longer name and a name read from a zip cannot be longer.
fn name_len(record: &Record) -> u16 {
    u16::try_from(record.name.len()).unwrap_or(u16::MAX)
}

/// Whether the entry needs zip64 in either of its headers.
fn needs_zip64(record: &Record) -> bool {
    wide(record.size) || wide(record.compressed_size) || wide(record.offset)
}

/// The version needed to extract, the same in both headers.
fn version_needed(record: &Record) -> u16 {
    if needs_zip64(record) {
        VERSION_ZIP64
    } else {
        VERSION_DEFAULT
    }
}

/// The general purpose flags: bit 11 for a name of UTF-8 with a non-ASCII
/// byte, which is what a reader needs the bit for.
fn flags(record: &Record) -> u16 {
    let unicode = std::str::from_utf8(&record.name).is_ok_and(|name| !name.is_ascii());
    if unicode { FLAG_UTF8 } else { 0 }
}

/// A zip64 extended information extra field holding `values`.
fn zip64_extra(values: &[u64]) -> Vec<u8> {
    if values.is_empty() {
        return Vec::new();
    }
    let len = u16::try_from(values.len().saturating_mul(8)).unwrap_or(u16::MAX);
    let mut out = Vec::new();
    out.extend(ZIP64_EXTRA_ID.to_le_bytes());
    out.extend(len.to_le_bytes());
    for value in values {
        out.extend(value.to_le_bytes());
    }
    out
}

/// The local file header of `record`, with its name and, when a size is
/// wide, the zip64 extra field with both sizes (§4.5.3).
pub(super) fn local(record: &Record) -> Vec<u8> {
    let zip64 = wide(record.size) || wide(record.compressed_size);
    let sizes = [record.size, record.compressed_size];
    let extra = zip64_extra(if zip64 { &sizes } else { &[] });
    let (compressed_size, size) = if zip64 {
        (SATURATED_32, SATURATED_32)
    } else {
        (field32(record.compressed_size), field32(record.size))
    };
    let mut out = Vec::new();
    out.extend(LOCAL_SIGNATURE.to_le_bytes());
    out.extend(version_needed(record).to_le_bytes());
    out.extend(flags(record).to_le_bytes());
    out.extend(record.method.to_le_bytes());
    out.extend(DOS_TIME.to_le_bytes());
    out.extend(DOS_DATE.to_le_bytes());
    out.extend(record.crc32.to_le_bytes());
    out.extend(compressed_size.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(name_len(record).to_le_bytes());
    out.extend(u16::try_from(extra.len()).unwrap_or(u16::MAX).to_le_bytes());
    out.extend(&record.name);
    out.extend(extra);
    out
}

/// The central directory header of `record`, with its name and a zip64
/// extra field holding, in the order of §4.5.3, those of the size, the
/// compressed size and the offset that are wide.
pub(super) fn central(record: &Record) -> Vec<u8> {
    let wide_values: Vec<u64> = [record.size, record.compressed_size, record.offset]
        .into_iter()
        .filter(|&value| wide(value))
        .collect();
    let extra = zip64_extra(&wide_values);
    let mut out = Vec::new();
    out.extend(CENTRAL_SIGNATURE.to_le_bytes());
    out.extend(VERSION_MADE_BY.to_le_bytes());
    out.extend(version_needed(record).to_le_bytes());
    out.extend(flags(record).to_le_bytes());
    out.extend(record.method.to_le_bytes());
    out.extend(DOS_TIME.to_le_bytes());
    out.extend(DOS_DATE.to_le_bytes());
    out.extend(record.crc32.to_le_bytes());
    out.extend(field32(record.compressed_size).to_le_bytes());
    out.extend(field32(record.size).to_le_bytes());
    out.extend(name_len(record).to_le_bytes());
    out.extend(u16::try_from(extra.len()).unwrap_or(u16::MAX).to_le_bytes());
    // Comment length, disk number, internal attributes, external attributes.
    out.extend([0; 10]);
    out.extend(field32(record.offset).to_le_bytes());
    out.extend(&record.name);
    out.extend(extra);
    out
}

/// The records that end the zip, for a central directory of `count`
/// headers that starts at `directory_offset` and is `directory_size` bytes:
/// the zip64 end record and locator when a count or a number does not fit
/// the end record (§4.4.22 to §4.4.24), then the end record, whose fields
/// that do not fit are saturated.
pub(super) fn end(count: u64, directory_offset: u64, directory_size: u64) -> Vec<u8> {
    let mut out = Vec::new();
    let zip64 = count >= u64::from(SATURATED_16) || wide(directory_offset) || wide(directory_size);
    if zip64 {
        // The zip64 record follows the central directory.
        let record_offset = directory_offset.saturating_add(directory_size);
        out.extend(ZIP64_SIGNATURE.to_le_bytes());
        // The size of the rest of the record (§4.3.14).
        out.extend(44_u64.to_le_bytes());
        out.extend(VERSION_MADE_BY.to_le_bytes());
        out.extend(VERSION_ZIP64.to_le_bytes());
        // This disk and the disk of the directory.
        out.extend([0; 8]);
        out.extend(count.to_le_bytes());
        out.extend(count.to_le_bytes());
        out.extend(directory_size.to_le_bytes());
        out.extend(directory_offset.to_le_bytes());
        out.extend(LOCATOR_SIGNATURE.to_le_bytes());
        // The disk of the zip64 record.
        out.extend(0_u32.to_le_bytes());
        out.extend(record_offset.to_le_bytes());
        // The number of disks.
        out.extend(1_u32.to_le_bytes());
    }
    let count = u16::try_from(count).unwrap_or(SATURATED_16);
    out.extend(END_SIGNATURE.to_le_bytes());
    // This disk and the disk of the directory.
    out.extend([0; 4]);
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(field32(directory_size).to_le_bytes());
    out.extend(field32(directory_offset).to_le_bytes());
    // The comment length.
    out.extend([0; 2]);
    out
}

#[cfg(test)]
mod tests;
