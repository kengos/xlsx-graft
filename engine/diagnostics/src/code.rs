//! The code table: one row per diagnostic code, read by `diagnostics!`
//! (`table.rs`, which shows the shape of a row).
//!
//! A shipped code and its argument names are never renamed or repurposed,
//! and a removed code's wire string is never reused: the tests list every
//! removed one and refuse a row that brings it back. Every row needs a
//! golden JSON example in the tests.
//!
//! line-budget-exempt: the code table is data, one row per code, and grows by
//! a row with every code; splitting it would split the one place a reader
//! scans for a code.

use crate::table::diagnostics;
use crate::{
    DecidedBy, Echo, EncodingReason, Fix, InflateReason, Limit, ZipDirectoryReason, ZipHeaderField,
};

diagnostics! {
    /// More diagnostics were found than the collector keeps. The extra ones
    /// were counted and dropped; this one, added last, says how many, and
    /// how many of them were errors, since the records kept may hold none.
    DiagnosticsTruncated {
        /// How many diagnostics were dropped.
        dropped: u64,
        /// How many of the dropped diagnostics were errors.
        dropped_errors: u64,
        /// How many the collector keeps.
        limit: Limit,
    } => "diagnostics_truncated", Notice,
        message: format!("{dropped} more diagnostic(s), {dropped_errors} of them error(s), were found after the first {limit} and are not listed"),
        decided_by: Some(DecidedBy::limit("maxDiagnostics", *limit)),
        fix: None;

    /// A part's XML is not well-formed: the parser stopped at `offset`.
    XmlMalformed {
        /// The byte offset in the part's text, read as UTF-8 after any byte
        /// order mark: the start of the markup that holds the problem, or
        /// where the parser stopped.
        offset: u64,
        /// What the parser found there, in its own English words.
        detail: Echo,
    } => "xml_malformed", Error,
        message: format!("the XML is not well-formed at byte {offset}: {detail}"),
        decided_by: Some(DecidedBy::standard("XML 1.0 (Fifth Edition) §2.1")),
        fix: Some(Fix::workbook("save the workbook again from Excel or the tool that wrote it"));

    /// A part's XML declares a document type (DOCTYPE), which package XML
    /// may not: its entity declarations are a denial-of-service vector.
    XmlDoctype {
        /// The byte offset of the declaration, as for `xml_malformed`.
        offset: u64,
    } => "xml_doctype", Error,
        message: format!("the XML declares a DOCTYPE at byte {offset}; package XML may not use one"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.2.5")),
        fix: Some(Fix::workbook("remove the DOCTYPE declaration, or save the workbook again from Excel"));

    /// A part's XML is not in UTF-8 or UTF-16, the only encodings package
    /// XML may use, or does not decode as the one it is in.
    XmlEncodingUnsupported {
        /// The encoding the declaration names, or else the one the bytes were
        /// read as.
        encoding: Echo,
        /// Why it cannot be read.
        reason: EncodingReason,
    } => "xml_encoding_unsupported", Error,
        message: format!("the XML cannot be read as UTF-8 or UTF-16 ({reason}, encoding {encoding})"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.2.5")),
        fix: Some(Fix::workbook("save the workbook again from Excel, which writes UTF-8"));

    /// A part's XML nests elements deeper than the limit.
    XmlTooDeep {
        /// The byte offset of the first element past the limit, as for
        /// `xml_malformed`.
        offset: u64,
        /// The deepest nesting found; the root element is at depth 1. When
        /// the XML turns out malformed past the limit, the deepest before that.
        depth: u64,
        /// The deepest nesting allowed.
        limit: Limit,
    } => "xml_too_deep", Error,
        message: format!("the XML nests elements {depth} deep from byte {offset}; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxXmlDepth", *limit)),
        fix: Some(Fix::policy(format!("maxXmlDepth: {depth}")));

    /// One element of a part's XML has more attributes than the limit.
    XmlTooManyAttributes {
        /// The byte offset of the element, as for `xml_malformed`.
        offset: u64,
        /// How many attributes the element has: the first element past the
        /// limit, so a later one may have more.
        count: u64,
        /// The most attributes allowed on one element.
        limit: Limit,
    } => "xml_too_many_attributes", Error,
        message: format!("the XML element at byte {offset} has {count} attributes; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxXmlAttributes", *limit)),
        fix: Some(Fix::policy(format!("maxXmlAttributes: {count}")));

    /// A part's XML declares more namespaces in scope at once than the limit.
    XmlTooManyNamespaces {
        /// The byte offset of the element whose declarations cross the
        /// limit, as for `xml_malformed`.
        offset: u64,
        /// How many namespace declarations are in scope at that element: the
        /// first past the limit, so a later one may have more.
        count: u64,
        /// The most declarations allowed in scope at once.
        limit: Limit,
    } => "xml_too_many_namespaces", Error,
        message: format!("the XML element at byte {offset} has {count} namespace declarations in scope; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxXmlNamespaces", *limit)),
        fix: Some(Fix::policy(format!("maxXmlNamespaces: {count}")));

    /// A part's XML is larger than the engine reads into memory.
    XmlTooLarge {
        /// The part's size in bytes: its length when in memory, else the
        /// size its zip entry declares, or the bytes read before stopping
        /// when that is larger.
        size: u64,
        /// The most bytes allowed.
        limit: Limit,
    } => "xml_too_large", Error,
        message: format!("the XML is {size} bytes; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxXmlBytes", *limit)),
        fix: Some(Fix::policy(format!("maxXmlBytes: {size}")));

    /// The file is not a zip, so it cannot be a package.
    NotAZip {} => "not_a_zip", Error,
        message: String::from("the file is not a zip, so it is not an xlsx package"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.3.1")),
        fix: Some(Fix::workbook("save the file again as an Excel workbook (.xlsx)"));

    /// The file is an OLE compound file, not a zip: a password-protected
    /// workbook, which Excel encrypts into one, or a legacy `.xls`.
    OleCompoundFile {} => "ole_compound_file", Error,
        message: String::from("the file is an OLE compound file, not a zip: a password-protected workbook or a legacy .xls"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.3.1")),
        fix: Some(Fix::workbook("remove the password, or open the file in Excel and save it as .xlsx"));

    /// A record the zip's end records point to runs past the end of the
    /// file, or past where the records after it start: the file was cut
    /// short or damaged. With an entry as its location, that entry's local
    /// record.
    ZipTruncated {} => "zip_truncated", Error,
        message: String::from("the zip ends before a record it describes: the file is truncated or damaged"),
        decided_by: Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.3.6")),
        fix: Some(Fix::workbook("copy the file again from where it came from, or save the workbook again from Excel"));

    /// The zip's central directory cannot be read as its end records
    /// describe it.
    ZipDirectoryInvalid {
        /// What is wrong with it.
        reason: ZipDirectoryReason,
    } => "zip_directory_invalid", Error,
        message: format!("the zip's central directory cannot be read ({reason})"),
        decided_by: Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.3.12")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The zip holds bytes before its central directory that no entry's
    /// record accounts for: a reader that walks the local headers would find
    /// what the central directory does not list. Reported at the entry the
    /// bytes come before, or without an entry when they come before the
    /// directory; the entries are still read.
    ZipUnlistedData {
        /// How many bytes no record accounts for.
        size: u64,
    } => "zip_unlisted_data", Error,
        message: format!("the zip holds {size} bytes that no entry in its central directory accounts for"),
        decided_by: Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.3.6")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The zip spans more than one disk, which a package may not.
    ZipMultiDisk {} => "zip_multi_disk", Error,
        message: String::from("the zip is split across disks, which a package may not be"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 Annex B.4, Table B.2")),
        fix: Some(Fix::workbook("join the parts into one file, or save the workbook again from Excel"));

    /// The zip starts after other data: its first local header is not at the
    /// start of the file, or its offsets do not count the data in front.
    ZipPrependedData {
        /// How many bytes come before the zip.
        size: u64,
    } => "zip_prepended_data", Error,
        message: format!("the zip starts after {size} bytes of other data"),
        decided_by: Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.3.6")),
        fix: Some(Fix::workbook("remove the data in front of the zip, or save the workbook again from Excel"));

    /// The zip's end record lists more entries than the limit. The central
    /// directory is not read.
    ZipEntryCountExceeded {
        /// How many entries the end record lists.
        count: u64,
        /// The most entries allowed.
        limit: Limit,
    } => "zip_entry_count_exceeded", Error,
        message: format!("the zip lists {count} entries; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxEntries", *limit)),
        fix: Some(Fix::policy(format!("maxEntries: {count}")));

    /// The zip's end record gives its central directory more bytes than the
    /// limit. The central directory is not read.
    ZipDirectoryTooLarge {
        /// The central directory's size in bytes, as the end record gives it.
        size: u64,
        /// The most bytes allowed.
        limit: Limit,
    } => "zip_directory_too_large", Error,
        message: format!("the zip's central directory is {size} bytes; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxDirectoryBytes", *limit)),
        fix: Some(Fix::policy(format!("maxDirectoryBytes: {size}")));

    /// A zip entry's name is longer than the limit. The entry is not read.
    EntryNameTooLong {
        /// The name's length in bytes.
        length: u64,
        /// The longest name allowed, in bytes.
        limit: Limit,
    } => "entry_name_too_long", Error,
        message: format!("the zip entry name is {length} bytes; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxEntryNameBytes", *limit)),
        fix: Some(Fix::policy(format!("maxEntryNameBytes: {length}")));

    /// A zip entry is encrypted, which a package's entries may not be. The
    /// entry is not read.
    ZipEncryptedEntry {} => "zip_encrypted_entry", Error,
        message: String::from("the zip entry is encrypted, which a package's entries may not be"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.3.6")),
        fix: Some(Fix::workbook("save the workbook again from Excel without zip encryption"));

    /// A zip entry is compressed with a method other than stored or deflate,
    /// the only two a package may use. The entry is not read.
    ZipUnsupportedCompression {
        /// The compression method's number in the zip.
        method: u64,
    } => "zip_unsupported_compression", Error,
        message: format!("the zip entry is compressed with method {method}; a package may use only stored (0) and deflate (8)"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.3.6")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A zip entry's local header or data descriptor disagrees with its
    /// central directory header. The entry is not read.
    ZipHeaderMismatch {
        /// The first field found to disagree.
        field: ZipHeaderField,
    } => "zip_header_mismatch", Error,
        message: format!("the zip entry's local header or data descriptor disagrees with its central directory header on {field}"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 Annex B.2")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A zip entry's local record overlaps another entry's, so one stretch of
    /// the file would be read as both. The entry is not read; the other one,
    /// which starts first, keeps the bytes.
    ZipEntriesOverlap {
        /// The name of the entry this one overlaps.
        other: Echo,
    } => "zip_entries_overlap", Error,
        message: format!("the zip entry's data overlaps that of the entry {other}"),
        decided_by: Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.3.6")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A zip entry's data runs into the central directory. The entry is not
    /// read.
    ZipEntryOverlapsDirectory {} => "zip_entry_overlaps_directory", Error,
        message: String::from("the zip entry's data runs into the central directory"),
        decided_by: Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.3.6")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A zip entry declares an inflated size larger than the limit. The
    /// entry is not read.
    ZipEntryTooLarge {
        /// The inflated size the entry declares, in bytes.
        size: u64,
        /// The most bytes allowed for one entry.
        limit: Limit,
    } => "zip_entry_too_large", Error,
        message: format!("the zip entry inflates to {size} bytes; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxEntryBytes", *limit)),
        fix: Some(Fix::policy(format!("maxEntryBytes: {size}")));

    /// The zip's entries declare more inflated bytes in all than the limit.
    /// Reported once, at the entry that crosses it; that entry and every
    /// later one are not read.
    ZipTotalTooLarge {
        /// The inflated sizes the entries declare, in all.
        size: u64,
        /// The most bytes allowed in all.
        limit: Limit,
    } => "zip_total_too_large", Error,
        message: format!("the zip's entries inflate to {size} bytes in all; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxTotalBytes", *limit)),
        fix: Some(Fix::policy(format!("maxTotalBytes: {size}")));

    /// A zip entry declares an inflated size more than the limit's times its
    /// compressed size: the shape of a zip bomb. The entry is not read.
    ZipRatioExceeded {
        /// The inflated size the entry declares, in bytes.
        size: u64,
        /// The entry's compressed size, in bytes.
        compressed_size: u64,
        /// The highest ratio allowed.
        limit: Limit,
    } => "zip_ratio_exceeded", Error,
        message: format!("the zip entry inflates {compressed_size} bytes to {size}, more than {limit} times as many"),
        decided_by: Some(DecidedBy::limit("maxCompressionRatio", *limit)),
        // No ratio admits data that takes no bytes yet inflates to some.
        fix: Some(match *compressed_size {
            0 => Fix::workbook("save the workbook again from Excel"),
            compressed_size => Fix::policy(format!("maxCompressionRatio: {}", size.div_ceil(compressed_size))),
        });

    /// A zip entry's inflated data does not have the CRC-32 its header
    /// gives: it is damaged.
    ZipCrcMismatch {
        /// The CRC-32 the header gives.
        expected: u64,
        /// The CRC-32 of the data.
        actual: u64,
    } => "zip_crc_mismatch", Error,
        message: format!("the zip entry's data has CRC-32 {actual:08x}; its header gives {expected:08x}"),
        decided_by: Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.4.7")),
        fix: Some(Fix::workbook("copy the file again from where it came from, or save the workbook again from Excel"));

    /// A zip entry's data does not inflate to what its header declares.
    ZipInflateFailed {
        /// What went wrong.
        reason: InflateReason,
    } => "zip_inflate_failed", Error,
        message: format!("the zip entry's data cannot be inflated ({reason})"),
        decided_by: Some(DecidedBy::standard(match reason {
            InflateReason::CorruptData => "RFC 1951 §3.2",
            InflateReason::EndedEarly | InflateReason::LongerThanDeclared => "PKWARE APPNOTE 6.3.10 §4.4.9",
        })),
        fix: Some(Fix::workbook("copy the file again from where it came from, or save the workbook again from Excel"));
}

#[cfg(test)]
mod tests;
