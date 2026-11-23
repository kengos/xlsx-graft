//! The attributes of the elements [`super::read`] reads: `sheet`,
//! `definedName` and `workbookView`, and the small helpers they and the walk
//! share.
//!
//! An attribute is the one the model holds only when it has no prefix
//! (`name`, not `x:name`), except a sheet's `r:id`, which is in the
//! relationships namespace of the root's kind: Transitional
//! `…/officeDocument/2006/relationships`, Strict
//! `http://purl.oclc.org/ooxml/officeDocument/relationships`
//! (ECMA-376-1:2016 §18.2.19). Values are as the XML reader normalizes
//! them. Namespace declarations are not attributes.

use super::super::{BookView, DefinedName};
use super::{Listed, SheetState};
use std::convert::Infallible;
use xlsx_graft_cell::unescape;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::events::attributes::Attribute;
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlReader, XmlVersion};

/// The namespace of `r:id`, Transitional and Strict, in the order of
/// [`super::NAMESPACES`]: an `r:id` is in the one of the root's kind.
const RELATIONSHIPS: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];

/// The namespace `resolved` binds, if it binds one.
pub(super) fn namespace<'a>(resolved: &ResolveResult<'a>) -> Option<&'a str> {
    match *resolved {
        ResolveResult::Bound(Namespace(ns)) => Some(ns),
        _ => None,
    }
}

/// `xml_schema_invalid` at `offset`, for `reason`, naming `name`.
pub(super) fn refuse(offset: u64, reason: SchemaReason, name: &str) -> Diagnostic {
    Diagnostic::XmlSchemaInvalid {
        offset,
        reason,
        name: Echo::new(name),
    }
}

/// Whether `c` is XML white space, which a type such as
/// `xsd:unsignedInt` collapses.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// The attributes of `start` that are not namespace declarations, each with
/// its name as written and its normalized value.
fn attributes<'s>(
    start: &'s BytesStart<'_>,
) -> impl Iterator<Item = (Attribute<'s>, String, String)> {
    (start.attributes().flatten())
        .filter(|attribute| {
            let key = attribute.key.as_ref();
            key != "xmlns" && !key.starts_with("xmlns:")
        })
        .map(|attribute| {
            let key = attribute.key.as_ref().to_owned();
            let value = attribute.normalized_value(XmlVersion::Implicit1_0);
            let value = value.unwrap_or_default().into_owned();
            (attribute, key, value)
        })
}

/// The `xsd:unsignedInt` `value` is, its white space collapsed, else
/// `xml_schema_invalid` at `offset` naming the attribute `key`.
fn unsigned(key: &str, value: &str, offset: u64) -> Result<u32, Diagnostic> {
    let parsed = value.trim_matches(is_space).parse::<u32>();
    parsed.map_err(|_| refuse(offset, SchemaReason::InvalidValue, key))
}

/// The `sheet` `start`, at `offset`, in a root of `NAMESPACES[flavor]`:
/// `name`, `sheetId` and `r:id` are required, in that order, `state` is
/// `visible` when not written.
pub(super) fn sheet(
    start: &BytesStart<'_>,
    reader: &XmlReader<'_>,
    flavor: usize,
    offset: u64,
) -> Result<Listed, Diagnostic> {
    let (mut name, mut sheet_id, mut relationship_id) = (None, None, None);
    let mut state = SheetState::Visible;
    let relationships = RELATIONSHIPS.get(flavor).copied();
    for (attribute, key, value) in attributes(start) {
        match key.as_str() {
            "name" => {
                let mut unescaped = String::new();
                let Ok(_) = unescape(&value, true, |piece| {
                    unescaped.push_str(piece);
                    Ok::<(), Infallible>(())
                });
                name = Some(unescaped);
            }
            "sheetId" => sheet_id = Some(unsigned(&key, &value, offset)?),
            "state" => {
                state = match value.as_str() {
                    "visible" => SheetState::Visible,
                    "hidden" => SheetState::Hidden,
                    "veryHidden" => SheetState::VeryHidden,
                    _ => return Err(refuse(offset, SchemaReason::InvalidValue, &key)),
                };
            }
            _ => {
                let (resolved, local) = reader.resolve_attribute(attribute.key);
                if local.as_ref() == "id" && namespace(&resolved) == relationships {
                    relationship_id = Some(value);
                }
            }
        }
    }
    let missing = |name: &str| refuse(offset, SchemaReason::MissingAttribute, name);
    Ok(Listed {
        name: name.ok_or_else(|| missing("name"))?,
        sheet_id: sheet_id.ok_or_else(|| missing("sheetId"))?,
        state,
        relationship_id: relationship_id.ok_or_else(|| missing("r:id"))?,
    })
}

/// The `definedName` `start`, at `offset`, with an empty text: every
/// attribute in the order written, and its `name`, which is required, and
/// `localSheetId`.
pub(super) fn defined_name(start: &BytesStart<'_>, offset: u64) -> Result<DefinedName, Diagnostic> {
    let (mut name, mut local_sheet_id) = (None, None);
    let mut written = Vec::new();
    for (_, key, value) in attributes(start) {
        match key.as_str() {
            "name" => name = Some(value.clone()),
            "localSheetId" => local_sheet_id = Some(unsigned(&key, &value, offset)?),
            _ => {}
        }
        written.push((key, value));
    }
    let missing = refuse(offset, SchemaReason::MissingAttribute, "name");
    Ok(DefinedName {
        attributes: written,
        name: name.ok_or(missing)?,
        local_sheet_id,
        text: String::new(),
    })
}

/// The `workbookView` `start`, at `offset`.
pub(super) fn view(start: &BytesStart<'_>, offset: u64) -> Result<BookView, Diagnostic> {
    let (mut active_tab, mut first_sheet) = (None, None);
    for (_, key, value) in attributes(start) {
        match key.as_str() {
            "activeTab" => active_tab = Some(unsigned(&key, &value, offset)?),
            "firstSheet" => first_sheet = Some(unsigned(&key, &value, offset)?),
            _ => {}
        }
    }
    Ok(BookView {
        active_tab,
        first_sheet,
    })
}
