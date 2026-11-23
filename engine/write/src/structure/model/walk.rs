//! [`read`]: the XML of the workbook part, read to its end into the sheets
//! as listed, the defined names, the views and whether `calcPr` is there.
//!
//! Only a few places are read: right inside the root `workbook`, the
//! `sheets`, `definedNames`, `bookViews` and `calcPr`, and right inside those
//! the `sheet`, `definedName` and `workbookView`, all in the root's own
//! SpreadsheetML namespace (Transitional or Strict, ECMA-376-1:2016 §18.2).
//! Every other element, in another namespace, in markup compatibility's
//! alternate content, or one this does not read, is skipped with all it
//! holds, and the text outside a `definedName` is not looked at. What is
//! refused is what would make the model wrong: a root that is not a
//! `workbook`, no `sheets` or no `sheet`, a second `sheets`, `definedNames`,
//! `bookViews` or `calcPr`, a `sheet` that lacks an attribute the model
//! needs or has a value its type does not allow, and an element inside a
//! `definedName`, whose text an edit that rewrites it would lose.
//! `xlsx-graft-read` refuses an attribute the schema does not give an element
//! it reads (`workbook`, `workbookPr`, `sheets` and `sheet`, as
//! `unexpected_attribute`) and skips `definedName`, `workbookView` and
//! `calcPr` whole, attributes too. The model reads only the attributes it
//! holds, on the elements it holds.
//!
//! The defined names are counted as they are read and the first
//! [`MAX_DEFINED_NAMES`] are kept: the count is bounded by the part's size
//! alone, and each name held costs memory beyond its text. Every
//! `definedName` element is counted, ones with no text included, which
//! `xlsx-graft-verify` does not count, so a part it accepts may be refused
//! here. A part that defines more is refused with `defined_names_too_many`.

mod attributes;
mod child;
mod text;

use super::{BookView, DefinedName, SheetState};
use attributes::{namespace, refuse};
use child::Frame;
use text::resolve;
use xlsx_graft_diagnostics::{Diagnostic, Limit, SchemaReason};
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlLimits, XmlReader};

/// The namespaces of SpreadsheetML, Transitional and Strict
/// (ECMA-376-1:2016 §8.1, ECMA-376-4:2016 §8.1). The root is in one, and
/// so is every element read.
const NAMESPACES: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// The most defined names a part may define, as `maxDefinedNames` of
/// `xlsx-graft-verify` is by default. Every `definedName` element is
/// counted, ones with no text included, which `xlsx-graft-verify` does not
/// count, so a workbook it accepts may be refused for having more. A later
/// change may move the limit into [`WriteLimits`](crate::WriteLimits).
const MAX_DEFINED_NAMES: u64 = 65_536;

/// A sheet as `sheets` lists it, before its `r:id` is tied to a part.
#[derive(Debug)]
pub(super) struct Listed {
    /// Its `name`, its escapes undone.
    pub(super) name: String,
    /// Its `sheetId`.
    pub(super) sheet_id: u32,
    /// Its `state`.
    pub(super) state: SheetState,
    /// Its `r:id`.
    pub(super) relationship_id: String,
}

/// What the XML of the workbook part says: the sheets, the defined names,
/// the views and whether it has a `calcPr`.
#[derive(Debug)]
pub(super) struct Walked {
    /// The root is in the Strict namespace.
    pub(super) strict: bool,
    /// The sheets, in order.
    pub(super) sheets: Vec<Listed>,
    /// The defined names, in order.
    pub(super) defined_names: Vec<DefinedName>,
    /// The workbook views, in order.
    pub(super) views: Vec<BookView>,
    /// There is a `calcPr`.
    pub(super) calc_pr: bool,
}

/// The state of the walk through the workbook part: the elements open, how
/// deep it is inside a skipped element, and the values read so far.
struct Walk {
    /// Which of [`NAMESPACES`] the root is in.
    flavor: usize,
    /// The elements read that are open.
    frames: Vec<Frame>,
    /// How many elements deep the walk is inside a skipped one.
    skipped: u64,
    /// What the root has had of each of its children read.
    seen: Seen,
    /// The sheets read so far.
    sheets: Vec<Listed>,
    /// The defined names kept so far: at most [`MAX_DEFINED_NAMES`].
    defined_names: Vec<DefinedName>,
    /// How many `definedName` elements were read, those not kept included.
    defined_name_count: u64,
    /// The workbook views read so far.
    views: Vec<BookView>,
    /// Whether a `calcPr` was read.
    calc_pr: bool,
}

/// Which children of the root were read: each may come once.
#[derive(Default)]
struct Seen {
    sheets: bool,
    defined_names: bool,
    book_views: bool,
    calc_pr: bool,
}

/// Reads the part from `bytes` under the XML limits `limits` to
/// [`Event::Eof`]: what it says, or the problem that stopped it.
pub(super) fn read(bytes: &[u8], limits: &XmlLimits) -> Result<Walked, Diagnostic> {
    let mut reader = XmlReader::open(bytes, limits)?;
    let mut walk = Walk {
        flavor: 0,
        frames: Vec::new(),
        skipped: 0,
        seen: Seen::default(),
        sheets: Vec::new(),
        defined_names: Vec::new(),
        defined_name_count: 0,
        views: Vec::new(),
        calc_pr: false,
    };
    let mut buf = Vec::new();
    loop {
        let event = reader.read_event_into(&mut buf)?;
        let offset = reader.offset();
        match event {
            Event::Start(start) => walk.start(&reader, &start, offset, true)?,
            Event::Empty(start) => walk.start(&reader, &start, offset, false)?,
            Event::End(_) => walk.end()?,
            Event::Text(text) => walk.text(&text.xml10_content()),
            Event::CData(data) => walk.text(&data.xml10_content()),
            Event::GeneralRef(reference) => walk.text(&resolve(&reference)),
            Event::Eof if walk.defined_name_count > MAX_DEFINED_NAMES => {
                return Err(Diagnostic::DefinedNamesTooMany {
                    count: walk.defined_name_count,
                    limit: Limit::default_of(MAX_DEFINED_NAMES),
                });
            }
            Event::Eof => {
                return Ok(Walked {
                    strict: walk.flavor == 1,
                    sheets: walk.sheets,
                    defined_names: walk.defined_names,
                    views: walk.views,
                    calc_pr: walk.calc_pr,
                });
            }
            _ => {}
        }
    }
}

impl Walk {
    /// Reads an element that starts at `offset`: a start tag when `opened`,
    /// else an empty element.
    fn start(
        &mut self,
        reader: &XmlReader<'_>,
        start: &BytesStart<'_>,
        offset: u64,
        opened: bool,
    ) -> Result<(), Diagnostic> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_add(u64::from(opened));
            return Ok(());
        }
        let (resolved, local) = reader.resolve_element(start.name());
        let (namespace, local) = (namespace(&resolved), local.as_ref());
        let name = start.name();
        let unexpected = || refuse(offset, SchemaReason::UnexpectedElement, name.as_ref());
        let frame = match self.frames.last().map(Frame::name) {
            None => {
                let flavor = NAMESPACES.iter().position(|&ns| Some(ns) == namespace);
                self.flavor = flavor
                    .filter(|_| local == "workbook")
                    .ok_or_else(unexpected)?;
                Frame::Workbook { offset }
            }
            Some("definedName") => return Err(unexpected()),
            Some(parent) => {
                let ours = NAMESPACES.get(self.flavor).copied() == namespace;
                let child = if ours {
                    self.child(parent, local, start, reader, offset)
                } else {
                    Ok(None)
                };
                let Some(frame) = child? else {
                    self.skipped = u64::from(opened);
                    return Ok(());
                };
                frame
            }
        };
        self.frames.push(frame);
        if !opened {
            self.end()?;
        }
        Ok(())
    }

    /// Reads the end of the element read last.
    fn end(&mut self) -> Result<(), Diagnostic> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_sub(1);
            return Ok(());
        }
        match self.frames.pop() {
            Some(Frame::Workbook { offset }) if !self.seen.sheets => {
                Err(refuse(offset, SchemaReason::MissingElement, "sheets"))
            }
            Some(Frame::Sheets { offset }) if self.sheets.is_empty() => {
                Err(refuse(offset, SchemaReason::MissingElement, "sheet"))
            }
            Some(Frame::DefinedName(name)) => {
                self.defined_names.push(name);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Takes `text` in when a `definedName` is open. The XML reader
    /// admits no text outside the root, and a `definedName` has no child
    /// to skip, so the one open is the one the text is in.
    fn text(&mut self, text: &str) {
        if let Some(Frame::DefinedName(name)) = self.frames.last_mut() {
            name.text.push_str(text);
        }
    }
}
