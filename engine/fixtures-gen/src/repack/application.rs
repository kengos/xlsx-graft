//! [`Application`]: the rewriter of `docProps/app.xml` that `repack` uses.

use std::io::{BufRead, Write};
use xlsx_graft_write::{PartRewriter, WriteError};
use xlsx_graft_xml::events::{BytesText, Event};
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

/// What the rewritten part names as the application.
pub(super) const NAME: &str = "xlsx-graft";

/// Writes `docProps/app.xml` again with the text of its `Application`
/// element replaced by [`NAME`] and everything else as it was. Only the
/// first text event inside the element is replaced (its text is one event
/// unless it holds references or a CDATA section), and an empty element,
/// `<Application/>`, is left as it is.
pub(super) struct Application;

impl PartRewriter for Application {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut rewriter = XmlRewriter::new(original, out, &XmlLimits::default());
        let mut buf = Vec::new();
        let mut inside = false;
        while let Some(event) = rewriter.next(&mut buf)? {
            match event {
                Event::Start(start) => {
                    inside = start.local_name().as_ref() == "Application";
                    rewriter.write(Event::Start(start))?;
                }
                Event::Text(_) if inside => rewriter.write(Event::Text(BytesText::new(NAME)))?,
                event => {
                    inside = false;
                    rewriter.write(event)?;
                }
            }
        }
        Ok(())
    }
}
