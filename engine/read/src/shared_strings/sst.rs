//! [`read`]: the shared strings part read as a stream into a
//! [`StringTable`], its shape held to the schema as the crate root says.
//!
//! The elements read are a stack of frames, each held to the schema shape
//! (`shape.rs`). A child that is skipped (`extLst`, `rPr`, `rPh`,
//! `phoneticPr`) is passed over with all it holds, as is an element in a
//! namespace `mc:Ignorable` names. The value of each `t` arrives in pieces
//! ([`Xstring`]).

mod shape;

use super::escape::Xstring;
use super::table::{Builder, StringTable};
use super::{Halt, rst};
use crate::ReadLimits;
use crate::compat::Ignorable;
use crate::schema::{NAMESPACES, is_blank, namespace, resolve};
use shape::{Child, Frame, attributes, child, refuse};
use std::io::BufRead;
use xlsx_graft_diagnostics::SchemaReason;
use xlsx_graft_spool::SpoolFactory;
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlStreamError, XmlStreamReader};

/// Where the walk is.
struct Walk<'a> {
    table: Builder<'a>,
    /// The namespace of the root, which every element shares.
    namespace: Option<&'static str>,
    frames: Vec<Frame>,
    /// The namespaces the `mc:Ignorable` of the elements read name.
    ignorable: Ignorable,
    /// How many elements deep the walk is inside a skipped one.
    skipped: u64,
    /// The value of the `t` being read.
    value: Xstring,
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
        ignorable: Ignorable::new(&NAMESPACES),
        skipped: 0,
        value: Xstring::default(),
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
            Event::Text(text) => {
                let text = text.xml10_content();
                walk.text(&text, is_blank(&text), offset)?;
            }
            Event::CData(data) => walk.text(&data.xml10_content(), data.is_empty(), offset)?,
            Event::GeneralRef(reference) => walk.text(&resolve(&reference), false, offset)?,
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
        let namespace = namespace(&resolved);
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
        attributes(&frame, start, reader, &mut self.ignorable, offset)?;
        match frame {
            Frame::Rst(rst::Frame::Rst { .. }) => self.table.begin()?,
            Frame::Rst(rst::Frame::Text) => self.value.begin(),
            Frame::Sst { .. } | Frame::Rst(rst::Frame::Run { .. }) => {}
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
        self.ignorable.leave();
        let table = &mut self.table;
        match self.frames.pop() {
            Some(Frame::Rst(rst::Frame::Text)) => self.value.push("", true, |p| table.text(p))?,
            Some(Frame::Rst(rst::Frame::Rst { .. })) => table.end()?,
            Some(Frame::Rst(frame)) => {
                if let Some((offset, name)) = rst::missing(&frame) {
                    return Err(refuse(offset, SchemaReason::MissingElement, name));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Reads `text`, which starts at `offset`: part of a `t`'s value, or
    /// white space between elements, when it is `blank`: written as white
    /// space, not as a reference or a CDATA section that stands for it.
    fn text(&mut self, text: &str, blank: bool, offset: u64) -> Result<(), Halt> {
        // A `t` takes any text, so a frame that refuses it is one of the
        // others; none stands before or after the root, where the XML
        // reader refuses text itself.
        let name = match self.frames.last() {
            _ if self.skipped > 0 => return Ok(()),
            Some(Frame::Rst(rst::Frame::Text)) => {
                let table = &mut self.table;
                return self.value.push(text, false, |piece| table.text(piece));
            }
            Some(Frame::Sst { .. }) => "sst",
            Some(Frame::Rst(rst::Frame::Rst { .. })) => "si",
            Some(Frame::Rst(rst::Frame::Run { .. })) | None => "r",
        };
        if blank {
            return Ok(());
        }
        Err(refuse(offset, SchemaReason::UnexpectedText, name))
    }
}
