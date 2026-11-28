//! Which elements of a worksheet `SheetEdit` takes for the ones it knows: by
//! namespace (the main one, Transitional or Strict, or markup
//! compatibility) and local name.

use crate::sheet::WorksheetChild;
use xlsx_graft_xml::name::{LocalName, Namespace, ResolveResult};

/// SpreadsheetML, Transitional and Strict (ECMA-376-1:2016 §8.1,
/// ECMA-376-4:2016 §8.1).
const MAIN: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// Markup compatibility, the namespace of `mc:AlternateContent`
/// (ECMA-376-3:2015 §10.1).
const MARKUP: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// An element name resolved as the reader resolves one.
pub(super) type Resolved<'a, 'n> = (ResolveResult<'a>, LocalName<'n>);

/// Whether the name is `local` in the main namespace.
pub(super) fn is_main(resolved: &Resolved<'_, '_>, local: &str) -> bool {
    in_main(&resolved.0) && resolved.1.as_ref() == local
}

/// Whether the name is `local` in the markup compatibility namespace.
pub(super) fn is_markup(resolved: &Resolved<'_, '_>, local: &str) -> bool {
    let (namespace, name) = resolved;
    matches!(namespace, ResolveResult::Bound(Namespace(ns)) if *ns == MARKUP)
        && name.as_ref() == local
}

/// The child of a worksheet's root the name is, when it is an element of the
/// main namespace whose local name the schema lists.
pub(super) fn child_kind(resolved: &Resolved<'_, '_>) -> Option<WorksheetChild> {
    let (namespace, name) = resolved;
    in_main(namespace)
        .then(|| WorksheetChild::from_local_name(name.as_ref().as_bytes()))
        .flatten()
}

/// Whether the namespace is the main one.
fn in_main(namespace: &ResolveResult<'_>) -> bool {
    matches!(namespace, ResolveResult::Bound(Namespace(ns)) if MAIN.contains(ns))
}
