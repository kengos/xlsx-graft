//! [`AppRewriter`]: the app part (`docProps/app.xml`) without the elements
//! that list the sheets, `HeadingPairs` and `TitlesOfParts` (ECMA-376-1:2016
//! §22.2.2), which a reader of the workbook does not need and which would
//! name sheets that are no longer there.

use super::ns::EXTENDED;
use crate::{PartRewriter, WriteError, WriteLimits};
use std::io::{BufRead, Write};
use xlsx_graft_xml::events::Event;
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

/// The elements of the root left out.
const TITLES: [&str; 2] = ["HeadingPairs", "TitlesOfParts"];

/// Writes the app part without its sheet lists.
pub(super) struct AppRewriter {
    /// The limits the part is read under.
    pub(super) limits: WriteLimits,
}

impl PartRewriter for AppRewriter {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        rewrite(original, out, &self.limits.xml).map(drop)
    }
}

/// Reads `original` and writes it to `out` without the children of the root
/// named in [`TITLES`], in an extended properties namespace, with
/// everything they hold. Whether there was one.
pub(super) fn rewrite(
    original: &mut dyn BufRead,
    out: &mut dyn Write,
    limits: &XmlLimits,
) -> Result<bool, WriteError> {
    let mut rewriter = XmlRewriter::new(original, out, limits);
    let mut buf = Vec::new();
    let (mut depth, mut leaving_out, mut found) = (0_usize, None, false);
    while let Some(event) = rewriter.next(&mut buf)? {
        let before = depth;
        match &event {
            Event::Start(_) => depth = depth.saturating_add(1),
            Event::End(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
        if leaving_out.is_some() {
            let closes = matches!(event, Event::End(_)) && Some(before) == leaving_out;
            leaving_out = leaving_out.filter(|_| !closes);
            continue;
        }
        let titles = match &event {
            Event::Start(start) | Event::Empty(start) if before == 1 => {
                let (namespace, local) = rewriter.resolve_element(start.name());
                let ours = EXTENDED.iter().any(|ns| {
                    matches!(namespace, ResolveResult::Bound(Namespace(bound)) if bound == *ns)
                });
                ours && TITLES.contains(&local.as_ref())
            }
            _ => false,
        };
        found |= titles;
        match &event {
            Event::Start(_) if titles => leaving_out = Some(depth),
            Event::Empty(_) if titles => {}
            _ => rewriter.write(event)?,
        }
    }
    Ok(found)
}
