//! What every reader of a SpreadsheetML part checks its shape with: the
//! namespaces of SpreadsheetML ([`NAMESPACES`]), of an `r:id`
//! ([`RELATIONSHIPS`]) and the ones a part with both is read in
//! ([`UNDERSTOOD`]), the one a name is bound to ([`namespace`]), the white
//! space that may stand between elements ([`is_blank`]) and that a value's
//! type collapses ([`is_space`]), an `xsd:boolean` ([`boolean`]), the text
//! a reference stands for ([`resolve`]), a sheet name as names are compared
//! ([`fold`]), and the `xml_schema_invalid` it refuses with ([`refuse`]).

use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};
use xlsx_graft_xml::events::BytesRef;
use xlsx_graft_xml::name::{Namespace, ResolveResult};

/// The namespaces of SpreadsheetML, Transitional and Strict.
pub(crate) const NAMESPACES: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// The namespace of `r:id`, Transitional and Strict, in the order of
/// [`NAMESPACES`]: an `r:id` is in the one of the root's kind.
pub(crate) const RELATIONSHIPS: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];

/// The namespaces a part that has `r:id`s is read in: named ignorable,
/// they are still read.
pub(crate) const UNDERSTOOD: [&str; 4] = [
    NAMESPACES[0],
    NAMESPACES[1],
    RELATIONSHIPS[0],
    RELATIONSHIPS[1],
];

/// The predefined entities (XML 1.0 §4.6), the only ones the XML reader
/// admits.
const ENTITIES: [(&str, char); 5] = [
    ("lt", '<'),
    ("gt", '>'),
    ("amp", '&'),
    ("apos", '\''),
    ("quot", '"'),
];

/// Whether `text` is all XML white space.
pub(crate) fn is_blank(text: &str) -> bool {
    text.bytes()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

/// Whether `c` is XML white space, which a type such as `xsd:unsignedInt`
/// or `xsd:boolean` collapses.
pub(crate) fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// The `xsd:boolean` `value` is, its white space collapsed.
pub(crate) fn boolean(value: &str) -> Option<bool> {
    match value.trim_matches(is_space) {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

/// The text a reference the XML reader admitted stands for.
pub(crate) fn resolve(reference: &BytesRef<'_>) -> String {
    let named = ENTITIES.iter().find(|(name, _)| *name == &**reference);
    let c = reference.resolve_char_ref().ok().flatten();
    c.or(named.map(|&(_, c)| c))
        .map(String::from)
        .unwrap_or_default()
}

/// `name` as sheet names are compared: lowercased as Unicode has it, the
/// same in every locale (the workbook part's `link.rs` says why).
pub(crate) fn fold(name: &str) -> String {
    name.to_lowercase()
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
