//! xlsx-graft read: the parts of a workbook a reader of its cells needs,
//! read from a [`Package`](xlsx_graft_package::Package) in memory that does
//! not grow with the workbook's rows. For now, the shared strings table.
//!
//! # Finding a part
//!
//! [`Workbook::find`] takes the package's internal `officeDocument`
//! relationship to the workbook part, and [`Workbook::related`] the
//! workbook's internal relationship of a part type, such as the shared
//! strings table's: by relationship, never by a part name. Both the
//! Transitional and the Strict relationship types are taken (ECMA-376-4 and
//! ECMA-376-1:2016 §12.3). Two of them that target different parts are
//! `relationship_type_duplicate`, since readers would differ on which to
//! take, and neither is read; two that target the same part are one. An
//! external relationship targets no part, so it is not one: readers that
//! read parts pass it over. What is found is a
//! [`Presence`](xlsx_graft_package::Presence):
//!
//! - none, where the relationships are all known, is
//!   [`Absent`](xlsx_graft_package::Presence::Absent);
//! - none where one was left out for a reason already reported, or a
//!   duplicate refused, is [`Unknown`](xlsx_graft_package::Presence::Unknown),
//!   as the package model has it;
//! - one found where others were left out is taken: most are left out for a
//!   target that is not there, which no reader takes, and holding every part
//!   to the rest would leave it unknown for a dangling link anywhere in the
//!   workbook's relationships.
//!
//! A caller finds the workbook once and looks each part type up once, so
//! each duplicate is reported once.
//!
//! # The shared strings table
//!
//! [`SharedStrings::read`] reads the table (§18.4) as a stream, one `si` at
//! a time, so neither the part nor an object per string is ever held. The
//! value of a string is its `t`, then the `t` of each rich text run `r`, as
//! written: white space is kept whatever `xml:space` says, line ends are
//! normalized as XML 1.0 §2.11 has it, and the `_xHHHH_` escapes of
//! ST_Xstring (§22.9.2.19) are undone. The phonetic runs (`rPh`) and their
//! properties (`phoneticPr`) are not part of the value; they, a run's
//! properties (`rPr`) and the extension list (`extLst`) are skipped unread.
//!
//! The strings are one run of UTF-8 bytes and a table of where each ends,
//! a `u32` each. Both stay in memory until together they would pass
//! `sharedStringsSpillBytes`; then both are moved to a
//! [`Spool`](xlsx_graft_spool::Spool) the caller's factory makes, and only
//! a write buffer of half that threshold each stays. A string is looked up
//! by its index into a caller's `String` ([`SharedStrings::get`]).
//!
//! What the table is refused for, each its one diagnostic, leaves it
//! [`SharedStrings::Unknown`]:
//!
//! - **Schema** (`xml_schema_invalid`): an element or attribute the schema
//!   (CT_Sst, CT_Rst, CT_RElt, Strict `sml.xsd` and its Transitional twin)
//!   does not allow where it stands, in another namespace than the root's
//!   (one of the two `sst` namespaces), or out of the schema's order; text
//!   where only elements may stand; a run without its `t`. What is skipped
//!   is not checked inside, nor are the values of `count`, `uniqueCount`
//!   and `xml:space`, which nothing reads.
//!   No markup compatibility processing (ECMA-376-3) runs first, so an
//!   `mc:Ignorable` attribute, or an element or attribute of a namespace it
//!   names, is refused like any other. Part 3 lets a SpreadsheetML part
//!   carry them, unlike a package XML stream; the processing comes with the
//!   worksheets, which carry them, and this part reads through it then.
//! - **Limits**: more strings than `maxSharedStrings`
//!   (`shared_strings_too_many`), or more bytes of text than
//!   `maxSharedStringsBytes` (`shared_strings_too_large`), which holds the
//!   offsets to 32 bits.
//! - **XML**: whatever the XML reader refuses.
//!
//! The part's media type is not checked: the relationship says what it is.
//!
//! The part is always read to its end, and so its zip entry's CRC-32
//! checked: after a table read whole, and after a problem found on the way.
//! When the entry turns out damaged, its own diagnostic
//! (`zip_crc_mismatch`, `zip_inflate_failed`) is the only one: the problem
//! found before it was found in bytes that are not the ones written. A part
//! the zip layer refused reports nothing more.
//!
//! A `t` goes to the table as it is read, event by event, and is held to
//! the limits on the way; but one event of the XML is still held whole, as
//! the XML reader reads it, so one long run of text is bounded only by the
//! zip limits on an entry.
//!
//! An `Err` is a failure of the host alone ([`ReadError`]): reading the
//! package, or the spool.

mod error;
mod limits;
mod shared_strings;
mod workbook;

pub use error::ReadError;
pub use limits::ReadLimits;
pub use shared_strings::{Lookup, SHARED_STRINGS, SharedStrings, StringTable};
pub use workbook::Workbook;

#[cfg(test)]
mod tests;
