//! The kind of content a media type, a relationship type or an element
//! names, and the hyperlinks that may point outside the package.
//!
//! Media types are matched by their lowered essence, relationship types
//! with ASCII letter case aside: a part that spells a type in another case
//! is taken for what the type names, which refuses more, never less.
//!
//! Sources:
//!
//! - ECMA-376-1:2016 §12.3.4 Connections, §12.3.7 Dialogsheet, §12.3.9
//!   External Workbook References, §12.3.14 Query Table, §15.2.9 Embedded
//!   Control Persistence (any media type; `application/vnd.ms-office.activeX+xml`
//!   its example), §15.2.10 Embedded Object and §15.2.11 Embedded Package
//!   (any media type), §15.3 Hyperlinks; §18.14.4 `ddeLink`, §18.14.11
//!   `oleLink`, §18.16.1 `DataBinding` (an XML map's source), §18.3.1.59
//!   `oleObject` (a worksheet's embedded object; the control persistence
//!   part's `ocx` root is §15.2.9's example),
//!   §18.3.1.97 `webPublishItem`, §18.2.25 `webPublishObject`.
//!   The relationship types are given there in the Strict namespace; the
//!   Transitional one ends in the same name.
//! - [MS-OFFMACRO2] §2.2.1.1–2.2.1.4: the macro sheets, the VBA project
//!   and the macro-enabled workbook.
//! - As Excel writes them, with no published part definition: an embedded
//!   object's `application/vnd.openxmlformats-officedocument.oleObject`,
//!   an ActiveX control's binary (`activeXControlBinary`,
//!   `application/vnd.ms-office.activeX`), a VBA project's signatures
//!   (named after `vbaProject`), a macro-enabled add-in's main part and a
//!   Power Query definition's `DataMashup` root. These are matched by
//!   prefix or by `macroenabled` anywhere in the media type, so a variant
//!   of them is refused too.

use xlsx_graft_diagnostics::PartKind;

/// The relationship types defined by ECMA-376, Transitional and Strict,
/// up to the name of the type.
const ECMA: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/",
    "http://purl.oclc.org/ooxml/officeDocument/relationships/",
];

/// The relationship types Microsoft defines, up to the name of the type.
/// The later VBA signatures are named in later namespaces; their part is
/// refused by its media type, and the VBA project they sign by its own
/// relationship, which is in this one.
const MICROSOFT: &str = "http://schemas.microsoft.com/office/2006/relationships/";

/// The kind of content a part of the media type `essence` holds.
pub(super) fn media_type(essence: &str) -> Option<PartKind> {
    let spreadsheet = essence.strip_prefix("application/vnd.openxmlformats-officedocument.");
    match spreadsheet {
        Some("spreadsheetml.dialogsheet+xml") => return Some(PartKind::Macro),
        Some("spreadsheetml.externallink+xml") => return Some(PartKind::ExternalLink),
        Some("spreadsheetml.connections+xml" | "spreadsheetml.querytable+xml") => {
            return Some(PartKind::DataConnection);
        }
        Some("oleobject") => return Some(PartKind::Ole),
        _ => {}
    }
    if essence.starts_with("application/vnd.ms-office.vbaproject")
        || essence.contains("macroenabled")
        || essence == "application/vnd.ms-excel.macrosheet+xml"
        || essence == "application/vnd.ms-excel.intlmacrosheet+xml"
    {
        return Some(PartKind::Macro);
    }
    (essence.starts_with("application/vnd.ms-office.activex")).then_some(PartKind::Activex)
}

/// The kind of content the target of a relationship of `rel_type` holds.
pub(super) fn relationship(rel_type: &str) -> Option<PartKind> {
    if let Some(name) = ECMA.iter().find_map(|prefix| after(rel_type, prefix)) {
        return match name.to_ascii_lowercase().as_str() {
            "dialogsheet" => Some(PartKind::Macro),
            "externallink" | "externallinkpath" => Some(PartKind::ExternalLink),
            "connections" | "querytable" => Some(PartKind::DataConnection),
            "control" => Some(PartKind::Activex),
            "oleobject" | "package" => Some(PartKind::Ole),
            _ => None,
        };
    }
    let name = after(rel_type, MICROSOFT)?.to_ascii_lowercase();
    if name.starts_with("vbaproject") || name == "xlmacrosheet" || name == "xlintlmacrosheet" {
        return Some(PartKind::Macro);
    }
    name.starts_with("activex").then_some(PartKind::Activex)
}

/// The kind of content an element of the local name `local` makes its
/// part hold, matched in its own letter case: every part is read as XML, a
/// VML drawing's `o:OLEObject` among them, which is no embedded object.
pub(super) fn element(local: &str) -> Option<PartKind> {
    match local {
        "ddeLink" => Some(PartKind::Dde),
        "oleLink" | "oleObject" => Some(PartKind::Ole),
        "ocx" => Some(PartKind::Activex),
        "DataMashup" | "DataBinding" => Some(PartKind::DataConnection),
        "webPublishItem" | "webPublishObject" => Some(PartKind::WebPublish),
        _ => None,
    }
}

/// Whether a relationship of `rel_type` to `target` outside the package is
/// a hyperlink to a Web page or a mail address, the one kind admitted. The
/// type must be written as ECMA-376 writes it, since this admits rather
/// than refuses.
pub(super) fn web_or_mail_link(rel_type: &str, target: &str) -> bool {
    let hyperlink = (ECMA.iter()).any(|prefix| rel_type.strip_prefix(prefix) == Some("hyperlink"));
    hyperlink
        && ["http://", "https://", "mailto:"]
            .iter()
            .any(|scheme| after(target, scheme).is_some())
}

/// What follows `prefix` in `text`, with ASCII letter case aside.
fn after<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| text.get(prefix.len()..))
        .flatten()
}

#[cfg(test)]
mod tests;
