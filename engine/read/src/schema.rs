//! What every reader of a SpreadsheetML part checks its shape with: the
//! namespaces of SpreadsheetML ([`NAMESPACES`]) and the one a name is bound
//! to ([`namespace`]), the white space that may stand between elements
//! ([`is_blank`]), and the `xml_schema_invalid` it refuses with
//! ([`refuse`]).

use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};
use xlsx_graft_xml::name::{Namespace, ResolveResult};

/// The namespaces of SpreadsheetML, Transitional and Strict.
pub(crate) const NAMESPACES: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// Whether `text` is all XML white space.
pub(crate) fn is_blank(text: &str) -> bool {
    text.bytes()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

/// The namespace `resolved` binds, if it binds one.
pub(crate) fn namespace<'a>(resolved: &ResolveResult<'a>) -> Option<&'a str> {
    match *resolved {
        ResolveResult::Bound(Namespace(ns)) => Some(ns),
        _ => None,
    }
}

/// `xml_schema_invalid` at `offset`, for `reason`, naming `name`.
pub(crate) fn refuse(offset: u64, reason: SchemaReason, name: &str) -> Diagnostic {
    Diagnostic::XmlSchemaInvalid {
        offset,
        reason,
        name: Echo::new(name),
    }
}
