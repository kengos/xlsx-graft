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
use crate::{DecidedBy, Echo, EncodingReason, Fix, Limit};

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
}

#[cfg(test)]
mod tests;
