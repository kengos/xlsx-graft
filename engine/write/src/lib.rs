//! xlsx-graft write: the zip container of the xlsx the engine writes,
//! written entry by entry. An entry is copied from a zip read
//! by `xlsx-graft-package` without being inflated and deflated again
//! ([`ZipWriter::copy_entry`]), or is new content deflated here
//! ([`DeflateWriter`], then [`ZipWriter::write_deflated`]), held in memory
//! until it passes a size the caller sets and then in scratch storage
//! (`xlsx-graft-spool`).
//!
//! # The zip it writes
//!
//! The same input gives the same bytes, and nothing in the zip comes from
//! the machine or the clock:
//!
//! - **Order**: the entries are in the order they were written, in the
//!   local records and in the central directory.
//! - **Time**: every entry has the modification time 1980-01-01 00:00:00,
//!   the earliest a zip can state.
//! - **Fields**: no entry comment, no archive comment, no extra field (the
//!   zip64 one apart, below), internal and external attributes 0, version
//!   made by 45 (MS-DOS), version needed 20 (45 for an entry that needs
//!   zip64), and no data descriptors: each local header holds the CRC-32
//!   and the sizes.
//! - **Names**: the bytes it was given. Bit 11 of the flags (UTF-8) is set
//!   only for a name that is valid UTF-8 with a non-ASCII byte.
//! - **zip64**: only where a size or offset is 4,294,967,295 or more, or
//!   the archive has 65,535 entries or more (APPNOTE 6.3.10 §4.5.3,
//!   §4.4.1.4); never otherwise.
//! - **Content**: an entry copied keeps its method and its stored bytes as
//!   they are. New content is raw deflate at level 6 (miniz_oxide).
//!
//! ```
//! use xlsx_graft_spool::MemorySpools;
//! use xlsx_graft_write::{DeflateWriter, ZipWriter};
//! use std::io::Write;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut spools = MemorySpools;
//! let mut content = DeflateWriter::new(&mut spools, 1 << 20);
//! content.write_all(b"<worksheet/>")?;
//! let mut zip = ZipWriter::new(Vec::new());
//! zip.write_deflated(b"xl/worksheets/sheet1.xml", content.finish()?)?;
//! let bytes = zip.finish()?;
//! assert!(bytes.starts_with(b"PK\x03\x04"));
//! # Ok(())
//! # }
//! ```

mod deflate;
mod error;
mod zip;

pub use deflate::{DeflateWriter, Deflated};
pub use error::WriteError;
pub use zip::ZipWriter;

#[cfg(test)]
mod tests;
