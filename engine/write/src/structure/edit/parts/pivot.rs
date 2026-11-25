//! A pivot cache's `worksheetSource` (ECMA-376-1:2016 §18.10.1.101): the
//! sheet its data is read from, by name, in its `sheet` attribute.
//!
//! A source with an `r:id` is an external workbook's (the sheet is that
//! workbook's), and one with no `sheet` names a defined name or a table
//! with its `name`: neither is touched.

use super::super::change::Change;
use super::super::ns::RELATIONSHIPS;
use super::super::rewrite::with_attributes;
use super::super::state::escaped;
use super::text::{Context, attribute, unescaped};
use std::io::{BufRead, Write};
use std::sync::Arc;
use xlsx_graft_xml::XmlRewriter;
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::name::{Namespace, ResolveResult};

/// What the changes do to a `worksheetSource`.
pub(super) enum Source {
    /// Nothing: it is another workbook's, names no sheet, or names a sheet
    /// no change touches.
    Keep,
    /// It names a sheet renamed: the start tag with the new name.
    Renamed(BytesStart<'static>),
    /// It names a sheet deleted: the name the workbook gave the sheet before
    /// the first change that touched this source.
    Deleted(Arc<str>),
}

/// What the changes of `context` do to the `worksheetSource` `start`, read
/// by `rw`.
pub(super) fn source<R: BufRead, W: Write>(
    rw: &XmlRewriter<R, W>,
    start: &BytesStart<'_>,
    context: &Context,
) -> Source {
    let external = start.attributes().flatten().any(|attribute| {
        let (resolved, local) = rw.resolve_attribute(attribute.key);
        let relationship =
            matches!(resolved, ResolveResult::Bound(Namespace(ns)) if RELATIONSHIPS.contains(&ns));
        relationship && local.as_ref() == "id"
    });
    let written = attribute(start, "sheet").filter(|_| !external);
    let Some(mut name) = written.map(|value| unescaped(&value)) else {
        return Source::Keep;
    };
    let mut renamed = false;
    let mut first: Option<&Arc<str>> = None;
    let mut cursor = 0;
    // Each change is applied at most once, so the loop ends in as many
    // rounds as there are changes.
    for _ in 0..context.change_count() {
        let Some((index, change)) = context.change_about(&name, cursor) else {
            break;
        };
        cursor = index.saturating_add(1);
        let first_sheet = *first.get_or_insert(change.sheet());
        match change {
            Change::Rename { new, .. } => {
                name.clone_from(new);
                renamed = true;
            }
            Change::Delete(_) => return Source::Deleted(Arc::clone(first_sheet)),
        }
    }
    if renamed {
        let value = escaped(&name);
        return Source::Renamed(with_attributes(start, &[("sheet", Some(&value))]));
    }
    Source::Keep
}
