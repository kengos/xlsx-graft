//! What a child of the root is to the placer: a child of the table, an
//! `mc:AlternateContent` that wraps one (looked ahead to the first element
//! of its first `mc:Choice`), or another element.

use super::apply::Rw;
use super::name::{child_kind, is_markup};
use crate::error::WriteError;
use crate::sheet::RootChild;
use xlsx_graft_xml::events::{BytesStart, Event};

/// The most events looked ahead inside an `mc:AlternateContent`: past them
/// the wrapper is an [`RootChild::Other`]. The events are held until the
/// kind is known, so this bounds the memory a hostile part takes.
const MAX_EVENTS: usize = 256;

/// The events read after a child's start tag to tell what it is, which the
/// caller writes as they were, and how many elements are open after them
/// (the child itself counts).
pub(super) struct Held {
    pub(super) events: Vec<Event<'static>>,
    pub(super) depth: usize,
}

/// What the element `start` is as a child of the root, with the events read
/// to know it. `opened` is whether it has an end tag of its own.
///
/// # Errors
///
/// The error of the reader.
pub(super) fn classify(
    rw: &mut Rw<'_>,
    start: &BytesStart<'_>,
    opened: bool,
) -> Result<(RootChild, Held), WriteError> {
    let held = Held {
        events: Vec::new(),
        depth: usize::from(opened),
    };
    if let Some(kind) = child_kind(&rw.resolve_element(start.name())) {
        return Ok((RootChild::Table(kind), held));
    }
    if opened && is_markup(&rw.resolve_element(start.name()), "AlternateContent") {
        return look_ahead(rw, held);
    }
    Ok((RootChild::Other, held))
}

/// Reads the events of an `mc:AlternateContent` up to the start tag of the
/// first element in its first `mc:Choice`: that element's kind, in the
/// table, is what the wrapper holds.
fn look_ahead(rw: &mut Rw<'_>, mut held: Held) -> Result<(RootChild, Held), WriteError> {
    let mut buf = Vec::new();
    let mut kind = RootChild::Other;
    while held.events.len() < MAX_EVENTS
        && let Some(event) = rw.next(&mut buf)?
    {
        let verdict = match &event {
            Event::Start(start) => verdict(rw, held.depth, start, true),
            Event::Empty(start) => verdict(rw, held.depth, start, false),
            Event::End(_) => Some(RootChild::Other),
            _ => None,
        };
        held.depth = match &event {
            Event::Start(_) => held.depth.saturating_add(1),
            Event::End(_) => held.depth.saturating_sub(1),
            _ => held.depth,
        };
        held.events.push(event.into_owned());
        if let Some(found) = verdict {
            kind = found;
            break;
        }
    }
    Ok((kind, held))
}

/// What the element `start` settles when it comes at `depth` inside the
/// wrapper: nothing for the first `mc:Choice` (the look-ahead goes on into
/// it), the kind of the first element inside it, and `Other` for anything
/// else.
fn verdict(rw: &Rw<'_>, depth: usize, start: &BytesStart<'_>, opened: bool) -> Option<RootChild> {
    let resolved = rw.resolve_element(start.name());
    match depth {
        1 if opened && is_markup(&resolved, "Choice") => None,
        2 => Some(child_kind(&resolved).map_or(RootChild::Other, RootChild::Wrapped)),
        _ => Some(RootChild::Other),
    }
}
