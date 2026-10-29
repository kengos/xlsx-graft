//! [`read`]: the shared strings part read as a stream into a
//! [`StringTable`], its shape held to the schema as the crate root says.
//!
//! The elements read are a stack of frames, each held to the schema shape
//! (`shape.rs`). A child that is skipped (`extLst`, `rPr`, `rPh`,
//! `phoneticPr`) is passed over with all it holds.

mod shape;

use super::Halt;
use super::escape::{LONGEST_ESCAPE, unescape};
use super::table::{Builder, StringTable};
use crate::ReadLimits;
use crate::schema::{NAMESPACES, is_blank, namespace};
use shape::{Child, Frame, attributes, child, refuse};
use std::io::BufRead;
use xlsx_graft_diagnostics::SchemaReason;
use xlsx_graft_spool::SpoolFactory;
use xlsx_graft_xml::events::{BytesRef, BytesStart, Event};
use xlsx_graft_xml::{XmlStreamError, XmlStreamReader};

/// The predefined entities (XML 1.0 §4.6), the only ones the XML reader
/// admits.
const ENTITIES: [(&str, char); 5] = [
    ("lt", '<'),
    ("gt", '>'),
    ("amp", '&'),
    ("apos", '\''),
    ("quot", '"'),
];

/// Where the walk is.
struct Walk<'a> {
    table: Builder<'a>,
    /// The namespace of the root, which every element shares.
    namespace: Option<&'static str>,
    frames: Vec<Frame>,
    /// How many elements deep the walk is inside a skipped one.
    skipped: u64,
    /// The end of the `t` read so far that may still start an escape,
    /// held back until more of it arrives.
    held: String,
}

/// Reads the part from `source` under `limits`, spilling to a spool from
/// `spools`, to [`Event::Eof`].
pub(super) fn read<B: BufRead>(
    source: B,
    limits: &ReadLimits,
    spools: &mut dyn SpoolFactory,
) -> Result<StringTable, Halt> {
    let mut reader = XmlStreamReader::new(source, &limits.xml);
    let mut walk = Walk {
        table: Builder::new(limits, spools),
        namespace: None,
        frames: Vec::new(),
        skipped: 0,
        held: String::new(),
    };
    let mut buf = Vec::new();
    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event,
            Err(XmlStreamError::Invalid(diagnostic)) => return Err(Halt::Refused(diagnostic)),
            Err(XmlStreamError::Io(error)) => return Err(Halt::Io(error)),
        };
        let offset = reader.offset();
        match event {
            Event::Start(start) => walk.start(&reader, &start, offset, true)?,
            Event::Empty(start) => walk.start(&reader, &start, offset, false)?,
            Event::End(_) => walk.end()?,
            Event::Text(text) => walk.text(&text.xml10_content(), offset)?,
            Event::CData(data) => walk.text(&data.xml10_content(), offset)?,
            Event::GeneralRef(reference) => walk.text(&resolve(&reference), offset)?,
            Event::Eof => return Ok(walk.table.finish()),
            _ => {}
        }
    }
}

impl Walk<'_> {
    /// Reads an element that starts at `offset`: a start tag when `opened`,
    /// else an empty element.
    fn start<B: BufRead>(
        &mut self,
        reader: &XmlStreamReader<B>,
        start: &BytesStart<'_>,
        offset: u64,
        opened: bool,
    ) -> Result<(), Halt> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_add(u64::from(opened));
            return Ok(());
        }
        let (resolved, local) = reader.resolve_element(start.name());
        let namespace = namespace(resolved);
        let unexpected = || {
            refuse(
                offset,
                SchemaReason::UnexpectedElement,
                start.name().as_ref(),
            )
        };
        let child = match (self.frames.last_mut(), self.namespace) {
            (None, _) => {
                let root = NAMESPACES.into_iter().find(|&ns| Some(ns) == namespace);
                let root = root.filter(|_| local.as_ref() == "sst");
                self.namespace = Some(root.ok_or_else(unexpected)?);
                Child::Read(Frame::Sst { stage: 0 })
            }
            (Some(frame), ours) if ours == namespace => {
                child(frame, local.as_ref(), offset).ok_or_else(unexpected)?
            }
            (Some(_), _) => return Err(unexpected()),
        };
        let frame = match child {
            Child::Skip => {
                self.skipped = u64::from(opened);
                return Ok(());
            }
            Child::Read(frame) => frame,
        };
        attributes(&frame, start, offset)?;
        match frame {
            Frame::Si { .. } => self.table.begin()?,
            Frame::Text => self.held.clear(),
            Frame::Sst { .. } | Frame::Run { .. } => {}
        }
        self.frames.push(frame);
        if !opened {
            self.end()?;
        }
        Ok(())
    }

    /// Reads the end of the element read last.
    fn end(&mut self) -> Result<(), Halt> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_sub(1);
            return Ok(());
        }
        match self.frames.pop() {
            Some(Frame::Text) => self.value("", true)?,
            Some(Frame::Si { .. }) => self.table.end()?,
            Some(Frame::Run { stage, offset }) if stage < 2 => {
                return Err(refuse(offset, SchemaReason::MissingElement, "t"));
            }
            _ => {}
        }
        Ok(())
    }

    /// Adds `text` to the value of the `t` being read, its escapes undone,
    /// holding back what may still start one unless it is the `last`.
    ///
    /// What was held back is joined to no more of `text` than finishes an
    /// escape it may start, so `text` is never copied whole.
    fn value(&mut self, text: &str, last: bool) -> Result<(), Halt> {
        let table = &mut self.table;
        let mut text = text;
        if !self.held.is_empty() {
            let cut = (text.char_indices().map(|(at, _)| at))
                .find(|&at| at >= LONGEST_ESCAPE)
                .unwrap_or(text.len());
            let (head, tail) = text.split_at(cut);
            let mut joined = std::mem::take(&mut self.held);
            let held = joined.len();
            joined.push_str(head);
            let read = unescape(&joined, last && tail.is_empty(), |p| table.text(p))?;
            if tail.is_empty() {
                self.held.push_str(joined.get(read..).unwrap_or_default());
                return Ok(());
            }
            // With a whole escape's length after it, nothing held is held
            // again: reading stopped in `head`.
            text = text.get(read.saturating_sub(held)..).unwrap_or_default();
        }
        let read = unescape(text, last, |piece| table.text(piece))?;
        self.held.push_str(text.get(read..).unwrap_or_default());
        Ok(())
    }

    /// Reads `text`, which starts at `offset`: part of a `t`'s value, or
    /// white space between elements.
    fn text(&mut self, text: &str, offset: u64) -> Result<(), Halt> {
        // A `t` takes any text, so a frame that refuses it is one of the
        // others; none stands before or after the root, where the XML
        // reader refuses text itself.
        let name = match self.frames.last() {
            _ if self.skipped > 0 => return Ok(()),
            Some(Frame::Text) => {
                return self.value(text, false);
            }
            Some(Frame::Sst { .. }) => "sst",
            Some(Frame::Si { .. }) => "si",
            Some(Frame::Run { .. }) | None => "r",
        };
        if is_blank(text) {
            return Ok(());
        }
        Err(refuse(offset, SchemaReason::UnexpectedText, name))
    }
}

/// The text a reference the XML reader admitted stands for.
fn resolve(reference: &BytesRef<'_>) -> String {
    let named = ENTITIES.iter().find(|(name, _)| *name == &**reference);
    let c = reference.resolve_char_ref().ok().flatten();
    c.or(named.map(|&(_, c)| c))
        .map(String::from)
        .unwrap_or_default()
}
