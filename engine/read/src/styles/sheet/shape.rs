//! The schema shape of the styles part: the elements read ([`Frame`]) and
//! where each may stand ([`child`]), and the attributes each may have
//! ([`attributes`]), with the namespaces `mc:Ignorable` names.
//!
//! Each frame knows how far into its schema sequence it is (its stage), so
//! a child is admitted only in the schema's order and as often as it may
//! come, as in the shared strings part.

use crate::compat::Ignorable;
use crate::schema::{namespace, refuse};
use crate::shared_strings::escape::unescape;
use std::convert::Infallible;
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};
use xlsx_graft_xml::XmlReader;
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::name::ResolveResult;

/// An element being read, and its stage in its schema sequence.
#[derive(Debug, Clone, Copy)]
pub(super) enum Frame {
    /// `styleSheet` (CT_Stylesheet): `numFmts?`, `fonts?`, `fills?`,
    /// `borders?`, `cellStyleXfs?`, `cellXfs?`, `cellStyles?`, `dxfs?`,
    /// `tableStyles?`, `colors?`, then `extLst?`.
    StyleSheet { stage: u8 },
    /// `numFmts` (CT_NumFmts): `numFmt*`.
    NumFmts,
    /// `numFmt` (CT_NumFmt): nothing.
    NumFmt,
    /// `cellXfs` (CT_CellXfs): `xf+`; `offset` is where it starts.
    CellXfs { stage: u8, offset: u64 },
    /// `xf` (CT_Xf): `alignment?`, `protection?`, then `extLst?`.
    Xf { stage: u8 },
}

/// What a child element is to its parent.
pub(super) enum Child {
    /// An element to read.
    Read(Frame),
    /// An element to skip, with all it holds.
    Skip,
}

/// What the attributes of an element read give.
#[derive(Debug, Default)]
pub(super) struct Values {
    /// `numFmtId`, of a `numFmt` or an `xf`.
    pub(super) id: Option<u32>,
    /// `formatCode`, of a `numFmt`, its escapes undone.
    pub(super) code: Option<String>,
}

/// What the element `local` at `offset` is as a child of `parent`, moving
/// `parent` on to its stage; `None` when it may not stand there.
pub(super) fn child(parent: &mut Frame, local: &str, offset: u64) -> Option<Child> {
    let (at, stage, repeats, child) = match (parent, local) {
        (Frame::StyleSheet { stage: at }, _) => {
            let order = [
                "numFmts",
                "fonts",
                "fills",
                "borders",
                "cellStyleXfs",
                "cellXfs",
                "cellStyles",
                "dxfs",
                "tableStyles",
                "colors",
                "extLst",
            ];
            let stage = order.iter().position(|&name| name == local)?;
            let child = match local {
                "numFmts" => Child::Read(Frame::NumFmts),
                "cellXfs" => Child::Read(Frame::CellXfs { stage: 0, offset }),
                _ => Child::Skip,
            };
            (at, u8::try_from(stage + 1).ok()?, false, child)
        }
        (Frame::NumFmts, "numFmt") => return Some(Child::Read(Frame::NumFmt)),
        (Frame::CellXfs { stage: at, .. }, "xf") => {
            (at, 1, true, Child::Read(Frame::Xf { stage: 0 }))
        }
        (Frame::Xf { stage: at }, "alignment") => (at, 1, false, Child::Skip),
        (Frame::Xf { stage: at }, "protection") => (at, 2, false, Child::Skip),
        (Frame::Xf { stage: at }, "extLst") => (at, 3, false, Child::Skip),
        _ => return None,
    };
    let admitted = stage > *at || (stage == *at && repeats);
    *at = stage;
    admitted.then_some(child)
}

/// Reads the attributes of `start`, an element read as `frame` at
/// `offset`, entered in `ignorable`: names in it what its `mc:Ignorable`
/// names, passes over those in them, and refuses one the schema does not
/// give it or a value it reads that its type does not allow.
pub(super) fn attributes(
    frame: &Frame,
    start: &BytesStart<'_>,
    reader: &XmlReader<'_>,
    ignorable: &mut Ignorable,
    offset: u64,
) -> Result<Values, Diagnostic> {
    let allowed: &[&str] = match frame {
        Frame::StyleSheet { .. } => &[],
        Frame::NumFmts | Frame::CellXfs { .. } => &["count"],
        Frame::NumFmt => &["numFmtId", "formatCode"],
        Frame::Xf { .. } => &[
            "numFmtId",
            "fontId",
            "fillId",
            "borderId",
            "xfId",
            "quotePrefix",
            "pivotButton",
            "applyNumberFormat",
            "applyFont",
            "applyFill",
            "applyBorder",
            "applyAlignment",
            "applyProtection",
        ],
    };
    ignorable.take(start, reader, offset)?;
    let mut values = Values::default();
    // Namespace declarations are not attributes.
    let attributes = (start.attributes().flatten()).filter(|attribute| {
        let key = attribute.key.as_ref();
        key != "xmlns" && !key.starts_with("xmlns:")
    });
    for attribute in attributes {
        let (resolved, local) = reader.resolve_attribute(attribute.key);
        let name = attribute.key.as_ref();
        let value = || {
            let value = attribute.normalized_value(XmlVersion::Implicit1_0);
            value.unwrap_or_default()
        };
        match (resolved, local.as_ref()) {
            (ResolveResult::Unbound, "numFmtId") if allowed.contains(&name) => {
                let id = value().trim_matches(is_space).parse::<u32>();
                let id = id.map_err(|_| refuse(offset, SchemaReason::InvalidValue, name))?;
                values.id = Some(id);
            }
            (ResolveResult::Unbound, "formatCode") if allowed.contains(&name) => {
                let mut code = String::new();
                let Ok(_) = unescape(&value(), true, |piece| {
                    code.push_str(piece);
                    Ok::<(), Infallible>(())
                });
                values.code = Some(code);
            }
            (ResolveResult::Unbound, _) if allowed.contains(&name) => {}
            (ref resolved, local) if ignorable.passes_over(namespace(resolved), local) => {}
            _ => return Err(refuse(offset, SchemaReason::UnexpectedAttribute, name)),
        }
    }
    let required = match frame {
        Frame::NumFmt if values.id.is_none() => Some("numFmtId"),
        Frame::NumFmt if values.code.is_none() => Some("formatCode"),
        _ => None,
    };
    match required {
        Some(name) => Err(refuse(offset, SchemaReason::MissingAttribute, name)),
        None => Ok(values),
    }
}

/// Whether `c` is XML white space, which `xsd:unsignedInt` collapses.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}
