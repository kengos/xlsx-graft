//! [`walk`]: one part read as XML and written again with the formulas the
//! changes touch rewritten, every other event as it came.
//!
//! The same walk runs twice over a part: written to `io::sink()`, to learn
//! what it changes and which formulas it keeps as they were, and written to
//! the part's new content when it changes something, so the second run
//! decides from the same bytes as the first.
//!
//! A formula is found as `xlsx_graft_formula::places` says (by local name,
//! in any namespace), and what a rename or a delete does to it is the
//! row's `on_sheet_change`:
//!
//! - **`Rewrite`**: in a part of a kind [`Rules::formulas`] allows, the text
//!   of the element is replaced by the formula with its sheet prefixes
//!   changed, as one text event. In any other part it is `Report`.
//! - **`Report`**: the formula is written as it was, and kept in the report
//!   when a change would touch it. Every attribute in `FORMULA_ATTRIBUTES`
//!   is `Report` (or `Ignore`): its value is never written again.
//! - A `Rewrite` or `Report` formula is kept in the report when
//!   `xlsx_graft_formula::analyze` reports a malformed place in it and its
//!   text holds the name of a sheet changed ([`KeptReason::Malformed`]),
//!   when it holds an element beside its text and the text holds the name
//!   ([`KeptReason::Markup`]), and when it is a structured reference to a
//!   table removed.
//!
//! The text of a formula element is every text, CDATA section and reference
//! in it, joined across comments and processing instructions. The events are
//! held until the element ends, and written as they were, or replaced by one
//! new text event. A formula element nested in another is a formula of its
//! own, so the formula elements being read are a stack; when an element
//! starts inside one, the events held are written as they were, and that
//! formula is never rewritten.
//!
//! What is held is bounded (`walk/held.rs`): past `MAX_FORMULA_BYTES` of
//! text, comments and processing instructions the events held are written as
//! they were, the rest of the element is written without being held, and
//! the formula is reported as [`KeptReason::TooLong`]. What is kept for the
//! report is bounded by `remaining`, the number of references the report can
//! still hold, which the caller gives (`parts/keeper.rs`):
//! past it a reference is counted and nothing of it is held.
//!
//! What a walk keeps and reports is in `walk/kept.rs`.
//!
//! `at` is the cell reference (`B2`) of the `c` the formula is in, the
//! `sqref` of the `dataValidation` or `conditionalFormatting` it is in, or
//! `definedName <name>`, each cut at `MAX_AT_BYTES`.

use super::super::rewrite::text_of;
use super::super::state::escaped;
use super::keeper::Keeper;
use super::text::{Context, attribute, clipped, defined_at};
use crate::structure::{KeptReason, KeptReference};
use held::{Held, Open};
use std::io::{BufRead, Write};
use std::sync::Arc;
use xlsx_graft_formula::places::{OnSheetChange, formula_element};
use xlsx_graft_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use xlsx_graft_xml::{XmlRewriter, XmlStreamError};

mod held;
mod kept;

/// What a walk may change in a part, by the part's kind.
#[derive(Debug, Clone, Copy)]
pub(super) struct Rules {
    /// Whether the text of a `Rewrite` element is rewritten.
    pub(super) formulas: bool,
    /// Whether a `worksheetSource`'s `sheet` is rewritten.
    pub(super) pivot: bool,
}

/// What walking a part found.
#[derive(Default)]
pub(super) struct Outcome {
    /// Whether the part's text is not what was read.
    pub(super) changed: bool,
    /// Whether a `#REF!` was written.
    pub(super) ref_written: bool,
    /// What was left as it was, up to `MAX_KEPT`.
    pub(super) kept: Vec<KeptReference>,
    /// How many more were left as they were.
    pub(super) omitted: u64,
}

/// The walk of one part.
struct Walk<'a> {
    context: &'a Context,
    rules: Rules,
    keeper: Keeper,
    /// Whether the part's text is not what was read.
    changed: bool,
    /// Whether a `#REF!` was written.
    ref_written: bool,
    /// The `r` of the `c` being read.
    cell: Option<Arc<str>>,
    /// The `sqref` of the data validation or conditional format being read.
    range: Option<Arc<str>>,
    open: Vec<Open>,
}

/// Reads the part `rw` reads to its end, writing it to `rw` with the
/// changes of `context` made as `rules` allow. `part` is the part's name.
/// At most `remaining` references (the number the report can still hold) are
/// kept in the outcome; the others are counted.
///
/// # Errors
///
/// The error of `rw`: the XML is refused, or reading or writing fails.
pub(super) fn walk<R: BufRead, W: Write>(
    rw: &mut XmlRewriter<R, W>,
    part: &str,
    context: &Context,
    rules: Rules,
    remaining: usize,
) -> Result<Outcome, XmlStreamError> {
    let mut walk = Walk {
        context,
        rules,
        keeper: Keeper::new(part, remaining),
        changed: false,
        ref_written: false,
        cell: None,
        range: None,
        open: Vec::new(),
    };
    let mut buf = Vec::new();
    while let Some(event) = rw.next(&mut buf)? {
        match event {
            Event::Start(start) => walk.start(rw, start, true)?,
            Event::Empty(start) => walk.start(rw, start, false)?,
            Event::End(end) => walk.end(rw, end)?,
            other => walk.other(rw, other)?,
        }
    }
    let (kept, omitted) = walk.keeper.finish();
    Ok(Outcome {
        changed: walk.changed,
        ref_written: walk.ref_written,
        kept,
        omitted,
    })
}

/// Writes `event` to `rw`.
fn put<R: BufRead, W: Write>(
    rw: &mut XmlRewriter<R, W>,
    event: Event<'_>,
) -> Result<(), XmlStreamError> {
    rw.write(event).map_err(XmlStreamError::Io)
}

impl Walk<'_> {
    /// Handles the start tag `start`: an element opened when `opened`, else
    /// an empty element.
    fn start<R: BufRead, W: Write>(
        &mut self,
        rw: &mut XmlRewriter<R, W>,
        start: BytesStart<'_>,
        opened: bool,
    ) -> Result<(), XmlStreamError> {
        if let Some(top) = self.open.last_mut() {
            for held in top.held.child() {
                put(rw, held)?;
            }
        }
        let local = rw.resolve_element(start.name()).1;
        let local = local.as_ref();
        match local {
            "c" if opened => self.cell = attribute(&start, "r").map(|r| clipped(&r)),
            "dataValidation" | "conditionalFormatting" if opened => {
                self.range = attribute(&start, "sqref").map(|sqref| clipped(&sqref));
            }
            _ => {}
        }
        self.attributes(local, &start);
        let renamed = self.pivot(rw, local, &start);
        let written = match &renamed {
            Some(renamed) => renamed.borrow(),
            None => start.borrow(),
        };
        match formula_element(local) {
            Some(row) if opened => {
                let at = self.at_of(local, &written);
                let held = Held::holding(written.into_owned());
                self.open.push(Open { row, at, held });
                Ok(())
            }
            _ if opened => put(rw, Event::Start(written)),
            _ => put(rw, Event::Empty(written)),
        }
    }

    /// Where a formula in the element `local`, whose start tag is `start`,
    /// is.
    fn at_of(&self, local: &str, start: &BytesStart<'_>) -> Option<Arc<str>> {
        let named = (local == "definedName")
            .then(|| attribute(start, "name"))
            .flatten()
            .filter(|name| !name.is_empty());
        match (named, local) {
            (Some(name), _) => Some(defined_at(&name)),
            (None, "f") if self.cell.is_some() => self.cell.clone(),
            _ => self.range.clone(),
        }
    }

    /// Handles the end tag `end`.
    fn end<R: BufRead, W: Write>(
        &mut self,
        rw: &mut XmlRewriter<R, W>,
        end: BytesEnd<'_>,
    ) -> Result<(), XmlStreamError> {
        let local = rw.resolve_element(end.name()).1;
        let local = local.as_ref();
        match local {
            "c" => self.cell = None,
            "dataValidation" | "conditionalFormatting" => self.range = None,
            _ => {}
        }
        match self.open.pop_if(|open| open.row.local == local) {
            Some(open) => self.close(rw, open, end),
            None => put(rw, Event::End(end)),
        }
    }

    /// Handles the end of the formula element `open`, whose end tag is `end`.
    fn close<R: BufRead, W: Write>(
        &mut self,
        rw: &mut XmlRewriter<R, W>,
        open: Open,
        end: BytesEnd<'_>,
    ) -> Result<(), XmlStreamError> {
        let Open { row, at, held } = open;
        let rewrite = row.on_sheet_change == OnSheetChange::Rewrite && self.rules.formulas;
        match held {
            Held::Holding { events, text, .. } => {
                let mut replaced = None;
                if !text.chars().all(char::is_whitespace) {
                    match self.judge(row.local, &at, &text, rewrite) {
                        Some(done) if rewrite => {
                            self.ref_written |= done.ref_written;
                            replaced = Some(done.text);
                        }
                        Some(done) => {
                            let reason = KeptReason::Outside;
                            self.keeper.keep(
                                row.local,
                                at.as_ref(),
                                Some(&done.sheet),
                                None,
                                reason,
                            );
                        }
                        None => {}
                    }
                }
                match replaced {
                    Some(text) => self.write_replaced(rw, events, &text)?,
                    None => {
                        for held in events {
                            put(rw, held)?;
                        }
                    }
                }
            }
            Held::Markup { text, .. } => {
                if let Some(sheet) = self.context.mentioned(&text) {
                    let reason = KeptReason::Markup;
                    self.keeper
                        .keep(row.local, at.as_ref(), Some(sheet), None, reason);
                }
            }
            Held::TooLong => {
                let reason = KeptReason::TooLong;
                self.keeper.keep(row.local, at.as_ref(), None, None, reason);
            }
        }
        put(rw, Event::End(end))
    }

    /// Writes the start tag of `events`, the events of a formula element
    /// that is rewritten, and then `text` in place of the rest of them.
    fn write_replaced<R: BufRead, W: Write>(
        &mut self,
        rw: &mut XmlRewriter<R, W>,
        events: Vec<Event<'static>>,
        text: &str,
    ) -> Result<(), XmlStreamError> {
        self.changed = true;
        for start in events.into_iter().take(1) {
            put(rw, start)?;
        }
        let text = text_of(&escaped(text));
        put(rw, Event::Text(BytesText::from_escaped(text)))
    }
}
