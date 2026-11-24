//! The namespaces, relationship types and media types `apply` reads and
//! writes, Transitional and Strict (ECMA-376-1:2016 §8.1, ECMA-376-4:2016
//! §8.1). A flavor is an index into each pair: 0 Transitional, 1 Strict.

/// SpreadsheetML.
pub(super) const MAIN: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// The namespace of `r:id`, and the start of the relationship types.
pub(super) const RELATIONSHIPS: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];

/// The relationship type of the app part, from the package.
pub(super) const APP_TYPES: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties",
    "http://purl.oclc.org/ooxml/officeDocument/relationships/extendedProperties",
];

/// The namespace of the app part's elements.
pub(super) const EXTENDED: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties",
    "http://purl.oclc.org/ooxml/officeDocument/extendedProperties",
];

/// The media type of a worksheet part.
pub(super) const WORKSHEET_MEDIA: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// The flavor of a workbook: 1 for Strict.
pub(super) fn flavor(strict: bool) -> usize {
    usize::from(strict)
}
