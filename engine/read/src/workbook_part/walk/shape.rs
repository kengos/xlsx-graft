//! The schema shape of the workbook part: the elements read ([`Frame`])
//! and where each may stand ([`child`]), and the attributes each may have
//! ([`attributes`]), with the namespaces `mc:Ignorable` names.
//!
//! Each frame knows how far into its schema sequence it is (its stage), so
//! a child is admitted only in the schema's order and as often as it may
//! come, as in the styles part. The Transitional schema has the Strict
//! one's elements and attributes, and `dateCompatibility` on `workbookPr`
//! besides, which ISO/IEC 29500-1:2008 §18.2.28 has and ECMA-376-1:2016
//! has not. It is taken whatever the root's kind and not read: Excel
//! ignores it in either (MS-OI29500 on §18.2.28, note j).

use crate::compat::Ignorable;
use crate::schema::{NAMESPACES, is_space, namespace, refuse};
use crate::shared_strings::escape::unescape;
use crate::workbook_part::SheetState;
use std::convert::Infallible;
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};
use xlsx_graft_xml::XmlReader;
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::name::ResolveResult;

/// The namespace of `r:id`, Transitional and Strict, in the order of
/// [`NAMESPACES`]: an `r:id` is in the one of the root's kind.
const RELATIONSHIPS: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];

/// The namespaces the part is read in: named ignorable, they are still
/// read.
pub(super) const UNDERSTOOD: [&str; 4] = [
    NAMESPACES[0],
    NAMESPACES[1],
    RELATIONSHIPS[0],
    RELATIONSHIPS[1],
];

/// The stage of `workbook` once `sheets` is read.
pub(super) const SHEETS: u8 = 6;

/// An element being read, and its stage in its schema sequence.
#[derive(Debug, Clone, Copy)]
pub(super) enum Frame {
    /// `workbook` (CT_Workbook): `fileVersion?`, `fileSharing?`,
    /// `workbookPr?`, `workbookProtection?`, `bookViews?`, `sheets`,
    /// `functionGroups?`, `externalReferences?`, `definedNames?`,
    /// `calcPr?`, `oleSize?`, `customWorkbookViews?`, `pivotCaches?`,
    /// `smartTagPr?`, `smartTagTypes?`, `webPublishing?`,
    /// `fileRecoveryPr*`, `webPublishObjects?`, then `extLst?`; `offset`
    /// is where it starts.
    Workbook { stage: u8, offset: u64 },
    /// `workbookPr` (CT_WorkbookPr): nothing.
    WorkbookPr,
    /// `sheets` (CT_Sheets): `sheet+`; `offset` is where it starts.
    Sheets { stage: u8, offset: u64 },
    /// `sheet` (CT_Sheet): nothing.
    Sheet,
}

/// What a child element is to its parent.
pub(super) enum Child {
    /// An element to read.
    Read(Frame),
    /// An element to skip, with all it holds.
    Skip,
}

/// What the attributes of an element read give.
#[derive(Debug)]
pub(super) struct Values {
    /// `name`, of a `sheet`, its escapes undone.
    pub(super) name: Option<String>,
    /// `sheetId`, of a `sheet`.
    pub(super) sheet_id: Option<u32>,
    /// `state`, of a `sheet`.
    pub(super) state: SheetState,
    /// `r:id`, of a `sheet`.
    pub(super) relationship_id: Option<String>,
    /// `date1904`, of `workbookPr`.
    pub(super) date1904: bool,
    /// `conformance` is `strict`, of `workbook`.
    pub(super) strict: bool,
}

/// What the element `local` at `offset` is as a child of `parent`, moving
/// `parent` on to its stage; `None` when it may not stand there.
pub(super) fn child(parent: &mut Frame, local: &str, offset: u64) -> Option<Child> {
    let (at, stage, repeats, child) = match (parent, local) {
        (Frame::Workbook { stage: at, .. }, _) => {
            let order = [
                "fileVersion",
                "fileSharing",
                "workbookPr",
                "workbookProtection",
                "bookViews",
                "sheets",
                "functionGroups",
                "externalReferences",
                "definedNames",
                "calcPr",
                "oleSize",
                "customWorkbookViews",
                "pivotCaches",
                "smartTagPr",
                "smartTagTypes",
                "webPublishing",
                "fileRecoveryPr",
                "webPublishObjects",
                "extLst",
            ];
            let stage = order.iter().position(|&name| name == local)?;
            let child = match local {
                "workbookPr" => Child::Read(Frame::WorkbookPr),
                "sheets" => Child::Read(Frame::Sheets { stage: 0, offset }),
                _ => Child::Skip,
            };
            let repeats = local == "fileRecoveryPr";
            (at, u8::try_from(stage + 1).ok()?, repeats, child)
        }
        (Frame::Sheets { stage: at, .. }, "sheet") => (at, 1, true, Child::Read(Frame::Sheet)),
        _ => return None,
    };
    let admitted = stage > *at || (stage == *at && repeats);
    *at = stage;
    admitted.then_some(child)
}

/// Reads the attributes of `start`, an element read as `frame` at
/// `offset` in a part whose root is in `NAMESPACES[flavor]`, entered in
/// `ignorable`: names in it what its `mc:Ignorable` names, passes over
/// those in them, and refuses one the schema does not give it, a value it
/// reads that its type does not allow, or a required one left out.
pub(super) fn attributes(
    frame: &Frame,
    flavor: usize,
    start: &BytesStart<'_>,
    reader: &XmlReader<'_>,
    ignorable: &mut Ignorable,
    offset: u64,
) -> Result<Values, Diagnostic> {
    let allowed: &[&str] = match frame {
        Frame::Workbook { .. } => &["conformance"],
        Frame::Sheets { .. } => &[],
        Frame::Sheet => &["name", "sheetId", "state"],
        Frame::WorkbookPr => &[
            "date1904",
            "dateCompatibility",
            "showObjects",
            "showBorderUnselectedTables",
            "filterPrivacy",
            "promptedSolutions",
            "showInkAnnotation",
            "backupFile",
            "saveExternalLinkValues",
            "updateLinks",
            "codeName",
            "hidePivotFieldList",
            "showPivotChartFilter",
            "allowRefreshQuery",
            "publishItems",
            "checkCompatibility",
            "autoCompressPictures",
            "refreshAllConnections",
            "defaultThemeVersion",
        ],
    };
    ignorable.take(start, reader, offset)?;
    let mut values = Values {
        name: None,
        sheet_id: None,
        state: SheetState::Visible,
        relationship_id: None,
        date1904: false,
        strict: false,
    };
    let relationships = RELATIONSHIPS.get(flavor).copied();
    // Namespace declarations are not attributes.
    let attributes = (start.attributes().flatten()).filter(|attribute| {
        let key = attribute.key.as_ref();
        key != "xmlns" && !key.starts_with("xmlns:")
    });
    for attribute in attributes {
        let (resolved, local) = reader.resolve_attribute(attribute.key);
        let name = attribute.key.as_ref();
        let value = attribute.normalized_value(XmlVersion::Implicit1_0);
        let value = value.unwrap_or_default();
        let invalid = || refuse(offset, SchemaReason::InvalidValue, name);
        let unbound = matches!(resolved, ResolveResult::Unbound) && allowed.contains(&name);
        match local.as_ref() {
            "conformance" if unbound => match value.as_ref() {
                "strict" => values.strict = true,
                "transitional" => {}
                _ => return Err(invalid()),
            },
            "date1904" if unbound => values.date1904 = boolean(&value).ok_or_else(invalid)?,
            "name" if unbound => {
                let mut sheet = String::new();
                let Ok(_) = unescape(&value, true, |piece| {
                    sheet.push_str(piece);
                    Ok::<(), Infallible>(())
                });
                values.name = Some(sheet);
            }
            "sheetId" if unbound => {
                let id = value.trim_matches(is_space).parse::<u32>();
                values.sheet_id = Some(id.map_err(|_| invalid())?);
            }
            "state" if unbound => {
                values.state = match value.as_ref() {
                    "visible" => SheetState::Visible,
                    "hidden" => SheetState::Hidden,
                    "veryHidden" => SheetState::VeryHidden,
                    _ => return Err(invalid()),
                };
            }
            _ if unbound => {}
            "id" if matches!(frame, Frame::Sheet) && namespace(&resolved) == relationships => {
                values.relationship_id = Some(value.into_owned());
            }
            local if ignorable.passes_over(namespace(&resolved), local) => {}
            _ => return Err(refuse(offset, SchemaReason::UnexpectedAttribute, name)),
        }
    }
    let required = match frame {
        Frame::Sheet if values.name.is_none() => Some("name"),
        Frame::Sheet if values.sheet_id.is_none() => Some("sheetId"),
        Frame::Sheet if values.relationship_id.is_none() => Some("r:id"),
        _ => None,
    };
    match required {
        Some(name) => Err(refuse(offset, SchemaReason::MissingAttribute, name)),
        None => Ok(values),
    }
}

/// The `xsd:boolean` `value` is, its white space collapsed.
fn boolean(value: &str) -> Option<bool> {
    match value.trim_matches(is_space) {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}
