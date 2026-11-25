//! [`WorkbookRewriter`]: the workbook part written again with the sheets,
//! the defined names, the workbook views and `calcPr` the edit changed,
//! and every other byte as it was.
//!
//! [`Plan`] (`rewrite/plan.rs`) is what the state says to write. The part is
//! read with an `XmlRewriter` and each event written as it came, except in
//! the places the plan changes, which are the children of the root in its
//! own namespace (the places the model reads, `structure/model/walk.rs`):
//!
//! - **`rewrite/sheets.rs`**: `sheets` is read to its end, and its `sheet`
//!   elements written again in the order the plan gives, with the new ones.
//! - **`rewrite/names.rs`**: each `definedName` is dropped, written again
//!   or written as it was, and the names added are written last.
//! - **`rewrite/insert.rs`**: the workbook views, `calcPr`, and the
//!   `definedNames` and `calcPr` elements the part did not have.
//!
//! Names, attribute values and text are written through
//! [`escape`](xlsx_graft_cell::escape) first when they are values, and then
//! through `xml_attribute` or `xml_text`; the plan holds each in the first
//! form.

mod insert;
mod names;
mod plan;
mod sheets;

pub(super) use insert::{text_of, with_attributes};

use crate::{PartRewriter, WriteError, WriteLimits};
use sheets::Collected;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

/// A defined name to write: the `name` and the text as ST_Xstring values
/// (their `_xHHHH_` escapes made), and its `localSheetId`.
pub(super) struct NameOut {
    name: String,
    local: Option<u32>,
    text: String,
}

/// A sheet to write, in its place in the list.
pub(super) enum SheetOut {
    /// A sheet of the workbook that was opened: the position of its
    /// element among the original `sheet`s, and its new name when renamed.
    Original { index: usize, name: Option<String> },
    /// A sheet added.
    Added {
        name: String,
        sheet_id: u32,
        rid: String,
    },
}

/// The new values of the attributes of a `workbookView`.
pub(super) struct ViewOut {
    active: Option<String>,
    first: Option<String>,
}

/// What to write of the workbook part.
pub(super) struct Plan {
    /// The sheets in their order, when they changed.
    sheets: Option<Vec<SheetOut>>,
    /// Whether a defined name was removed, written again or added.
    names_changed: bool,
    /// How many defined names are written.
    names_total: usize,
    /// The original defined names left out, by position among them.
    removed: BTreeSet<usize>,
    /// The original defined names written again, by position among them.
    rebuilt: BTreeMap<usize, NameOut>,
    /// The defined names added, in order.
    added: Vec<NameOut>,
    /// The workbook views, by position among them.
    views: Vec<ViewOut>,
    /// Whether `calcPr` gets `fullCalcOnLoad`.
    ref_written: bool,
    /// Whether the part has no `calcPr` to give it to.
    insert_calc: bool,
    /// Whether names are added and the part has no `definedNames` element to
    /// write them in.
    insert_names: bool,
}

/// Writes the workbook part as a [`Plan`] says.
pub(super) struct WorkbookRewriter {
    pub(super) plan: Plan,
    pub(super) limits: WriteLimits,
}

impl PartRewriter for WorkbookRewriter {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        run(&self.plan, original, out, &self.limits.xml)
    }
}

/// The element of the root that is open and being rewritten.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Sheets,
    Names,
    Views,
}

/// The rewriter the part is written with.
type Rw<'a> = XmlRewriter<&'a mut dyn BufRead, &'a mut dyn Write>;

/// The walk through the part.
struct Run<'a> {
    plan: &'a Plan,
    rw: Rw<'a>,
    /// The root's prefix with its `:`, `""` when it has none.
    prefix: String,
    /// The root's namespace.
    namespace: String,
    /// How many elements are open.
    depth: usize,
    section: Section,
    /// The depth of an element dropped, until its end.
    skipping: Option<usize>,
    /// Whether `definedNames` is still to be added.
    pending_names: bool,
    /// Whether `calcPr` is still to be added.
    pending_calc: bool,
    /// The position of the next `definedName` among the original ones.
    name_index: usize,
    /// The position of the next `workbookView` among the original ones.
    view_index: usize,
    sheets: Collected,
}

/// Reads `original` and writes it to `out` as `plan` says.
fn run(
    plan: &Plan,
    original: &mut dyn BufRead,
    out: &mut dyn Write,
    limits: &XmlLimits,
) -> Result<(), WriteError> {
    let mut run = Run {
        plan,
        rw: XmlRewriter::new(original, out, limits),
        prefix: String::new(),
        namespace: String::new(),
        depth: 0,
        section: Section::None,
        skipping: None,
        pending_names: plan.insert_names,
        pending_calc: plan.insert_calc,
        name_index: 0,
        view_index: 0,
        sheets: Collected::default(),
    };
    let mut buf = Vec::new();
    while let Some(event) = run.rw.next(&mut buf)? {
        run.event(event)?;
    }
    Ok(())
}

impl Run<'_> {
    /// Handles `event`, which the XML reader has just read.
    fn event(&mut self, event: Event<'_>) -> Result<(), WriteError> {
        let before = self.depth;
        match &event {
            Event::Start(_) => self.depth = self.depth.saturating_add(1),
            Event::End(_) => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        if let Some(open) = self.skipping {
            let closes = matches!(event, Event::End(_)) && before == open;
            self.skipping = self.skipping.filter(|_| !closes);
            return Ok(());
        }
        if self.section == Section::Sheets {
            return self.collect(event, before);
        }
        let handled = match &event {
            Event::Start(start) if before == 0 => {
                self.root(start);
                false
            }
            Event::Start(start) => self.open(before, start, true)?,
            Event::Empty(start) => self.open(before, start, false)?,
            Event::End(_) => self.close(before)?,
            _ => false,
        };
        if !handled {
            self.rw.write(event)?;
        }
        Ok(())
    }

    /// Takes the prefix and the namespace of the root `start`.
    fn root(&mut self, start: &BytesStart<'_>) {
        let name = start.name();
        let name = name.as_ref();
        let colon = name.find(':').map_or(0, |at| at.saturating_add(1));
        self.prefix = name.get(..colon).unwrap_or_default().to_owned();
        let (resolved, _) = self.rw.resolve_element(start.name());
        self.namespace = namespace(&resolved).unwrap_or_default().to_owned();
    }

    /// The local name of `start` when it is in the root's namespace.
    fn local(&self, start: &BytesStart<'_>) -> Option<String> {
        let (resolved, local) = self.rw.resolve_element(start.name());
        let ours = namespace(&resolved) == Some(self.namespace.as_str());
        ours.then(|| local.as_ref().to_owned())
    }

    /// Handles an element start (`opened`) or an empty element at depth
    /// `before`. Whether the element was handled (written in a new form or left
    /// out), so the event is not written as it came.
    fn open(
        &mut self,
        before: usize,
        start: &BytesStart<'_>,
        opened: bool,
    ) -> Result<bool, WriteError> {
        let local = self.local(start);
        let local = local.as_deref();
        match (before, self.section, local) {
            (1, _, local) => self.child(start, local.unwrap_or_default(), opened),
            (2, Section::Names, Some("definedName")) => self.name(start, opened),
            (2, Section::Views, Some("workbookView")) => self.view(start, opened),
            _ => Ok(false),
        }
    }

    /// Handles the end of an element at depth `before`.
    fn close(&mut self, before: usize) -> Result<bool, WriteError> {
        if before == 1 {
            self.insert_pending(None)?;
        }
        if before == 2 && self.section == Section::Names {
            self.write_added()?;
        }
        if before == 2 {
            self.section = Section::None;
        }
        Ok(false)
    }

    /// Handles a child of the root whose local name is `local`.
    fn child(
        &mut self,
        start: &BytesStart<'_>,
        local: &str,
        opened: bool,
    ) -> Result<bool, WriteError> {
        self.pending_calc &= local != "calcPr";
        self.insert_pending(Some(local))?;
        match local {
            "sheets" if opened && self.plan.sheets.is_some() => {
                self.section = Section::Sheets;
                Ok(false)
            }
            "definedNames" if self.plan.names_changed => self.names_start(start, opened),
            // An empty `bookViews` has no views to hold the section open for,
            // and the section ends at the next end of a child of the root.
            "bookViews" => {
                self.section = Section::Views;
                Ok(false)
            }
            "calcPr" if self.plan.ref_written => self.calc(start, opened),
            _ => Ok(false),
        }
    }
}

/// The namespace `resolved` binds, if it binds one.
fn namespace<'a>(resolved: &ResolveResult<'a>) -> Option<&'a str> {
    match *resolved {
        ResolveResult::Bound(Namespace(ns)) => Some(ns),
        _ => None,
    }
}
