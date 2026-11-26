//! The start tag a walk writes for an element it changes: a pivot cache's
//! `worksheetSource` (`kept.rs`) and a `sheetView`, whose `tabSelected` is
//! set or removed as the part's [`Tabs`](super::super::super::tabs::Tabs)
//! say.

use super::super::text::attribute;
use super::Walk;
use crate::structure::edit::rewrite::with_attributes;
use std::io::{BufRead, Write};
use xlsx_graft_xml::XmlRewriter;
use xlsx_graft_xml::events::BytesStart;

impl Walk<'_> {
    /// The start tag to write for `start`, the element `local`, when that is
    /// not `start` itself: a `worksheetSource` of a sheet renamed, or a
    /// `sheetView` whose tab is selected or unselected.
    ///
    /// A `sheetView` is left as it is when its `workbookViewId` is not there
    /// or is no `u32`, when it has the `tabSelected` the tabs ask for (`1` and
    /// `true` are a selected tab), and when it is unselected and the tabs ask
    /// for that.
    pub(super) fn retag<R: BufRead, W: Write>(
        &mut self,
        rw: &XmlRewriter<R, W>,
        local: &str,
        start: &BytesStart<'_>,
    ) -> Option<BytesStart<'static>> {
        let renamed = self.pivot(rw, local, start);
        let Some(tabs) = self.rules.tabs.as_ref().filter(|_| local == "sheetView") else {
            return renamed;
        };
        let selected = attribute(start, "tabSelected")
            .is_some_and(|value| matches!(value.trim(), "1" | "true"));
        let id = attribute(start, "workbookViewId").and_then(|id| id.trim().parse::<u32>().ok());
        let value = match (id.and_then(|id| tabs.wants(id)), selected) {
            (Some(true), false) => Some("1"),
            (Some(false), true) => None,
            _ => return renamed,
        };
        self.changed = true;
        Some(with_attributes(start, &[("tabSelected", value)]))
    }
}
