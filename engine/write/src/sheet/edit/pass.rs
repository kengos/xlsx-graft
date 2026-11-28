//! Writing an element of the part as it is: the rest of an element read, with
//! or without writing it, and an element whose start tag and first events
//! are already read.

use super::apply::Rw;
use super::wrapped::Held;
use crate::error::WriteError;
use xlsx_graft_xml::events::{BytesStart, Event};

/// Reads events until `depth` elements are closed, writing them when `keep`.
///
/// # Errors
///
/// The error of the reader or the writer.
pub(super) fn through(rw: &mut Rw<'_>, mut depth: usize, keep: bool) -> Result<(), WriteError> {
    let mut buf = Vec::new();
    while depth > 0
        && let Some(event) = rw.next(&mut buf)?
    {
        depth = match &event {
            Event::Start(_) => depth.saturating_add(1),
            Event::End(_) => depth.saturating_sub(1),
            _ => depth,
        };
        if keep {
            rw.write(event)?;
        }
    }
    Ok(())
}

/// Writes the element `start` as it is, then `held` and the rest of the
/// element.
pub(super) fn copy(
    rw: &mut Rw<'_>,
    start: BytesStart<'static>,
    opened: bool,
    held: Held,
) -> Result<(), WriteError> {
    let event = if opened {
        Event::Start(start)
    } else {
        Event::Empty(start)
    };
    rw.write(event)?;
    for event in held.events {
        rw.write(event)?;
    }
    through(rw, held.depth, true)
}
