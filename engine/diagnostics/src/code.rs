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
    DecidedBy, Echo, EncodingReason, Fix, FormulaMalformedReason, FunctionDenial, InflateReason,
    Limit, PartKind, PartNameReason, SchemaReason, SheetNameReason, TargetApp, TargetFallback,
    TargetList, TargetReason, ZipDirectoryReason, ZipHeaderField,
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

    /// A namespace declaration in a part's XML is longer, as written, than
    /// the limit.
    XmlNamespaceTooLong {
        /// The byte offset of the element that declares it, as for
        /// `xml_malformed`.
        offset: u64,
        /// The bytes of the declaration's value, as written.
        length: u64,
        /// The most bytes a declaration's value may hold.
        limit: Limit,
    } => "xml_namespace_too_long", Error,
        message: format!("the XML element at byte {offset} declares a namespace of {length} bytes; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxXmlNamespaceBytes", *limit)),
        fix: Some(Fix::policy(format!("maxXmlNamespaceBytes: {length}")));

    /// One event of a part's XML, a tag, a run of text, a CDATA section, a
    /// comment, a processing instruction, the declaration, a document type
    /// declaration or a reference, is longer than the limit. Reading stops one byte past it, so the event's whole length is
    /// not known.
    XmlEventTooLarge {
        /// The byte offset where the event starts, as for `xml_malformed`.
        offset: u64,
        /// The bytes of the event read when reading stopped, as UTF-8: the
        /// event holds at least this many.
        size: u64,
        /// The most bytes one event may hold.
        limit: Limit,
    } => "xml_event_too_large", Error,
        message: format!("the XML event at byte {offset} holds at least {size} bytes; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxXmlEventBytes", *limit)),
        fix: Some(Fix::policy(format!("maxXmlEventBytes: {size}")));

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
    /// strings table or the styles, which are then not read at all.
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

    /// A string of the shared strings table is longer than the 32,767
    /// characters, counted in UTF-16 code units, Excel keeps in a cell:
    /// Excel for Mac asks to repair the workbook, and cuts the string there,
    /// whether or not a cell refers to it.
    /// Its text is its `t`, or the `t` of each of its runs together; its
    /// phonetic runs (`rPh`) are not counted. Each `_xHHHH_` escape counts
    /// as the one code unit it stands for, an unpaired surrogate included,
    /// as Excel counts it.
    SharedStringTooLong {
        /// The string's index in the table, from 0, as a cell of `t="s"`
        /// gives it.
        index: u64,
        /// The string's length in UTF-16 code units.
        length: u64,
    } => "shared_string_too_long", Error,
        message: format!("shared string {index} is {length} UTF-16 code units long; Excel keeps 32767 and cuts the rest"),
        decided_by: Some(DecidedBy::Default {}),
        fix: Some(Fix::workbook("shorten the string to 32767 UTF-16 code units"));

    /// The styles part's cell formats (`cellXfs`) hold more `xf` records
    /// than the limit. The styles are not read.
    CellFormatsTooMany {
        /// The records counted when reading stopped: the first past the
        /// limit, so the part holds at least this many.
        count: u64,
        /// The most cell formats allowed.
        limit: Limit,
    } => "cell_formats_too_many", Error,
        message: format!("the styles part holds at least {count} cell formats; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxCellFormats", *limit)),
        fix: Some(Fix::policy(format!("maxCellFormats: {count}")));

    /// The styles part's number formats (`numFmts`) hold more `numFmt`
    /// records than the limit. The styles are not read.
    NumberFormatsTooMany {
        /// The records counted when reading stopped: the first past the
        /// limit, so the part holds at least this many.
        count: u64,
        /// The most number formats allowed.
        limit: Limit,
    } => "number_formats_too_many", Error,
        message: format!("the styles part holds at least {count} number formats; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxNumberFormats", *limit)),
        fix: Some(Fix::policy(format!("maxNumberFormats: {count}")));

    /// Two sheets of the workbook have the same name, letter case aside,
    /// which Excel does not allow in any locale. Reported at every sheet
    /// after the first; both stay listed.
    SheetNameDuplicate {
        /// The sheet's name.
        name: Echo,
        /// The name of the sheet before it with the same name.
        other: Echo,
    } => "sheet_name_duplicate", Error,
        message: format!("the sheet {name} has the same name as the sheet {other}, letter case aside"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.2.19; MS-OI29500 on §18.2.19, note d")),
        fix: Some(Fix::workbook("rename one of the sheets, or save the workbook again from Excel"));

    /// Two sheets of the workbook are one part: their relationships, or
    /// their one relationship, target the same part. Reported at every
    /// sheet after the first; both stay listed.
    SheetPartDuplicate {
        /// The sheet's name.
        name: Echo,
        /// The name of the sheet before it that is the same part.
        other: Echo,
    } => "sheet_part_duplicate", Error,
        message: format!("the sheet {name} is the same part as the sheet {other}"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.2.19")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A sheet's `r:id` names no relationship of the workbook part that
    /// targets a part: none has the `Id` (reported against the workbook
    /// part), or the one that has it is external (reported against it).
    /// The sheet is listed without its part.
    SheetRelationshipMissing {
        /// The sheet's `r:id`.
        relationship_id: Echo,
    } => "sheet_relationship_missing", Error,
        message: format!("the sheet's relationship {relationship_id} is not a relationship of the workbook to a part, so the sheet has no part"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.2.19")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The relationship a sheet's `r:id` names is not of a sheet's type
    /// (a worksheet, chartsheet, dialogsheet or macro sheet). The sheet is
    /// listed without its part.
    SheetRelationshipTypeInvalid {
        /// The relationship type, as written.
        rel_type: Echo,
    } => "sheet_relationship_type_invalid", Error,
        message: format!("a sheet's relationship is of type {rel_type}, which is not a sheet's, so the sheet has no part"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §12.3")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    // What Excel repairs as it opens the workbook part's sheets and names
    // and the table parts' tables, or reads otherwise than they say.

    /// A sheet's name is one Excel does not take, so it repairs the
    /// workbook as it opens it and renames the sheet. A name the same as
    /// another sheet's is `sheet_name_duplicate`.
    SheetNameInvalid {
        /// The sheet's name.
        name: Echo,
        /// What is wrong with it.
        reason: SheetNameReason,
    } => "sheet_name_invalid", Error,
        message: match reason {
            SheetNameReason::Empty => "a sheet's name is empty".to_owned(),
            SheetNameReason::TooLong => format!("the sheet name {name} is longer than 31 characters, counted in UTF-16 code units"),
            SheetNameReason::Character => format!("the sheet name {name} holds one of [ ] : * ? / \\"),
            SheetNameReason::Apostrophe => format!("the sheet name {name} starts or ends with '"),
        },
        decided_by: Some(DecidedBy::standard("MS-OI29500 on ECMA-376-1:2016 §18.2.19, note a")),
        fix: Some(Fix::workbook("rename the sheet"));

    /// Two sheets of the workbook have the same `sheetId`, so Excel
    /// repairs the workbook as it opens it and gives the later one another.
    /// Reported at every sheet after the first.
    SheetIdDuplicate {
        /// The sheet's name.
        name: Echo,
        /// The name of the sheet before it with the same `sheetId`.
        other: Echo,
        /// The `sheetId` they share.
        sheet_id: u64,
    } => "sheet_id_duplicate", Error,
        message: format!("the sheet {name} has the same sheetId {sheet_id} as the sheet {other}"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.2.19")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A defined name's name is not one a formula can name, so Excel
    /// repairs the workbook as it opens it and drops the defined name.
    DefinedNameInvalid {
        /// The defined name's name.
        name: Echo,
    } => "defined_name_invalid", Error,
        message: format!("the defined name {name} is empty, or does not start with a letter, _ or \\ and go on with letters, digits, _, \\, . and ?"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.17.2.5; MS-OI29500 on §18.2.5, note q, and on §18.17.2.5, note a")),
        fix: Some(Fix::workbook("rename the defined name"));

    /// A defined name's `localSheetId` counts past the last sheet, so
    /// Excel repairs the workbook as it opens it and drops the defined
    /// name.
    DefinedNameSheetIndexInvalid {
        /// The defined name's name.
        name: Echo,
        /// Its `localSheetId`.
        local_sheet_id: u64,
        /// How many sheets the workbook has.
        sheets: u64,
    } => "defined_name_sheet_index_invalid", Error,
        message: format!("the defined name {name} is scoped to the sheet at index {local_sheet_id}, but the workbook has {sheets} sheet(s)"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.2.5; MS-OI29500 on §18.2.5, note l")),
        fix: Some(Fix::workbook("scope the defined name to a sheet the workbook has, or to the workbook"));

    /// Two defined names of one scope have the same name, letter case
    /// aside. Excel opens the workbook without a word and keeps only the
    /// later one. Reported at every defined name before the last.
    DefinedNameDuplicate {
        /// The defined name's name.
        name: Echo,
        /// The name of the defined name after it, in the same scope, that
        /// Excel keeps instead.
        other: Echo,
    } => "defined_name_duplicate", Notice,
        message: format!("the defined name {name} has the same name as the later defined name {other} of its scope, letter case aside; Excel keeps only the later one"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.17.2.5; MS-OI29500 on §18.2.5, note n")),
        fix: Some(Fix::workbook("remove or rename one of the defined names"));

    /// A defined name refers to a sheet the workbook does not have. Excel
    /// opens the workbook without a word and reads the reference as one to
    /// another workbook. Reported once per sheet a defined name names.
    DefinedNameSheetMissing {
        /// The defined name's name.
        name: Echo,
        /// The sheet it refers to.
        sheet: Echo,
    } => "defined_name_sheet_missing", Notice,
        message: format!("the defined name {name} refers to the sheet {sheet}, which the workbook does not have; Excel reads it as a sheet of another workbook"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.17.2.3")),
        fix: Some(Fix::workbook("point the defined name at a sheet the workbook has"));

    /// Two tables of one sheet overlap, so Excel repairs the workbook as
    /// it opens it and drops the later one. Reported at every table that
    /// overlaps one listed before it.
    TableRangeOverlap {
        /// The table's `displayName`.
        name: Echo,
        /// The `displayName` of the table before it that it overlaps.
        other: Echo,
    } => "table_range_overlap", Error,
        message: format!("the table {name} overlaps the table {other} on the same sheet"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.5.1.2")),
        fix: Some(Fix::workbook("move or resize one of the tables"));

    /// A table's `displayName` is the same as another table's, or as a
    /// defined name's, letter case aside, so Excel repairs the workbook as
    /// it opens it and renames the table. Reported at every table after
    /// the first with the name.
    TableNameDuplicate {
        /// The table's `displayName`.
        name: Echo,
        /// The name it has the same as.
        other: Echo,
        /// Whether `other` is a defined name's; else it is the
        /// `displayName` of a table before it.
        defined_name: bool,
    } => "table_name_duplicate", Error,
        message: if *defined_name {
            format!("the table {name} has the same name as the defined name {other}, letter case aside")
        } else {
            format!("the table {name} has the same name as the table {other}, letter case aside")
        },
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.5.1.2")),
        fix: Some(Fix::workbook("rename the table"));

    /// Two tables of the workbook have the same `id`, so Excel repairs the
    /// workbook as it opens it and drops the later one. Reported at every
    /// table after the first with the `id`.
    TableIdDuplicate {
        /// The table's `displayName`.
        name: Echo,
        /// The `displayName` of the table before it with the same `id`.
        other: Echo,
        /// The `id` they share.
        id: u64,
    } => "table_id_duplicate", Error,
        message: format!("the table {name} has the same id {id} as the table {other}"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.5.1.2")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A cell of `t="s"` refers to a shared string the shared strings table
    /// does not hold: its index is past the table's end, or the workbook
    /// has no table. Reading a sheet's rows stops at the cell; verify's
    /// sheet checks go on.
    SharedStringMissing {
        /// The index the cell's value gives.
        index: u64,
    } => "shared_string_missing", Error,
        message: format!("a cell refers to shared string {index}, which the shared strings table does not hold"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.3.1.4, §18.4.8")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// The text of one row of a sheet, its strings and error values and
    /// the value being read as UTF-8 together, is larger than the limit.
    /// Reading the sheet stops at the row.
    RowTooLarge {
        /// The row's number.
        row: u64,
        /// The bytes of text counted when reading stopped: past the limit,
        /// so the row holds at least this many.
        size: u64,
        /// The most bytes allowed.
        limit: Limit,
    } => "row_too_large", Error,
        message: format!("row {row} holds at least {size} bytes of text; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxRowBytes", *limit)),
        fix: Some(Fix::policy(format!("maxRowBytes: {size}")));

    /// A row's or a cell's place in the sheet is not one it may have: a row
    /// number of 0 or past 1,048,576, a cell's `r` that is not the A1
    /// reference of one cell up to XFD1048576 or names another row than its
    /// row's, or a row or cell without `r` that would come after the last
    /// row or column. Reading a sheet's rows stops there; verify's sheet
    /// checks go on, the row or cell left out.
    CellReferenceInvalid {
        /// The row number or the cell reference as written; a cell's that
        /// names another row as it reads, its letters in capitals and its
        /// row without leading zeros; without `r`, as it would be.
        reference: Echo,
    } => "cell_reference_invalid", Error,
        message: format!("{reference} is not a place in the sheet a row or cell of it may have"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.3.1.4, §18.17.5.1; MS-OI29500 on §18.3.1.73, note a")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A row comes at or before the row before it, or a cell at or before
    /// the cell before it in its row: readers would differ on what the
    /// sheet holds there. Reading a sheet's rows stops there; verify's
    /// sheet checks go on, the row or cell left out.
    CellOrderInvalid {
        /// The row number or the cell reference.
        reference: Echo,
        /// The row or cell before it.
        previous: Echo,
    } => "cell_order_invalid", Error,
        message: format!("{reference} comes after {previous}; rows go down the sheet and a row's cells go across it"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.3.1.4, §18.3.1.73; MS-OI29500 on §18.3.1.73, note a")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    // What Excel repairs in a worksheet as it opens it, found by verify's
    // sheet checks, which raise `shared_string_missing`,
    // `cell_reference_invalid` and `cell_order_invalid` too.

    /// A cell's `s` is a cell format at or past the number of the styles
    /// part's `cellXfs`: Excel repairs the workbook, dropping the cell's
    /// format.
    CellStyleMissing {
        /// The cell's A1 reference; when it has no place in the sheet, as
        /// `cell_reference_invalid` and `cell_order_invalid` name it, or, in
        /// a row that has none, its `r` as written, empty without one.
        reference: Echo,
        /// The cell format it names.
        index: u64,
        /// How many cell formats the styles part has.
        count: u64,
    } => "cell_style_missing", Error,
        message: format!("cell {reference} uses cell format {index}, but the styles part has {count}"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.3.1.4, §18.8.10")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A column's `style` is a cell format at or past the number of the
    /// styles part's `cellXfs`: Excel repairs the workbook, dropping the
    /// columns' format.
    ColumnStyleMissing {
        /// The columns, `C:E`, each in letters when it is one, else as
        /// written.
        columns: Echo,
        /// The cell format they name.
        index: u64,
        /// How many cell formats the styles part has.
        count: u64,
    } => "column_style_missing", Error,
        message: format!("columns {columns} use cell format {index}, but the styles part has {count}"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.3.1.13, §18.8.10")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// A row's `s` is a cell format at or past the number of the styles
    /// part's `cellXfs`: Excel drops the row's format without a word.
    RowStyleMissing {
        /// The row's number; when it has no place in the sheet, its `r` as
        /// written, or as it would be.
        row: Echo,
        /// The cell format it names.
        index: u64,
        /// How many cell formats the styles part has.
        count: u64,
    } => "row_style_missing", Notice,
        message: format!("row {row} uses cell format {index}, but the styles part has {count}, so Excel drops the row's format"),
        decided_by: Some(DecidedBy::standard("ECMA-376-1:2016 §18.3.1.73, §18.8.10")),
        fix: Some(Fix::workbook("save the workbook again from Excel"));

    /// Two merged ranges of a sheet overlap, or are the same range: Excel
    /// repairs the workbook, dropping one.
    MergedCellsOverlap {
        /// The range that comes later in the sheet, its top left corner
        /// first, whichever way round it is written.
        reference: Echo,
        /// The range it overlaps, written the same way.
        other: Echo,
    } => "merged_cells_overlap", Error,
        message: format!("the merged cells {reference} overlap {other}"),
        decided_by: Some(DecidedBy::Default {}),
        fix: Some(Fix::workbook("unmerge one of the two ranges"));

    /// A sheet merges more ranges than the limit: those past it are not
    /// checked, so the sheet is refused.
    MergedCellsTooMany {
        /// How many ranges it merges.
        count: u64,
        /// The most merged ranges checked.
        limit: Limit,
    } => "merged_cells_too_many", Error,
        message: format!("the sheet merges {count} ranges; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxMergedCells", *limit)),
        fix: Some(Fix::policy(format!("maxMergedCells: {count}")));

    /// A cell's inline string is longer than the 32,767 UTF-16 code units
    /// Excel keeps in a cell: Excel repairs the workbook, cutting it.
    InlineStringTooLong {
        /// The cell's A1 reference; when it has no place in the sheet, as
        /// `cell_reference_invalid` and `cell_order_invalid` name it, or, in
        /// a row that has none, its `r` as written, empty without one.
        reference: Echo,
        /// Its length in UTF-16 code units.
        length: u64,
    } => "inline_string_too_long", Error,
        message: format!("cell {reference} holds an inline string of {length} UTF-16 code units; Excel keeps 32767 and cuts the rest"),
        decided_by: Some(DecidedBy::Default {}),
        fix: Some(Fix::workbook("shorten the cell's text to 32767 UTF-16 code units"));

    // The end of what verify's sheet checks raise.

    /// A part holds content of a kind the engine denies by default, and the
    /// template does not allow: the part itself (a VBA project, an external
    /// link, an OLE object), or, at a relationship, a link it makes outside
    /// the package; the package's own relationships count as the package's. One diagnostic per part and kind, however many times
    /// the part shows it.
    PartDenied {
        /// The kind of content.
        kind: PartKind,
    } => "part_denied", Error,
        message: format!("the part, or a relationship it declares, holds {kind} content, which is denied unless the template allows it"),
        decided_by: Some(DecidedBy::Default {}),
        fix: Some(Fix::template("parts.allow", kind.to_string()));

    /// A part holds content of a kind the host's policy forbids, whether or
    /// not the template allows it. One diagnostic per part and kind, as
    /// for `part_denied`.
    PartForbidden {
        /// The kind of content.
        kind: PartKind,
        /// Why the policy forbids it, in the policy's own words.
        reason: Echo,
    } => "part_forbidden", Error,
        message: format!("the part, or a relationship it declares, holds {kind} content, which the host's policy forbids: {reason}"),
        decided_by: Some(DecidedBy::Policy {}),
        fix: None;

    /// A formula could not be read, so it is refused whole: past where the
    /// formula reader stopped, what the formula states is a guess. The
    /// formula codes from here on are one diagnostic per part, defined name
    /// reached through (or none) and finding, located at the first place it
    /// was found, counting every time the part's formulas show it, or, for
    /// one found through a defined name, every time that name's formula
    /// does.
    FormulaMalformed {
        /// Where the formula reader stopped trusting it, first.
        reason: FormulaMalformedReason,
        /// How many times it was found.
        count: u64,
    } => "formula_malformed", Error,
        message: format!("a formula could not be read ({reason}), so it is refused: past that, what it states is a guess; {count} time(s), the first here"),
        decided_by: Some(match reason {
            FormulaMalformedReason::UndecodableEscape => DecidedBy::standard("ECMA-376-1:2016 §22.9.2.19"),
            FormulaMalformedReason::UpperCaseEscape
            | FormulaMalformedReason::BracketsTooDeep
            | FormulaMalformedReason::Markup => DecidedBy::Default {},
            _ => DecidedBy::standard("ECMA-376-1:2016 §18.17.2"),
        }),
        fix: Some(match reason {
            FormulaMalformedReason::BracketsTooDeep => Fix::workbook("nest the formula's brackets less deeply"),
            _ => Fix::workbook("save the workbook again from Excel"),
        });

    /// A formula is longer than the limit, so its text was not kept: it is
    /// refused unread.
    FormulaTooLong {
        /// The size in bytes, as UTF-8, of the longest found.
        size: u64,
        /// The most bytes of one formula kept.
        limit: Limit,
        /// How many times it was found.
        count: u64,
    } => "formula_too_long", Error,
        message: format!("a formula is longer than the limit, so it is refused unread: {size} bytes at the longest, the limit {limit}; {count} time(s), the first here"),
        decided_by: Some(DecidedBy::limit("maxFormulaBytes", *limit)),
        fix: Some(Fix::policy(format!("maxFormulaBytes: {size}")));

    /// The workbook part defines more names than the limit: those past it
    /// are no names a formula is judged by, so the workbook is refused.
    DefinedNamesTooMany {
        /// How many it defines.
        count: u64,
        /// The most defined names read.
        limit: Limit,
    } => "defined_names_too_many", Error,
        message: format!("the workbook part defines {count} names; the limit is {limit}"),
        decided_by: Some(DecidedBy::limit("maxDefinedNames", *limit)),
        fix: Some(Fix::policy(format!("maxDefinedNames: {count}")));

    /// A formula holds content of a kind the engine denies by default, and
    /// the template does not allow: a DDE link (`dde`) or a reference to
    /// another workbook (`external_link`).
    FormulaDenied {
        /// The kind of content.
        kind: PartKind,
        /// How many times it was found.
        count: u64,
    } => "formula_denied", Error,
        message: format!("a formula holds {kind} content, which is denied unless the template allows it; {count} time(s), the first here"),
        decided_by: Some(DecidedBy::Default {}),
        fix: Some(Fix::template("parts.allow", kind.to_string()));

    /// A formula holds content of a kind the host's policy forbids, whether
    /// or not the template allows it.
    FormulaForbidden {
        /// The kind of content.
        kind: PartKind,
        /// Why the policy forbids it, in the policy's own words.
        reason: Echo,
        /// How many times it was found.
        count: u64,
    } => "formula_forbidden", Error,
        message: format!("a formula holds {kind} content, which the host's policy forbids: {reason}; {count} time(s), the first here"),
        decided_by: Some(DecidedBy::Policy {}),
        fix: None;

    /// A formula calls, or may call, a function the engine's table does not
    /// mark reviewed safe and has not found dangerous, and the template
    /// does not allow.
    FunctionDenied {
        /// The function, upper-cased, `_XLUDF.` before a user-defined one.
        name: Echo,
        /// Why the engine denies it.
        denial: FunctionDenial,
        /// How many times it was found.
        count: u64,
    } => "function_denied", Error,
        message: format!("a formula calls, or may call, {name}, which the engine {}, so it is denied unless the template allows it; {count} time(s), the first here", match denial {
            FunctionDenial::Unreviewed => "has not reviewed yet",
            FunctionDenial::Unknown => "does not know",
        }),
        decided_by: Some(DecidedBy::Default {}),
        fix: Some(Fix::template("functions.allow", name.to_string()));

    /// A formula calls, or may call, a function the engine's table finds
    /// dangerous, and the template does not allow.
    FunctionDangerous {
        /// The function, upper-cased.
        name: Echo,
        /// Why the engine finds it dangerous.
        reason: Echo,
        /// How many times it was found.
        count: u64,
    } => "function_dangerous", Error,
        message: format!("a formula calls, or may call, {name}, which the engine finds dangerous ({reason}), so it is denied unless the template allows it; {count} time(s), the first here"),
        decided_by: Some(DecidedBy::Default {}),
        fix: Some(Fix::template("functions.allow", name.to_string()));

    /// A formula calls, or may call, a function the host's policy forbids,
    /// whether or not the template allows it.
    FunctionForbidden {
        /// The function, upper-cased.
        name: Echo,
        /// Why the policy forbids it, in the policy's own words.
        reason: Echo,
        /// How many times it was found.
        count: u64,
    } => "function_forbidden", Error,
        message: format!("a formula calls, or may call, {name}, which the host's policy forbids: {reason}; {count} time(s), the first here"),
        decided_by: Some(DecidedBy::Policy {}),
        fix: None;

    /// A template's `targets:` or `bestEffort:` entry names no versions the
    /// engine can check the workbook against, or, in `bestEffort:`, versions
    /// `targets:` names too. The engine reads every entry
    /// and reports each one that fails.
    TargetInvalid {
        /// The list the entry is in.
        list: TargetList,
        /// The entry's position in its list, the first being 0.
        index: u64,
        /// The entry as written.
        entry: Echo,
        /// Why it names no versions.
        reason: TargetReason,
        /// What the message lists, comma-separated: every application name
        /// for `unknown_app`; the versions of the application the engine
        /// knows for `unknown_version`, `version_required` and `empty`; the
        /// entry as it may be written for `version_not_allowed`; the
        /// versions both lists hold for `overlap`, or the application's
        /// name when it has no versions; empty for `malformed`. Always
        /// present, and clipped as every echo is.
        names: Echo,
    } => "target_invalid", Error,
        message: format!("{}[{index}] `{entry}` {}", match list {
            TargetList::Targets => "targets",
            TargetList::BestEffort => "bestEffort",
        }, match reason {
            TargetReason::Malformed => "is not written as `name`, `name version` or `name OP version`, OP one of >=, >, <=, <".to_owned(),
            TargetReason::UnknownApp => format!("names an application the engine does not know; it knows {names}"),
            TargetReason::UnknownVersion => format!("names a version the engine does not know; the versions it knows are {names}"),
            TargetReason::VersionRequired => format!("names no version, which the application needs; the versions the engine knows are {names}"),
            TargetReason::VersionNotAllowed => format!("gives a version to an application that has none; write it as {names}"),
            TargetReason::Empty => format!("holds none of the versions the engine knows, which are {names}"),
            TargetReason::Overlap => format!("holds {names}, which targets holds as well; a version is either required or best effort"),
        }),
        decided_by: Some(DecidedBy::Template {}),
        fix: None;

    /// A formula calls a function that a version the template's `targets:`
    /// holds, where the workbook must work, lacks, or that no source the
    /// compatibility table cites says it has, a feature it needs counted
    /// with it. One per part, defined name reached through, function,
    /// application and list.
    TargetFunctionUnsupported {
        /// The function, upper-cased, without its prefixes (`_xlfn.`,
        /// `_xlws.`, `_xleta.`).
        name: Echo,
        /// The application.
        app: TargetApp,
        /// The versions of it the list holds that lack the function, or a
        /// feature in `requires`, oldest first, comma-separated (`2016,
        /// 2019`); empty when none does, and then `unverified` is not.
        /// Google Sheets' only version is `current`.
        unsupported: Echo,
        /// The versions of it the list holds that no source the table
        /// cites says have the function, or a feature in `requires`, and
        /// none says lack it, as `unsupported`; empty when none.
        unverified: Echo,
        /// The features the function needs as well (`dynamic_array`) that
        /// some of those versions lack where the function itself is there
        /// or not confirmed, or are not confirmed to have where it is
        /// there, comma-separated; empty when none.
        requires: Echo,
        /// What a version that lacks the function shows instead: the
        /// function's own whenever the function itself is not there, or
        /// not confirmed, in some of those versions; the first a source
        /// gives for the features in `requires` only when the function
        /// itself is there in all of them; `unknown` when no source says.
        fallback: TargetFallback,
        /// How many times it was found.
        count: u64,
    } => "target_function_unsupported", Error,
        message: crate::target::Unmet {
            what: format!("a formula calls {name}"),
            list: "targets",
            app: *app,
            unsupported,
            unverified,
            requires,
            fallback: *fallback,
            count: *count,
        }
        .message(),
        decided_by: Some(DecidedBy::Template {}),
        fix: Some(Fix::workbook(format!("call a function other than {name} so the workbook works in {}", crate::target::spelled(*app))));

    /// A formula calls a function that a version the template's `bestEffort:`
    /// holds, where the workbook should work, lacks, or that no source the
    /// compatibility table cites says it has, a feature it needs counted
    /// with it. One per part, defined name reached through, function,
    /// application and list.
    BestEffortFunctionUnsupported {
        /// The function, upper-cased, without its prefixes (`_xlfn.`,
        /// `_xlws.`, `_xleta.`).
        name: Echo,
        /// The application.
        app: TargetApp,
        /// The versions of it the list holds that lack the function, or a
        /// feature in `requires`, oldest first, comma-separated (`2016,
        /// 2019`); empty when none does, and then `unverified` is not.
        /// Google Sheets' only version is `current`.
        unsupported: Echo,
        /// The versions of it the list holds that no source the table
        /// cites says have the function, or a feature in `requires`, and
        /// none says lack it, as `unsupported`; empty when none.
        unverified: Echo,
        /// The features the function needs as well (`dynamic_array`) that
        /// some of those versions lack where the function itself is there
        /// or not confirmed, or are not confirmed to have where it is
        /// there, comma-separated; empty when none.
        requires: Echo,
        /// What a version that lacks the function shows instead: the
        /// function's own whenever the function itself is not there, or
        /// not confirmed, in some of those versions; the first a source
        /// gives for the features in `requires` only when the function
        /// itself is there in all of them; `unknown` when no source says.
        fallback: TargetFallback,
        /// How many times it was found.
        count: u64,
    } => "best_effort_function_unsupported", Warn,
        message: crate::target::Unmet {
            what: format!("a formula calls {name}"),
            list: "bestEffort",
            app: *app,
            unsupported,
            unverified,
            requires,
            fallback: *fallback,
            count: *count,
        }
        .message(),
        decided_by: Some(DecidedBy::Template {}),
        fix: Some(Fix::workbook(format!("call a function other than {name} so the workbook works in {}", crate::target::spelled(*app))));
}

#[cfg(test)]
mod tests;
