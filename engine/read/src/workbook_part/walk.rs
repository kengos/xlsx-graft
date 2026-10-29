//! [`read`]: the workbook part, in memory, read into its sheets as listed
//! and its date system, its shape held to the schema as the crate root
//! says.
//!
//! The elements read are a stack of frames, each held to the schema shape
//! (`shape.rs`), as in the styles part. A child that is skipped (every
//! child of `workbook` but `workbookPr` and `sheets`) is passed over with
//! all it holds, as is an element in a namespace `mc:Ignorable` names and a
//! `mc:AlternateContent` right inside `workbook`.

mod shape;

use super::{DateSystem, SheetState};
use crate::ReadLimits;
use crate::compat::{Ignorable, MARKUP_COMPATIBILITY};
use crate::schema::{NAMESPACES, UNDERSTOOD, is_blank, namespace, refuse};
use shape::{Child, Frame, Values, attributes, child};
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};
use xlsx_graft_xml::XmlReader;
use xlsx_graft_xml::events::{BytesStart, Event};

/// A sheet as `sheets` lists it, before its `r:id` is looked up.
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

/// What the part says.
#[derive(Debug)]
pub(super) struct Read {
    /// The sheets, in order.
    pub(super) sheets: Vec<Listed>,
    /// How it counts days.
    pub(super) date_system: DateSystem,
}

/// Where the walk is.
struct Walk {
    /// Which of [`NAMESPACES`] the root is in, which every element read
    /// shares.
    flavor: Option<usize>,
    /// The elements read.
    frames: Vec<Frame>,
    /// The namespaces the `mc:Ignorable` of the elements read name.
    ignorable: Ignorable,
    /// How many elements deep the walk is inside a skipped one.
    skipped: u64,
    /// The sheets listed so far.
    sheets: Vec<Listed>,
    /// `workbook@conformance` is `strict`.
    strict: bool,
    /// `workbookPr@date1904`.
    date1904: bool,
}

/// Reads the part from `bytes` under the XML limits of `limits` to
/// [`Event::Eof`]: what it says, or the problem that stopped it.
pub(super) fn read(bytes: &[u8], limits: &ReadLimits) -> Result<Read, Diagnostic> {
    let mut reader = XmlReader::open(bytes, &limits.xml)?;
    let mut walk = Walk {
        flavor: None,
        frames: Vec::new(),
        ignorable: Ignorable::new(&UNDERSTOOD),
        skipped: 0,
        sheets: Vec::new(),
        strict: false,
        date1904: false,
    };
    let mut buf = Vec::new();
    loop {
        let event = reader.read_event_into(&mut buf)?;
        let offset = reader.offset();
        match event {
            Event::Start(start) => walk.start(&reader, &start, offset, true)?,
            Event::Empty(start) => walk.start(&reader, &start, offset, false)?,
            Event::End(_) => walk.end()?,
            Event::Text(text) if !is_blank(&text.xml10_content()) => walk.text(offset)?,
            Event::CData(data) if !data.is_empty() => walk.text(offset)?,
            Event::GeneralRef(_) => walk.text(offset)?,
            Event::Eof => {
                let date_system = match (walk.date1904, walk.strict) {
                    (true, _) => DateSystem::Base1904,
                    (false, true) => DateSystem::Base1900,
                    (false, false) => DateSystem::Compatibility1900,
                };
                let sheets = walk.sheets;
                return Ok(Read {
                    sheets,
                    date_system,
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
        let namespace = namespace(&resolved);
        let local = local.as_ref();
        let unexpected = || {
            let name = start.name();
            refuse(offset, SchemaReason::UnexpectedElement, name.as_ref())
        };
        let ours = self
            .flavor
            .and_then(|flavor| NAMESPACES.get(flavor))
            .copied();
        let child = match self.frames.last_mut() {
            None => {
                let flavor = NAMESPACES.iter().position(|&ns| Some(ns) == namespace);
                let flavor = flavor.filter(|_| local == "workbook");
                self.flavor = Some(flavor.ok_or_else(unexpected)?);
                Child::Read(Frame::Workbook { stage: 0, offset })
            }
            Some(frame) if ours == namespace => {
                child(frame, local, offset).ok_or_else(unexpected)?
            }
            // Excel writes one between `workbookPr` and `bookViews`; what it
            // offers is never one of the children read.
            Some(Frame::Workbook { .. })
                if namespace == Some(MARKUP_COMPATIBILITY) && local == "AlternateContent" =>
            {
                Child::Skip
            }
            Some(_) if namespace.is_some_and(|ns| self.ignorable.contains(ns)) => Child::Skip,
            Some(_) => return Err(unexpected()),
        };
        let frame = match child {
            Child::Skip => {
                self.skipped = u64::from(opened);
                return Ok(());
            }
            Child::Read(frame) => frame,
        };
        self.ignorable.enter();
        self.frames.push(frame);
        let flavor = self.flavor.unwrap_or_default();
        let values = attributes(&frame, flavor, start, reader, &mut self.ignorable, offset)?;
        self.read(&frame, values);
        if !opened {
            self.end()?;
        }
        Ok(())
    }

    /// Takes in what the attributes of `frame` give, which
    /// [`attributes`] made sure are there.
    fn read(&mut self, frame: &Frame, values: Values) {
        match frame {
            Frame::Workbook { .. } => self.strict = values.strict,
            Frame::WorkbookPr => self.date1904 = values.date1904,
            Frame::Sheet => self.sheets.push(Listed {
                name: values.name.unwrap_or_default(),
                sheet_id: values.sheet_id.unwrap_or_default(),
                state: values.state,
                relationship_id: values.relationship_id.unwrap_or_default(),
            }),
            Frame::Sheets { .. } => {}
        }
    }

    /// Reads the end of the element read last.
    fn end(&mut self) -> Result<(), Diagnostic> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_sub(1);
            return Ok(());
        }
        self.ignorable.leave();
        match self.frames.pop() {
            Some(Frame::Workbook { stage, offset }) if stage < shape::SHEETS => {
                Err(refuse(offset, SchemaReason::MissingElement, "sheets"))
            }
            Some(Frame::Sheets { stage: 0, offset }) => {
                Err(refuse(offset, SchemaReason::MissingElement, "sheet"))
            }
            _ => Ok(()),
        }
    }

    /// Refuses text other than white space at `offset`: no element read
    /// takes it, and none stands outside the root, where the XML reader
    /// refuses text itself.
    fn text(&self, offset: u64) -> Result<(), Diagnostic> {
        if self.skipped > 0 {
            return Ok(());
        }
        let name = match self.frames.last() {
            Some(Frame::WorkbookPr) => "workbookPr",
            Some(Frame::Sheets { .. }) => "sheets",
            Some(Frame::Sheet) => "sheet",
            Some(Frame::Workbook { .. }) | None => "workbook",
        };
        Err(refuse(offset, SchemaReason::UnexpectedText, name))
    }
}
