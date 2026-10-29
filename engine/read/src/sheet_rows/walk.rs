//! [`Walk`]: the worksheet part read event by event up to the end of its
//! next row, its shape held to the schema as the crate root says.
//!
//! The elements read are a stack of frames, each held to the schema shape
//! (`shape.rs`), as in the other parts. Every child of `worksheet` but
//! `sheetData` is passed over with all it holds, as is a
//! `mc:AlternateContent` right inside `worksheet`, an element in a
//! namespace `mc:Ignorable` names, and a cell's formula and extension
//! list. Where each row and cell stands, and what a cell's value is, is
//! `cell.rs`.

use super::Halt;
use super::cell::{Cells, Open};
use super::shape::{self, Child, Frame, attributes, child};
use crate::compat::{Ignorable, MARKUP_COMPATIBILITY};
use crate::schema::{NAMESPACES, UNDERSTOOD, is_blank, namespace, refuse, resolve};
use crate::shared_strings::escape::Xstring;
use crate::shared_strings::rst;
use std::io::BufRead;
use xlsx_graft_diagnostics::SchemaReason;
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlStreamError, XmlStreamReader};

/// Where the walk is.
#[derive(Debug)]
pub(super) struct Walk {
    /// Which of [`NAMESPACES`] the root is in, which every element read
    /// shares.
    flavor: Option<usize>,
    /// The elements read.
    frames: Vec<Frame>,
    /// The namespaces the `mc:Ignorable` of the elements read name.
    ignorable: Ignorable,
    /// How many elements deep the walk is inside a skipped one.
    skipped: u64,
    /// The value of the `v` or `t` being read.
    value: Xstring,
    /// Where the rows and cells read so far stand, and the cell being read.
    pub(super) open: Open,
    /// The buffer each event is read into.
    buf: Vec<u8>,
}

impl Walk {
    /// A walk of a part not yet read.
    pub(super) fn new() -> Self {
        Self {
            flavor: None,
            frames: Vec::new(),
            ignorable: Ignorable::new(&UNDERSTOOD),
            skipped: 0,
            value: Xstring::default(),
            open: Open::default(),
            buf: Vec::new(),
        }
    }

    /// Reads from `reader` up to the end of the next row, put in `cells`:
    /// `true` then, `false` at [`Event::Eof`].
    pub(super) fn next<B: BufRead>(
        &mut self,
        reader: &mut XmlStreamReader<B>,
        cells: &mut Cells<'_>,
    ) -> Result<bool, Halt> {
        let mut buf = std::mem::take(&mut self.buf);
        let next = self.read(reader, &mut buf, cells);
        self.buf = buf;
        next
    }

    /// [`next`](Self::next), each event read into `buf`.
    fn read<B: BufRead>(
        &mut self,
        reader: &mut XmlStreamReader<B>,
        buf: &mut Vec<u8>,
        cells: &mut Cells<'_>,
    ) -> Result<bool, Halt> {
        loop {
            let event = match reader.read_event_into(buf) {
                Ok(event) => event,
                Err(XmlStreamError::Invalid(diagnostic)) => return Err(diagnostic.into()),
                Err(XmlStreamError::Io(error)) => return Err(Halt::Io(error)),
            };
            let offset = reader.offset();
            let ended = match event {
                Event::Start(start) => self.start(reader, &start, offset, true, cells)?,
                Event::Empty(start) => self.start(reader, &start, offset, false, cells)?,
                Event::End(_) => self.end(cells)?,
                Event::Text(text) => {
                    let text = text.xml10_content();
                    self.text(&text, is_blank(&text), offset, cells)?
                }
                Event::CData(data) => {
                    self.text(&data.xml10_content(), data.is_empty(), offset, cells)?
                }
                Event::GeneralRef(reference) => {
                    self.text(&resolve(&reference), false, offset, cells)?
                }
                Event::Eof => return Ok(false),
                _ => false,
            };
            if ended {
                return Ok(true);
            }
        }
    }

    /// Reads an element that starts at `offset`: a start tag when `opened`,
    /// else an empty element. `true` when it ends a row.
    fn start<B: BufRead>(
        &mut self,
        reader: &XmlStreamReader<B>,
        start: &BytesStart<'_>,
        offset: u64,
        opened: bool,
        cells: &mut Cells<'_>,
    ) -> Result<bool, Halt> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_add(u64::from(opened));
            return Ok(false);
        }
        let (resolved, local) = reader.resolve_element(start.name());
        let namespace = namespace(&resolved);
        let local = local.as_ref();
        let unexpected = || {
            let name = start.name();
            Halt::from(refuse(
                offset,
                SchemaReason::UnexpectedElement,
                name.as_ref(),
            ))
        };
        let ours = (self.flavor)
            .and_then(|flavor| NAMESPACES.get(flavor))
            .copied();
        let child = match self.frames.last_mut() {
            None => {
                let flavor = NAMESPACES.iter().position(|&ns| Some(ns) == namespace);
                let flavor = flavor.filter(|_| local == "worksheet");
                self.flavor = Some(flavor.ok_or_else(unexpected)?);
                Child::Read(Frame::Worksheet {
                    stage: 0,
                    offset,
                    data: false,
                })
            }
            Some(frame) if ours == namespace => {
                child(frame, local, offset).ok_or_else(unexpected)?
            }
            // Excel writes them there for form controls and the like; what
            // one offers is never `sheetData`.
            Some(Frame::Worksheet { .. })
                if namespace == Some(MARKUP_COMPATIBILITY) && local == "AlternateContent" =>
            {
                Child::Skip
            }
            Some(_) if namespace.is_some_and(|ns| self.ignorable.contains(ns)) => Child::Skip,
            Some(_) => return Err(unexpected()),
        };
        let mut frame = match child {
            Child::Skip => {
                self.skipped = u64::from(opened);
                return Ok(false);
            }
            Child::Read(frame) => frame,
        };
        self.ignorable.enter();
        let values = attributes(&frame, start, reader, &mut self.ignorable, offset)?;
        match &mut frame {
            Frame::Row { .. } => self.open.row(values.row, cells)?,
            Frame::Cell { inline, .. } => {
                self.open.cell(&values, offset, cells)?;
                *inline = values.kind == shape::Kind::Inline;
            }
            Frame::Value | Frame::Rst(rst::Frame::Text) => {
                self.open.valued();
                self.value.begin();
            }
            Frame::Rst(rst::Frame::Rst { .. }) => self.open.valued(),
            Frame::Worksheet { .. } | Frame::SheetData | Frame::Rst(rst::Frame::Run { .. }) => {}
        }
        self.frames.push(frame);
        if opened { Ok(false) } else { self.end(cells) }
    }

    /// Reads the end of the element read last. `true` when it ends a row.
    fn end(&mut self, cells: &mut Cells<'_>) -> Result<bool, Halt> {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_sub(1);
            return Ok(false);
        }
        self.ignorable.leave();
        match self.frames.pop() {
            Some(Frame::Worksheet {
                offset,
                data: false,
                ..
            }) => {
                return Err(refuse(offset, SchemaReason::MissingElement, "sheetData").into());
            }
            Some(Frame::Row { .. }) => return Ok(true),
            Some(Frame::Cell { .. }) => self.open.finish(cells)?,
            Some(Frame::Value | Frame::Rst(rst::Frame::Text)) => {
                self.value.push("", true, |piece| cells.push(piece))?;
            }
            Some(Frame::Rst(frame)) => {
                if let Some((offset, name)) = rst::missing(&frame) {
                    return Err(refuse(offset, SchemaReason::MissingElement, name).into());
                }
            }
            _ => {}
        }
        Ok(false)
    }

    /// Reads `text`, which starts at `offset`: part of a value, or white
    /// space between elements, when it is `blank`: written as white space,
    /// not as a reference or a CDATA section that stands for it.
    fn text(
        &mut self,
        text: &str,
        blank: bool,
        offset: u64,
        cells: &mut Cells<'_>,
    ) -> Result<bool, Halt> {
        // A value takes any text; none stands before or after the root,
        // where the XML reader refuses text itself.
        let name = match self.frames.last() {
            _ if self.skipped > 0 => return Ok(false),
            Some(Frame::Value | Frame::Rst(rst::Frame::Text)) => {
                self.value.push(text, false, |piece| cells.push(piece))?;
                return Ok(false);
            }
            Some(Frame::SheetData) => "sheetData",
            Some(Frame::Row { .. }) => "row",
            Some(Frame::Cell { .. }) => "c",
            Some(Frame::Rst(rst::Frame::Rst { .. })) => "is",
            Some(Frame::Rst(rst::Frame::Run { .. })) => "r",
            Some(Frame::Worksheet { .. }) | None => "worksheet",
        };
        if blank {
            return Ok(false);
        }
        Err(refuse(offset, SchemaReason::UnexpectedText, name).into())
    }
}
