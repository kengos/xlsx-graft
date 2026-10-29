//! [`read`]: the styles part, in memory, read into the kind of each cell
//! format, its shape held to the schema as the crate root says.
//!
//! The elements read are a stack of frames, each held to the schema shape
//! (`shape.rs`). A child that is skipped (every child of `styleSheet` but
//! `numFmts` and `cellXfs`, and the children of `xf`) is passed over with
//! all it holds, as is an element in a namespace `mc:Ignorable` names.

mod shape;

use super::FormatKind;
use super::format::classify;
use super::kind::builtin;
use crate::ReadLimits;
use crate::compat::Ignorable;
use crate::schema::{NAMESPACES, is_blank, namespace, refuse};
use shape::{Child, Frame, Values, attributes, child};
use std::collections::BTreeMap;
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};
use xlsx_graft_xml::XmlReader;
use xlsx_graft_xml::events::{BytesStart, Event};

/// Where the walk is.
struct Walk<'a> {
    limits: &'a ReadLimits,
    /// The namespace of the root, which every element read shares.
    namespace: Option<&'static str>,
    /// The elements read.
    frames: Vec<Frame>,
    /// The namespaces the `mc:Ignorable` of the elements read name.
    ignorable: Ignorable,
    /// How many elements deep the walk is inside a skipped one.
    skipped: u64,
    /// The kind of each number format the part defines, by `numFmtId`.
    numbers: BTreeMap<u32, FormatKind>,
    /// How many number formats the part defines.
    defined: u64,
    /// The kind of each cell format, in order.
    cells: Vec<FormatKind>,
}

/// Reads the part from `bytes` under `limits` to [`Event::Eof`]: the kind
/// of each cell format, in order, or the problem that stopped it.
pub(super) fn read(bytes: &[u8], limits: &ReadLimits) -> Result<Vec<FormatKind>, Diagnostic> {
    let mut reader = XmlReader::open(bytes, &limits.xml)?;
    let mut walk = Walk {
        limits,
        namespace: None,
        frames: Vec::new(),
        ignorable: Ignorable::new(&NAMESPACES),
        skipped: 0,
        numbers: BTreeMap::new(),
        defined: 0,
        cells: Vec::new(),
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
            Event::CData(data) if !is_blank(&data.xml10_content()) => walk.text(offset)?,
            Event::GeneralRef(_) => walk.text(offset)?,
            Event::Eof => return Ok(walk.cells),
            _ => {}
        }
    }
}

impl Walk<'_> {
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
                let root = root.filter(|_| local.as_ref() == "styleSheet");
                self.namespace = Some(root.ok_or_else(unexpected)?);
                Child::Read(Frame::StyleSheet { stage: 0 })
            }
            (Some(frame), ours) if ours == namespace => {
                child(frame, local.as_ref(), offset).ok_or_else(unexpected)?
            }
            (Some(_), _) if namespace.is_some_and(|ns| self.ignorable.contains(ns)) => Child::Skip,
            (Some(_), _) => return Err(unexpected()),
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
        let values = attributes(&frame, start, reader, &mut self.ignorable, offset)?;
        self.read(&frame, values)?;
        if !opened {
            self.end()?;
        }
        Ok(())
    }

    /// Takes in what the attributes of `frame` give.
    fn read(&mut self, frame: &Frame, values: Values) -> Result<(), Diagnostic> {
        match frame {
            Frame::NumFmt => {
                self.defined = self.defined.saturating_add(1);
                let limit = self.limits.max_number_formats;
                if self.defined > limit.value {
                    let count = self.defined;
                    return Err(Diagnostic::NumberFormatsTooMany { count, limit });
                }
                let kind = values.code.as_deref().map_or(FormatKind::Unknown, classify);
                let id = values.id.unwrap_or_default();
                // Two definitions of one id that disagree leave it unknown.
                let agreed = self.numbers.get(&id).is_none_or(|&was| was == kind);
                let kind = if agreed { kind } else { FormatKind::Unknown };
                self.numbers.insert(id, kind);
            }
            Frame::Xf { .. } => {
                let count = u64::try_from(self.cells.len()).unwrap_or(u64::MAX);
                let count = count.saturating_add(1);
                let limit = self.limits.max_cell_formats;
                if count > limit.value {
                    return Err(Diagnostic::CellFormatsTooMany { count, limit });
                }
                // An `xf` with no `numFmtId` has the default style's, 0.
                let id = values.id.unwrap_or_default();
                let defined = self.numbers.get(&id).copied();
                let kind = defined.or_else(|| builtin(id));
                self.cells.push(kind.unwrap_or(FormatKind::Unknown));
            }
            Frame::StyleSheet { .. } | Frame::NumFmts | Frame::CellXfs { .. } => {}
        }
        Ok(())
    }

    /// Reads the end of the element read last.
    fn end(&mut self) -> Result<(), Diagnostic> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_sub(1);
            return Ok(());
        }
        self.ignorable.leave();
        match self.frames.pop() {
            Some(Frame::CellXfs { stage: 0, offset }) => {
                Err(refuse(offset, SchemaReason::MissingElement, "xf"))
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
            Some(Frame::NumFmts) => "numFmts",
            Some(Frame::NumFmt) => "numFmt",
            Some(Frame::CellXfs { .. }) => "cellXfs",
            Some(Frame::Xf { .. }) => "xf",
            Some(Frame::StyleSheet { .. }) | None => "styleSheet",
        };
        Err(refuse(offset, SchemaReason::UnexpectedText, name))
    }
}
