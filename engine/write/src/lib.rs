//! xlsx-graft write: edits an xlsx package and writes the zip container of
//! the result, entry by entry.
//!
//! # Editing a package
//!
//! [`PackageEdit`] takes a `Package` read by `xlsx-graft-package` and
//! collects changes to it: [`replace`](PackageEdit::replace) a part (its
//! [`PartRewriter`] gets the original content and writes the new),
//! [`add`](PackageEdit::add) a part (its [`PartWriter`] writes the content;
//! the edit writes its `Override` in `[Content_Types].xml`, and a
//! relationship to it when the caller gives a [`Link`]) and
//! [`remove`](PackageEdit::remove) a part (with its Relationships part, the
//! `Override`s and the relationships that target it).
//! [`finish`](PackageEdit::finish) then writes the zip. The entries it does
//! not change are copied as they are, so a workbook's parts the edit does
//! not know (a pivot cache, a macro project, an extension list) come out as
//! they went in; a part is read and written one at a time, so the memory a
//! write takes does not grow with the size of the workbook but with the
//! largest part rewritten, up to [`WriteLimits::part_spill_bytes`].
//!
//! A problem in the workbook (an entry that cannot be read, content a
//! rewriter refuses) is a diagnostic, and `finish` returns `Ok(None)`. One
//! `Ok(None)` has no diagnostic of `finish`'s own: the edit adds or removes a
//! part and the content types stream cannot say what every mapping names (no
//! stream, two, one refused for its name or over the XML limits or not
//! well-formed, or a mapping that names nothing). The reason is the
//! diagnostic that `Package::open` pushed for that stream, so
//! a host keeps the `Diagnostics` it passed to `Package::open` to show it. After
//! `Ok(None)` and after a [`WriteError`], the output holds the bytes written
//! so far, which are not a zip: a host that passed a pipe or a socket may
//! have received part of one. A caller that writes to a file uses
//! [`write_file_atomically`] (not built for WebAssembly), which renames a
//! temporary file to the path only when the zip was written whole. A caller
//! that writes to memory takes the bytes only from `Ok(Some(_))`.
//!
//! # Writing a zip
//!
//! [`PackageEdit`] writes through [`ZipWriter`], which can be used alone. An
//! entry is copied from a zip read by `xlsx-graft-package` without being
//! inflated and deflated again ([`ZipWriter::copy_entry`]), or is new content deflated here
//! ([`DeflateWriter`], then [`ZipWriter::write_deflated`]), held in memory
//! until it passes a size the caller sets and then in scratch storage
//! (`xlsx-graft-spool`).
//!
//! # Writing a worksheet
//!
//! [`sheet`] holds what a new worksheet and an edit of one write the same
//! way: the order of the root's children ([`sheet::WorksheetChild`]) and
//! where the kinds a caller writes go among them ([`sheet::ChildPlacer`]),
//! and the values of `cols`, `row`, `sheetViews`, `mergeCells` and
//! `sheetProtection`, each refused when made if Excel cannot hold it.
//!
//! # Writing rows
//!
//! [`RowsWriter`] writes the `<row>` elements of a worksheet's `<sheetData>`
//! one row at a time, deflating each row when it is pushed, and
//! [`finish`](RowsWriter::finish) returns them as [`SpooledRows`], which
//! inflates and writes them on each call
//! of [`write_sheet_data`](SpooledRows::write_sheet_data).
//!
//! - **Order**: the rows come in ascending row order and the cells of a row
//!   in ascending column order, rows 1 to 1,048,576 and columns 1 to
//!   16,384. A row or cell that does not follow the order above, or a
//!   value `write_cell` refuses, is an error of the caller, and the writer
//!   is as it was before the call: nothing of the row was written.
//! - **Empty rows**: a row without cells writes nothing, and its number
//!   still counts for the order.
//! - **Cells**: each cell is written by `write_cell` without changing a
//!   string into a shared string: a string is an inline string
//!   (`inlineStr`), a shared string is written by its number.
//!   A row has no attribute besides `r`.
//! - **The range**: [`cells`](SpooledRows::cells) is the top-left and
//!   bottom-right cell of every cell written, and `None` when there is no
//!   cell. [`truncated`](SpooledRows::truncated) counts the cells whose
//!   string `write_cell` cut at 32,767 UTF-16 code units.
//! - **What is not written**: the worksheet part itself, from `<worksheet>`
//!   to `</sheetData>` and after, is written by the caller:
//!   `write_sheet_data` writes the rows only.
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

// No temporary file on WebAssembly: the host brings its own way to write a file.
#[cfg(not(target_family = "wasm"))]
mod atomic;
mod deflate;
mod edit;
mod error;
mod limits;
mod part;
mod rows;
pub mod sheet;
pub mod structure;
mod zip;

#[cfg(not(target_family = "wasm"))]
pub use atomic::write_file_atomically;
pub use deflate::{DeflateWriter, Deflated};
pub use edit::{Link, LinkSource, PackageEdit};
pub use error::WriteError;
pub use limits::WriteLimits;
pub use part::{PartRewriter, PartWriter};
pub use rows::{Row, RowCell, RowsWriter, SpooledRows};
pub use zip::ZipWriter;

#[cfg(test)]
mod tests;
