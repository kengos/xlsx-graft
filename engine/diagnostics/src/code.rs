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
    DecidedBy, Echo, EncodingReason, Fix, InflateReason, Limit, PartNameReason, SchemaReason,
    ZipDirectoryReason, ZipHeaderField,
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

    /// An XML part breaks its schema: a package XML stream (the content
    /// types stream or a Relationships part), whose error spoils only the
    /// element, or the whole stream when its root is wrong; or the shared
    /// strings table, which is then not read at all.
    XmlSchemaInvalid {
        /// The byte offset of the element, as for `xml_malformed`.
        offset: u64,
        /// How it breaks the schema.
        reason: SchemaReason,
        /// The element or attribute, as written.
        name: Echo,
    } => "xml_schema_invalid", Error,
        message: format!("the XML breaks its schema at byte {offset} ({reason}: {name})"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 Annex C, ECMA-376-1:2016 Annex A")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

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

    /// A zip entry's name is not a part name, so the entry is not a part.
    EntryNameInvalid {
        /// What is wrong with the name.
        reason: PartNameReason,
    } => "entry_name_invalid", Error,
        message: format!("the zip entry name is not a part name ({reason}), so the entry is not read as a part"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.2.2.2")),
        fix: Some(Fix::workbook("rename the entry to a valid part name, or save the workbook again from Excel"));

    /// Two zip entries name the same part, their names equal but for ASCII
    /// letter case. Reported at every entry after the first; none of them
    /// is read as a part, since readers differ on which one they take.
    PartNameDuplicate {
        /// The first entry with the name.
        other: Echo,
    } => "part_name_duplicate", Error,
        message: format!("the zip entry names the same part as the entry {other}; neither is read as a part"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.2.2.3")),
        fix: Some(Fix::workbook("remove one of the entries, or save the workbook again from Excel"));

    /// A part's name continues another part's name with more segments
    /// (`/a/b` beside `/a`). It is not read as a part; the other one is.
    PartNameDerived {
        /// The part name it is derived from.
        other: Echo,
    } => "part_name_derived", Error,
        message: format!("the part name is derived from the part name {other}, so it is not read as a part"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.2.2.3")),
        fix: Some(Fix::workbook("rename one of the parts, or save the workbook again from Excel"));

    /// The package has no content types stream (`[Content_Types].xml`), so
    /// no part has a media type. Reported once, instead of at every part.
    ContentTypesMissing {} => "content_types_missing", Error,
        message: String::from("the package has no [Content_Types].xml, so no part has a media type"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.3.7")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The content types stream has more than one `Default` for an
    /// extension, letter case aside. None of them applies.
    ContentTypesDuplicateDefault {
        /// The extension, as the later `Default` spells it.
        extension: Echo,
    } => "content_types_duplicate_default", Error,
        message: format!("[Content_Types].xml has more than one Default for the extension {extension}; none of them applies"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.2.3.2.1")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The content types stream has more than one `Override` for a part
    /// name, letter case aside. None of them applies.
    ContentTypesDuplicateOverride {
        /// The part name, as the later `Override` spells it.
        part_name: Echo,
    } => "content_types_duplicate_override", Error,
        message: format!("[Content_Types].xml has more than one Override for the part {part_name}; none of them applies"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.2.3.2.1")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// An `Override`'s `PartName` is not a part name, so it matches no part.
    ContentTypesPartNameInvalid {
        /// The `PartName`, as written.
        part_name: Echo,
        /// What is wrong with it.
        reason: PartNameReason,
    } => "content_types_part_name_invalid", Error,
        message: format!("[Content_Types].xml has an Override for {part_name}, which is not a part name ({reason})"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.2.3.2.5")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A media type in the content types stream does not follow RFC 7231
    /// §3.1.1.1 without white space or comments. The `Default` or
    /// `Override` that gives it applies to no part.
    ContentTypeInvalid {
        /// The media type, as written.
        content_type: Echo,
    } => "content_type_invalid", Error,
        message: format!("[Content_Types].xml gives the media type {content_type}, which is not one"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.2.3.2.4")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A part has no media type: no `Override` names it and no `Default`
    /// matches its extension.
    PartContentTypeMissing {} => "part_content_type_missing", Error,
        message: String::from("the part has no media type in [Content_Types].xml"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §7.2.3.2.1")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A part's media type disagrees with its name: a Relationships part's
    /// name takes the Relationships media type, and no other part's does.
    PartContentTypeMismatch {
        /// The part's media type.
        content_type: Echo,
    } => "part_content_type_mismatch", Error,
        message: format!("the part's media type {content_type} does not fit its name: the Relationships media type is for Relationships parts, and only them"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.5.2.1")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A Relationships part's source part is not in the package. Its
    /// relationships are not read.
    RelationshipsSourceMissing {} => "relationships_source_missing", Error,
        message: String::from("the Relationships part's source part is not in the package, so its relationships are not read"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.5.2.3")),
        fix: Some(Fix::workbook("remove the Relationships part, or save the workbook again from Excel"));

    /// A Relationships part has more relationships than the limit. None of
    /// them is read.
    RelationshipsTooMany {
        /// How many relationships the part has.
        count: u64,
        /// The most relationships allowed in one part.
        limit: Limit,
    } => "relationships_too_many", Error,
        message: format!("the Relationships part has {count} relationships; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxRelationships", *limit)),
        fix: Some(Fix::policy(format!("maxRelationships: {count}")));

    /// The package's Relationships parts hold more relationships in all than
    /// the limit. Reported once, at the part that crosses it; that part and
    /// every later one are not read.
    RelationshipsTotalTooMany {
        /// How many relationships the parts read so far hold, that one's
        /// included.
        count: u64,
        /// The most relationships allowed in all.
        limit: Limit,
    } => "relationships_total_too_many", Error,
        message: format!("the package's Relationships parts hold {count} relationships so far; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxTotalRelationships", *limit)),
        fix: Some(Fix::policy(format!("maxTotalRelationships: {count}")));

    /// Two relationships in one Relationships part share an `Id`. Reported
    /// at every one after the first; none of them is read.
    RelationshipIdDuplicate {} => "relationship_id_duplicate", Error,
        message: String::from("another relationship in the same Relationships part has this Id; none of them is read"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.5.3.4")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// An internal relationship's target is not a relative reference that
    /// resolves to a part name. The relationship is not read.
    RelationshipTargetInvalid {
        /// The `Target`, as written.
        target: Echo,
        /// What is wrong with it.
        reason: PartNameReason,
    } => "relationship_target_invalid", Error,
        message: format!("the relationship's target {target} does not resolve to a part name ({reason})"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.5.3.4")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// An internal relationship's target climbs past the package root with
    /// `..`, where resolving stays at the root, as RFC 3986 §5.2.4 has it.
    /// The relationship is read.
    RelationshipTargetAboveRoot {
        /// The `Target`, as written.
        target: Echo,
    } => "relationship_target_above_root", Notice,
        message: format!("the relationship's target {target} climbs above the package root; it resolves as if it stopped there"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.4.3.5")),
        fix: None;

    /// An internal relationship's target names no part in the package.
    RelationshipTargetMissing {
        /// The part name the target resolves to.
        part_name: Echo,
    } => "relationship_target_missing", Error,
        message: format!("the relationship's target {part_name} is not a part in the package"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.5.3.4")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A relationship is from or to a Relationships part, which may have
    /// none. The relationship is not read. A Relationships part whose source
    /// is one is reported once, without a relationship, and none of its
    /// relationships is read.
    RelationshipRelsPart {} => "relationship_rels_part", Error,
        message: String::from("the relationship is from or to a Relationships part, which may have no relationships"),
        decided_by: Some(DecidedBy::standard("ECMA-376-2:2021 §6.5.2.1")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The package relationships have no `officeDocument` relationship to a
    /// part, so the package has no main document: it is not a workbook.
    MainDocumentMissing {} => "main_document_missing", Error,
        message: String::from("the package relationships name no main document (officeDocument), so the package is not a workbook"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 SpreadsheetML Workbook part")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// No relationship targets the part, so nothing that follows
    /// relationships reaches it. Relationships parts are reached by name and
    /// are never reported.
    PartUnreferenced {} => "part_unreferenced", Notice,
        message: String::from("no relationship targets the part, so a reader that follows relationships never reaches it"),
        decided_by: None,
        fix: None;

    /// Two relationships of one type in one Relationships part target
    /// different parts where the part type allows one, such as the
    /// workbook's shared strings table: which one a reader takes is not
    /// known, so neither is read. Reported once, at the first relationship
    /// whose target differs from the first one's.
    RelationshipTypeDuplicate {
        /// The relationship type, as written.
        rel_type: Echo,
    } => "relationship_type_duplicate", Error,
        message: format!("another relationship of type {rel_type} in the same Relationships part targets another part, where only one is allowed; neither is read"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §12.3")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The shared strings table holds more strings than the limit. The table
    /// is not read.
    SharedStringsTooMany {
        /// The strings counted when reading stopped: the first past the
        /// limit, so the table holds at least this many.
        count: u64,
        /// The most strings allowed.
        limit: Limit,
    } => "shared_strings_too_many", Error,
        message: format!("the shared strings table holds at least {count} strings; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxSharedStrings", *limit)),
        fix: Some(Fix::policy(format!("maxSharedStrings: {count}")));

    /// The text of the shared strings table, as UTF-8 once read, is larger
    /// than the limit. The table is not read. The offsets into the text are
    /// 32-bit, so the limit is never past `u32::MAX`, and a table that
    /// passes that is sent back to the workbook.
    SharedStringsTooLarge {
        /// The bytes of text when reading stopped: the text read so far and
        /// the piece of it that passed the limit, so the table holds at least
        /// this much.
        size: u64,
        /// The most bytes of text allowed.
        limit: Limit,
    } => "shared_strings_too_large", Error,
        message: format!("the shared strings table holds at least {size} bytes of text; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxSharedStringsBytes", *limit)),
        fix: Some(if *size <= u64::from(u32::MAX) {
            Fix::policy(format!("maxSharedStringsBytes: {size}"))
        } else {
            Fix::workbook("split the workbook: no limit admits more than 4 GiB of shared strings")
        });
}

#[cfg(test)]
mod tests;
