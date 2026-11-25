//! The formulas of the parts other than the workbook part, rewritten for the
//! renames and deletes `apply` writes: which parts are read, the walk of each
//! (`parts/walk.rs`), the formulas that are not rewritten, reported in
//! the `StructureReport`, and the defined names of the workbook part that
//! are reported.
//!
//! Nothing is read when the edit made no rename or delete. Otherwise each
//! candidate part is read once as a stream to learn whether anything in it
//! changes; only the parts that change are replaced, with a rewriter that
//! walks the part again, so every other part stays the compressed bytes the
//! package has.
//!
//! Formulas are rewritten only in the parts of these media types: worksheet,
//! macrosheet, dialogsheet, chart and table, and a pivot cache definition's
//! `worksheetSource`. A formula element in any other XML part is reported
//! and not written, since what an element named like a formula means in such
//! a part is not known. The app part (`docProps/app.xml`) is not read:
//! `AppRewriter` replaces it.
//!
//! The report holds at most `MAX_KEPT` references in all. Each part's walk
//! is given the number of references the report can still hold (`Keeper`),
//! so no part keeps more than the report can take; the second walk of a part,
//! which writes it, keeps nothing.
//!
//! - **`parts/walk.rs`**: the walk of one part.
//! - **`parts/keeper.rs`**: the references a walk keeps.
//! - **`parts/text.rs`**: what the changes do to one formula.
//! - **`parts/pivot.rs`**: a pivot cache's `worksheetSource`.
//! - **`parts/names.rs`**: the defined names that are reported.

mod keeper;
mod names;
mod pivot;
#[cfg(test)]
mod tests;
mod text;
mod walk;

use super::change::{Change, SheetIds};
use super::snapshot::Snapshot;
use super::state::State;
use crate::structure::report::MAX_KEPT;
use crate::structure::{KeptReason, KeptReference};
use crate::{PackageEdit, PartRewriter, WriteError, WriteLimits};
use keeper::Keeper;
use names::names;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::rc::Rc;
use std::sync::Arc;
use text::Context;
use walk::{Outcome, Rules, walk};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::PartName;
use xlsx_graft_xml::{XmlRewriter, XmlStreamError};

/// The media types of the parts whose formulas are rewritten, lowered as
/// `ContentType::essence` gives them.
const REWRITTEN: [&str; 5] = [
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
    "application/vnd.ms-excel.macrosheet+xml",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml",
    "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml",
];

/// The media type of a pivot cache definition part, lowered as
/// `ContentType::essence` gives it.
const PIVOT_CACHE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotcachedefinition+xml";

/// What the formulas of the other parts came to.
#[derive(Default)]
pub(super) struct Found {
    /// Whether a `#REF!` was written.
    pub(super) ref_written: bool,
    /// What was left as it was, up to `MAX_KEPT`.
    pub(super) kept: Vec<KeptReference>,
    /// How many more were left as they were.
    pub(super) omitted: u64,
}

impl Found {
    /// Adds `kept`, which holds no more references than `MAX_KEPT` had left,
    /// and the `omitted` that did not fit.
    fn add(&mut self, kept: Vec<KeptReference>, omitted: u64) {
        self.kept.extend(kept);
        self.omitted = self.omitted.saturating_add(omitted);
    }

    /// The number of references the report can still hold.
    fn remaining(&self) -> usize {
        MAX_KEPT.saturating_sub(self.kept.len())
    }
}

/// A part to read.
struct Candidate {
    index: usize,
    name: PartName,
    rules: Rules,
}

/// What a part of the media type `essence` may have rewritten, and whether
/// it is read at all (an XML part).
fn rules(essence: &str) -> Option<Rules> {
    is_xml(essence).then(|| Rules {
        formulas: REWRITTEN.contains(&essence),
        pivot: essence == PIVOT_CACHE,
    })
}

/// Writes a part again with the formulas the changes touch rewritten.
struct PartFormulas {
    name: String,
    rules: Rules,
    context: Rc<Context>,
    limits: WriteLimits,
}

impl PartRewriter for PartFormulas {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut rw = XmlRewriter::new(original, out, &self.limits.xml);
        // The first walk kept the references; this one keeps none.
        walk(&mut rw, &self.name, &self.context, self.rules, 0)?;
        Ok(())
    }
}

/// Rewrites the formulas of the parts of `edit`'s package, for the
/// `changes` (with the `ids` that number the sheet names of their deletes):
/// each part but the workbook part, a Relationships part, one of `removed`
/// and one whose media type is not XML (`+xml`, `application/xml` or
/// `text/xml`), in the order of the package's parts. The workbook part's
/// defined names, in `state`, are only reported, as
/// `State::rewrite_formulas` rewrote them.
///
/// # Errors
///
/// [`WriteError::Io`] when reading the package fails, or the error of
/// `PackageEdit::replace`.
pub(super) fn rewrite<'r, R: Read + Seek>(
    edit: &mut PackageEdit<'r, R>,
    snapshot: &Snapshot,
    state: &State<'_>,
    (changes, ids): (Vec<Change>, SheetIds),
    removed: &[&PartName],
) -> Result<Found, WriteError> {
    if changes.is_empty() {
        return Ok(Found::default());
    }
    let limits = snapshot.limits;
    let gone: BTreeSet<&str> = removed.iter().map(|name| name.key()).collect();
    let parts: Vec<(PartName, Option<Rules>)> = (edit.package_mut().parts().iter())
        .map(|part| {
            let rules = part.content_type().and_then(|kind| rules(kind.essence()));
            (part.name().clone(), rules)
        })
        .collect();
    let app = snapshot.app.as_ref().map(|app| &app.name);
    let removed_tables = (snapshot.workbook.tables().iter()).filter(|table| {
        let name = parts.get(table.part()).map(|(name, _)| name.key());
        name.is_some_and(|key| gone.contains(key))
    });
    let mut tables: BTreeMap<String, Arc<str>> = BTreeMap::new();
    let spelled = (removed_tables.flat_map(|table| [table.name(), table.display_name()])).flatten();
    for spelled in spelled {
        tables
            .entry(spelled.to_lowercase())
            .or_insert_with(|| Arc::from(spelled));
    }
    let context = Rc::new(Context::new(changes, ids, tables));
    let candidates = (parts.into_iter().enumerate())
        .filter(|(_, (name, _))| {
            !name.is_relationships()
                && *name != snapshot.workbook_name
                && Some(name) != app
                && !gone.contains(name.key())
        })
        .filter_map(|(index, (name, rules))| rules.map(|rules| Candidate { index, name, rules }));
    let mut found = Found::default();
    for candidate in candidates {
        let name = candidate.name.as_str();
        let remaining = found.remaining();
        let outcome = scan(edit, &candidate, &context, &limits, remaining)?;
        found.ref_written |= outcome.ref_written;
        found.add(outcome.kept, outcome.omitted);
        if outcome.changed {
            let rewriter = PartFormulas {
                name: name.to_owned(),
                rules: candidate.rules,
                context: Rc::clone(&context),
                limits,
            };
            edit.replace(&candidate.name, Box::new(rewriter))?;
        }
    }
    let (kept, omitted) = names(snapshot, state, &context, found.remaining());
    found.add(kept, omitted);
    Ok(found)
}

/// Walks the part `candidate`, with its output written to `io::sink()`,
/// keeping up to `remaining` references (the number the report can still
/// hold). A part that cannot be read as XML to its end (a damaged zip entry
/// or XML that is not well formed) is the one reference
/// [`KeptReason::Unreadable`] (counted when `remaining` is 0), and is not
/// changed.
fn scan<R: Read + Seek>(
    edit: &mut PackageEdit<'_, R>,
    candidate: &Candidate,
    context: &Context,
    limits: &WriteLimits,
    remaining: usize,
) -> Result<Outcome, WriteError> {
    let name = candidate.name.as_str();
    // The diagnostic the zip reader pushes for a damaged entry is not kept:
    // the part is reported as unreadable.
    let mut opened = Diagnostics::new();
    let entry = edit.package_mut().open_part(candidate.index, &mut opened)?;
    let Some(entry) = entry else {
        return Ok(unreadable(name, remaining));
    };
    let mut rw = XmlRewriter::new(BufReader::new(entry), io::sink(), &limits.xml);
    let walked = walk(&mut rw, name, context, candidate.rules, remaining);
    classify(walked, rw.get_ref().get_ref().is_stopped(), name, remaining)
}

/// What the walk of the part `name` is: its outcome, or the one reference
/// [`KeptReason::Unreadable`] (kept when `remaining` is not 0) when the XML is
/// refused, or when the zip entry stopped on damage of its own (`stopped`;
/// the zip reader pushed the diagnostic for it).
///
/// # Errors
///
/// [`WriteError::Io`] for a failure of the host.
fn classify(
    walked: Result<Outcome, XmlStreamError>,
    stopped: bool,
    name: &str,
    remaining: usize,
) -> Result<Outcome, WriteError> {
    match walked {
        Ok(outcome) => Ok(outcome),
        Err(XmlStreamError::Invalid(_)) => Ok(unreadable(name, remaining)),
        Err(XmlStreamError::Io(_)) if stopped => Ok(unreadable(name, remaining)),
        Err(XmlStreamError::Io(error)) => Err(WriteError::from(error)),
    }
}

/// The outcome for the part `name`, which could not be read, when the report
/// can still hold `remaining` references.
fn unreadable(name: &str, remaining: usize) -> Outcome {
    let mut keeper = Keeper::new(name, remaining);
    keeper.keep("", None, None, None, KeptReason::Unreadable);
    let (kept, omitted) = keeper.finish();
    Outcome {
        kept,
        omitted,
        ..Outcome::default()
    }
}

/// Whether `essence` is an XML media type: one that ends in `+xml`, or
/// `application/xml` or `text/xml`.
fn is_xml(essence: &str) -> bool {
    essence.ends_with("+xml") || essence == "application/xml" || essence == "text/xml"
}
