//! xlsx-graft package: an untrusted xlsx's zip container, read from its
//! central directory under limits before any entry is inflated.
//!
//! This crate holds the zip layer only. Part names, the content types stream
//! and the relationships graph are built on it and are not here yet.
//!
//! # Reading a zip
//!
//! [`ZipArchive::open`] takes a `Read + Seek`, the [`ZipLimits`] in force
//! and the [`Diagnostics`](xlsx_graft_diagnostics::Diagnostics) to push every
//! problem into. It returns `Ok(None)` when the central directory cannot be
//! read (the diagnostics say why), and `Err` only for a failure of the host
//! ([`PackageError`]).
//!
//! The rule behind most of what it refuses: a zip is read only when every
//! reader would read the same zip from it. So the file must be laid out
//! exactly, every byte accounted for once, in this order: the local records
//! the entries point to, the central directory, the zip64 record and
//! locator if any, the end record and its comment to the end of the file;
//! and a value written twice (in a local header and the central directory,
//! in the end record and the zip64 record) must be the same both times.
//! What it refuses:
//!
//! - **Not a zip**: a file that starts as an OLE compound file (an encrypted
//!   workbook or a legacy `.xls`) is `ole_compound_file` alone; one with no
//!   end of central directory record is `not_a_zip`, or `zip_truncated` when
//!   it starts with a local header.
//! - **The end records** (APPNOTE 6.3.10 §4.3.16, and §4.3.14–15 for zip64):
//!   the end record is the last signature in the final 65,557 bytes, as most
//!   readers take it, and its comment must run exactly to the end of the
//!   file, as the others require: bytes after it are `zip_directory_invalid`
//!   (`trailing_bytes`), a comment cut short `zip_truncated`. A zip64 record
//!   must end where its locator starts, and each field of the end record
//!   must either leave its value to the zip64 record or give the same one.
//!   A zip on more than one disk is `zip_multi_disk` (ECMA-376-2:2021 Annex
//!   B.4). The entry count and the directory's size are held to their
//!   limits BEFORE the directory is read or anything is allocated for it.
//! - **The central directory** (§4.3.12): fewer or more headers than the end
//!   record lists, bytes between the directory and the end records, or a
//!   size left to a zip64 extra field that does not hold it, is
//!   `zip_directory_invalid`. Offsets that do not count data in front of the
//!   zip are `zip_prepended_data` (in a zip64 archive, whose locator then
//!   points at no record, `zip_directory_invalid`), and are not corrected
//!   for: the reader never guesses where a zip starts.
//!
//! Those stop the reading. From here on, a problem refuses one entry at a
//! time: the entry stays in [`ZipArchive::entries`], not readable, refused
//! by one diagnostic, the first found in this order:
//!
//! 1. its name is longer than the limit (`entry_name_too_long`), it is
//!    encrypted (`zip_encrypted_entry`) or its method is neither stored nor
//!    deflate (`zip_unsupported_compression`), the only two a package may use
//!    (ECMA-376-2:2021 §7.3.6);
//! 2. taking every entry's local record in the order they start in the file:
//!    it starts before an earlier one ends (`zip_entries_overlap`; the
//!    earlier one keeps the bytes, and of two that start at the same byte,
//!    the one the central directory lists first) or in the central
//!    directory, or its data runs into it (`zip_entry_overlaps_directory`);
//!    it runs past the end of the file (`zip_truncated`); or its local
//!    header or data descriptor disagrees with the central directory
//!    (`zip_header_mismatch`, naming the field; Annex B.2). A data
//!    descriptor runs to where the next record starts, and its length (12,
//!    16, 20 or 24 bytes) tells whether it has a signature and 8-byte sizes.
//!    Overlap is checked before a local header is read, so however many
//!    entries point at the same bytes, the local headers make the reader
//!    read no more than the file;
//! 3. it declares more than one entry may inflate to (`zip_entry_too_large`)
//!    or more than the ratio limit times its compressed size
//!    (`zip_ratio_exceeded`). Last, the entry whose declared size takes the
//!    total past its limit is `zip_total_too_large`, and every later entry
//!    is left unread without a diagnostic of its own.
//!
//! Bytes no local record accounts for are reported, and the entries still
//! read: `zip_prepended_data` in front of the first record, and
//! `zip_unlisted_data` between records or before the central directory,
//! where a reader that walks the local headers would find an entry the
//! central directory does not list. These are not refusals: a gap is
//! reported at the entry after it, which may also be refused. A refused
//! entry's record still takes its bytes; after a record whose end is not
//! known, no gap is reported.
//! Diagnostics come in the order they are found: the archive's own, then
//! each pass above, the second in file order and the others in central
//! directory order. Entries with the same name are all listed: telling part
//! names apart is the layer above's.
//!
//! # Reading an entry
//!
//! [`ZipArchive::open_entry`] hands out an [`EntryReader`] (a [`std::io::Read`])
//! for a readable entry. It yields at most the size the entry declares,
//! which the limits were checked on, so it cannot be made to inflate more
//! than they allow. Data that inflates to more or less than declared, that
//! is not valid deflate, or whose CRC-32 differs, pushes `zip_inflate_failed`
//! or `zip_crc_mismatch` against the entry; the read fails, and
//! [`EntryReader::is_stopped`] tells that failure from one of the host.
//!
//! ```
//! use std::io::{Cursor, Read};
//! use xlsx_graft_diagnostics::Diagnostics;
//! use xlsx_graft_package::{ZipArchive, ZipLimits};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // An empty zip: its end of central directory record alone.
//! let mut zip = b"PK\x05\x06".to_vec();
//! zip.extend([0; 18]);
//! let mut diagnostics = Diagnostics::new();
//! let archive = ZipArchive::open(Cursor::new(zip), &ZipLimits::default(), &mut diagnostics)?;
//! let Some(mut archive) = archive else {
//!     return Ok(()); // the diagnostics say why
//! };
//! if let Some(index) = archive.position(b"[Content_Types].xml") {
//!     if let Some(mut entry) = archive.open_entry(index, &mut diagnostics)? {
//!         // Hold an XML part to its own limit: read one byte past it.
//!         let mut bytes = Vec::new();
//!         match entry.by_ref().take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes) {
//!             Ok(_) => {}
//!             Err(_) if entry.is_stopped() => {} // the diagnostics say why
//!             Err(error) => return Err(error.into()),
//!         }
//!     }
//! }
//! assert!(diagnostics.records().is_empty());
//! # Ok(())
//! # }
//! ```

mod error;
mod fields;
mod limits;
mod zip;

pub use error::PackageError;
pub use limits::ZipLimits;
pub use zip::{Entry, EntryReader, Method, ZipArchive};

#[cfg(test)]
mod tests;
