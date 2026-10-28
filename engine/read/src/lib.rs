//! xlsx-graft read: the parts of a workbook a reader of its cells needs,
//! read from a [`Package`](xlsx_graft_package::Package) in memory that does
//! not grow with the workbook's rows: the shared strings table, and what
//! the styles make of a cell's number.
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
//! # The styles
//!
//! [`Styles::read`] reads the styles part (§18.8) in memory, under
//! `maxXmlBytes` (`xml_too_large`), as the part is small: what it keeps is
//! the [`FormatKind`] of each cell format (`cellXfs/xf`), from its
//! `numFmtId`, and nothing else. The number formats the part defines
//! (`numFmts/numFmt`) are read on the way and their format codes dropped;
//! every other child of `styleSheet` and of `xf` is skipped unread. A
//! cell's format is looked up by its index, a cell's `s`
//! ([`Styles::kind`]). With no styles part every cell is General, as Excel
//! shows it; a styles part without `cellXfs` holds no cell format, not
//! even the 0 a cell without `s` has, so each is unknown: Excel requires
//! every format a cell refers to be there (MS-OI29500 on §18.8.39, note
//! a).
//!
//! - Only the cell format counts, not the cell style (`cellStyleXfs`) it
//!   comes from, and `applyNumberFormat` is not read: in a cell format it
//!   says whether a change to the style overwrites the format, not whether
//!   the format applies (MS-OI29500 on §18.8.10, note b, and on §18.8.45,
//!   note e).
//! - An `xf` with no `numFmtId` takes 0, as the default style has: General
//!   unless the part defines 0.
//! - A `numFmtId` the part defines takes its definition, a built-in one
//!   included; two definitions of one id that make different kinds leave
//!   it [`FormatKind::Unknown`]. One it does not define takes the built-in
//!   format of §18.8.30, and is unknown when there is none (5–8, 23–26,
//!   41–44, 63–66 and past 81), which the standard leaves to the definition
//!   the workbook ought to hold.
//! - The built-in ids 27–36 and 50–58 differ by language: each takes its
//!   Japanese and Korean meaning, a date but for the times 32 and 33.
//! - A format code is read token by token, never by a pattern: its up to
//!   four sections at `;`, with what is quoted (`"..."`), escaped (`\`, and
//!   the character after `_` and `*`) and bracketed (`[Red]`, `[>100]`,
//!   `[$-411]`) passed over, but for elapsed time (`[h]`, `[mm]`, `[ss]`).
//!   `General`, or `G/` and a localized name in letters other than ASCII
//!   (`G/標準`), is General.
//!   The date and time letters left (`y`, `m`, `d`, `h`, `s`, the era `g`
//!   and `e`, the Buddhist year `b` but for the calendars `b1` and `b2`,
//!   the Japanese day of the week `a`, in
//!   either case; `e` is an exponent only as `E+` or `E-`, as in Office)
//!   decide a section's kind, an `m` being minutes right after hours or
//!   right before seconds and a month otherwise. The sections for
//!   text (one with `@`, or the fourth) do not count unless they are all
//!   there is, nor do sections of literals alone (`"-"` for zero), which
//!   show no number; sections whose kinds differ make the format unknown,
//!   but for General and Number, which make Number. `_xHHHH_` escapes in
//!   a format code are undone first.
//!
//! What the styles are refused for, each its one diagnostic, leaves them
//! [`Styles::Unknown`]:
//!
//! - **Schema** (`xml_schema_invalid`): as for the shared strings table,
//!   against CT_Stylesheet, CT_NumFmts, CT_NumFmt, CT_CellXfs and CT_Xf;
//!   and a `numFmtId` that is not an `unsignedInt`, a `numFmt` without its
//!   `numFmtId` or `formatCode`, a `cellXfs` without an `xf`. The values of
//!   the other attributes are not checked.
//!   Of markup compatibility, `mc:Ignorable` alone is taken, as every part
//!   Excel writes carries it on `styleSheet`: an element or attribute of a
//!   namespace it names is passed over, on the element that names it and
//!   inside it. Any other markup compatibility element or attribute on or
//!   in an element read is refused, as is a prefix it names that is not
//!   declared or that names markup compatibility itself; one that names
//!   SpreadsheetML or XML's own namespace is taken and changes nothing, as
//!   a namespace the reader understands is read whatever it names. What a
//!   skipped element holds is not looked at.
//! - **Limits**: more cell formats than `maxCellFormats`
//!   (`cell_formats_too_many`), more number formats than `maxNumberFormats`
//!   (`number_formats_too_many`).
//! - **XML**: whatever the XML reader refuses.
//!
//! The part is read whole before its XML is, so a damaged entry reports
//! its own diagnostic and nothing more.
//!
//! An `Err` is a failure of the host alone ([`ReadError`]): reading the
//! package, or the spool.

mod compat;
mod error;
mod limits;
mod shared_strings;
mod styles;
mod workbook;

pub use error::ReadError;
pub use limits::ReadLimits;
pub use shared_strings::{Lookup, SHARED_STRINGS, SharedStrings, StringTable};
pub use styles::{FormatKind, STYLES, Styles};
pub use workbook::Workbook;

#[cfg(test)]
mod tests;
