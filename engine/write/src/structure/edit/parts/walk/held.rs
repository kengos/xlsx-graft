//! [`Open`], a formula element being read, and [`Held`], what it keeps of
//! its content while it is read, with the bound on that.
//!
//! The content is kept in one of three states. Only the innermost open
//! element can be `Holding`: when an element starts inside another, the
//! events the outer one holds are written and it becomes `Markup`. So what
//! the open elements keep is bounded by `MAX_FORMULA_BYTES` of text each,
//! and a part's nesting depth (256 by default) limits their number: about
//! 256 * 64 KiB = 16 MiB, plus the events of the innermost one.

use super::super::text::{MAX_FORMULA_BYTES, resolve};
use super::{Walk, put};
use std::io::{BufRead, Write};
use std::sync::Arc;
use xlsx_graft_formula::places::FormulaElement;
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlRewriter, XmlStreamError};

/// A formula element being read.
pub(super) struct Open {
    /// The row of the table it is.
    pub(super) row: &'static FormulaElement,
    /// Where it is.
    pub(super) at: Option<Arc<str>>,
    /// What it keeps of its content.
    pub(super) held: Held,
}

/// What a formula element keeps of its content.
pub(super) enum Held {
    /// Nothing is written yet. The events from its start tag on are held
    /// until the element ends, to be written as they were or replaced by one
    /// new text event.
    Holding {
        /// The events held.
        events: Vec<Event<'static>>,
        /// The text so far (text, CDATA sections and references).
        text: String,
        /// The bytes of the text and of the comments and processing
        /// instructions read so far.
        size: usize,
    },
    /// An element started inside: the events are written as they come, the
    /// text is still read to look for a sheet's name in it.
    Markup {
        /// The text so far.
        text: String,
        /// The bytes of the text and of the comments and processing
        /// instructions read so far.
        size: usize,
    },
    /// More than `MAX_FORMULA_BYTES`: the events are written as they come and
    /// nothing is held or read.
    TooLong,
}

impl Held {
    /// The state of an element that starts with `start`.
    pub(super) fn holding(start: BytesStart<'static>) -> Self {
        Self::Holding {
            events: vec![Event::Start(start)],
            text: String::new(),
            size: 0,
        }
    }

    /// An element starts inside: the events to write now (the ones held,
    /// when something was held), and the state is `Markup` unless it is
    /// `TooLong`.
    pub(super) fn child(&mut self) -> Vec<Event<'static>> {
        let Self::Holding { events, text, size } = self else {
            return Vec::new();
        };
        let (events, text, size) = (std::mem::take(events), std::mem::take(text), *size);
        *self = Self::Markup { text, size };
        events
    }

    /// Takes the event `event` that is not a tag, whose text (when it has
    /// any) is `piece` and whose size is `weight` bytes: an event of 0 bytes
    /// (an empty comment, `<!---->`) counts as one byte, since holding it
    /// takes memory.
    ///
    /// The events to write now, in order: the ones held when `self` becomes
    /// `TooLong`, then `event` itself unless it is held.
    pub(super) fn push<'e>(
        &mut self,
        event: Event<'e>,
        piece: &str,
        weight: usize,
    ) -> (Vec<Event<'static>>, Option<Event<'e>>) {
        let weight = weight.max(1);
        match self {
            Self::Holding { events, text, size } => {
                *size = size.saturating_add(weight);
                if *size > MAX_FORMULA_BYTES {
                    let held = std::mem::take(events);
                    *self = Self::TooLong;
                    return (held, Some(event));
                }
                text.push_str(piece);
                events.push(event.into_owned());
                (Vec::new(), None)
            }
            Self::Markup { text, size } => {
                *size = size.saturating_add(weight);
                if *size > MAX_FORMULA_BYTES {
                    *self = Self::TooLong;
                } else {
                    text.push_str(piece);
                }
                (Vec::new(), Some(event))
            }
            Self::TooLong => (Vec::new(), Some(event)),
        }
    }
}

impl Walk<'_> {
    /// Handles an event that is not a tag: text, a CDATA section, a
    /// reference, a comment, a processing instruction or a declaration.
    pub(super) fn other<R: BufRead, W: Write>(
        &mut self,
        rw: &mut XmlRewriter<R, W>,
        event: Event<'_>,
    ) -> Result<(), XmlStreamError> {
        let Some(top) = self.open.last_mut() else {
            return put(rw, event);
        };
        let piece = match &event {
            Event::Text(text) => text.xml10_content().into_owned(),
            Event::CData(data) => data.xml10_content().into_owned(),
            Event::GeneralRef(reference) => resolve(reference),
            _ => String::new(),
        };
        let weight = piece.len().max(raw_len(&event));
        let (held, now) = top.held.push(event, &piece, weight);
        for event in held {
            put(rw, event)?;
        }
        now.map_or(Ok(()), |event| put(rw, event))
    }
}

/// The bytes of a comment or a processing instruction; 0 for any other event.
fn raw_len(event: &Event<'_>) -> usize {
    match event {
        Event::Comment(comment) => comment.len(),
        Event::PI(instruction) => instruction.len(),
        _ => 0,
    }
}
