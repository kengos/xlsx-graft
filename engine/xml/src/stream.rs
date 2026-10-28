//! [`XmlStreamReader`]: quick-xml over a [`BufRead`], behind the encoding
//! and character checks of its [`Source`] and the per-event guard. The
//! in-memory [`XmlReader`](crate::XmlReader) is this reader over a slice.

use crate::encoding::check_decl;
use crate::guard::Guard;
use crate::{XmlLimits, malformed};
use quick_xml::NsReader;
use quick_xml::events::Event;
use quick_xml::name::{LocalName, Namespace, QName, ResolveResult};
use source::Source;
use std::io::BufRead;
use xlsx_graft_diagnostics::Diagnostic;

mod decoder;
mod deepest;
mod error;
mod source;

pub use error::XmlStreamError;

/// Reads one part's XML from a [`BufRead`], event by event, refusing what a
/// package's XML may not hold, without holding the part in memory. The
/// crate root lists what it checks and where it differs from
/// [`XmlReader`](crate::XmlReader).
///
/// Each event is read into a buffer the caller owns, as quick-xml's
/// `read_event_into` does, so the reader stays free to resolve the
/// namespaces of the names in it. That buffer holds one event at a time,
/// and that event whole: one long text can be as large as the part, which
/// only the zip limits on an entry bound.
///
/// A part is read well only once [`Event::Eof`] comes back: what follows
/// the root element, the end of the encoding, and, through a zip entry, its
/// CRC are checked only when the reader reads that far. A caller that stops
/// at the root's end tag has not checked them.
///
/// # Examples
///
/// ```
/// use xlsx_graft_xml::events::Event;
/// use xlsx_graft_xml::name::{Namespace, ResolveResult};
/// use xlsx_graft_xml::{XmlLimits, XmlStreamReader};
///
/// let xml: &[u8] = br#"<sst xmlns="urn:s"><si><t>a</t></si></sst>"#;
/// let mut reader = XmlStreamReader::new(xml, &XmlLimits::default());
/// let mut buf = Vec::new();
/// let Event::Start(sst) = reader.read_event_into(&mut buf)? else {
///     panic!("expected a start tag");
/// };
/// let (namespace, local) = reader.resolve_element(sst.name());
/// assert_eq!(namespace, ResolveResult::Bound(Namespace("urn:s")));
/// assert_eq!(local.as_ref(), "sst");
/// while !matches!(reader.read_event_into(&mut buf)?, Event::Eof) {}
/// # Ok::<(), xlsx_graft_xml::XmlStreamError>(())
/// ```
#[derive(Debug)]
pub struct XmlStreamReader<R> {
    reader: NsReader<Source<R>>,
    guard: Guard,
    /// Where the event read last starts.
    offset: u64,
    /// The error that stopped the reader, returned by every later call.
    failed: Option<XmlStreamError>,
}

impl<R: BufRead> XmlStreamReader<R> {
    /// A reader over `reader`, a part not yet read, under `limits`.
    ///
    /// Nothing is read until the first event: a problem with the encoding
    /// comes back from [`read_event_into`](Self::read_event_into).
    /// `max_bytes` does not apply, since the part is never held whole; the
    /// zip limits on what an entry inflates to bound how much is read.
    pub fn new(reader: R, limits: &XmlLimits) -> Self {
        let mut reader = NsReader::from_reader(Source::new(reader));
        reader.config_mut().check_comments = true;
        // The guard enforces `max_namespaces` with its own diagnostic.
        reader.resolver_mut().set_max_namespace_bindings(usize::MAX);
        Self {
            reader,
            guard: Guard::new(*limits),
            offset: 0,
            failed: None,
        }
    }

    /// The next event, read into `buf` after clearing it; [`Event::Eof`]
    /// once the document has ended well, which is when the part has been
    /// read well.
    ///
    /// # Errors
    ///
    /// [`XmlStreamError::Invalid`] with the first problem found, as a
    /// diagnostic the caller pushes with the part it read:
    /// `xml_encoding_unsupported`, `xml_malformed`, `xml_doctype`,
    /// `xml_too_deep`, `xml_too_many_attributes`, `xml_too_many_namespaces`
    /// or `xml_namespace_too_long`. [`XmlStreamError::Io`] when `R` fails.
    /// Every call after either returns it again.
    pub fn read_event_into<'b>(
        &mut self,
        buf: &'b mut Vec<u8>,
    ) -> Result<Event<'b>, XmlStreamError> {
        if let Some(failed) = &self.failed {
            return Err(failed.again());
        }
        buf.clear();
        let offset = self.reader.buffer_position();
        self.offset = offset;
        let error = match self.reader.read_event_into(buf) {
            Err(quick_xml::Error::Io(error)) => {
                XmlStreamError::from_source(self.reader.get_ref().decoder(), &error)
            }
            Err(error) => malformed(self.reader.error_position(), &error.to_string()).into(),
            Ok(event) => match self.check(offset, &event) {
                Ok(()) => return Ok(event),
                Err(diagnostic) => diagnostic.into(),
            },
        };
        self.failed = Some(error.again());
        Err(error)
    }

    /// Checks `event`, which starts at `offset`: its declaration against the
    /// byte order mark when it is the first, then the guard.
    fn check(&mut self, offset: u64, event: &Event<'_>) -> Result<(), Diagnostic> {
        if let Event::Decl(decl) = event
            && offset == 0
        {
            check_decl(decl, self.reader.get_ref().decoder().found())?;
        }
        let limits = self.guard.limits();
        self.guard
            .check(event)
            .map_err(|stop| stop.diagnostic(offset, &limits))
    }

    /// Where the event read last starts, as [`XmlReader::offset`] says:
    /// bytes of the part's text read as UTF-8, after any byte order mark.
    ///
    /// [`XmlReader::offset`]: crate::XmlReader::offset
    #[must_use]
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// The reader the part is read from, to ask it why it failed after an
    /// [`XmlStreamError::Io`]. There is no `get_mut`: reading from it would
    /// skip part of the XML.
    #[must_use]
    pub fn get_ref(&self) -> &R {
        self.reader.get_ref().get_ref()
    }

    /// The namespace and local name of the element name `name`, as
    /// [`XmlReader::resolve_element`](crate::XmlReader::resolve_element)
    /// resolves it.
    #[must_use]
    pub fn resolve_element<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolver().resolve_element(name)
    }

    /// The namespace and local name of the attribute name `name`, as
    /// [`XmlReader::resolve_attribute`](crate::XmlReader::resolve_attribute)
    /// resolves it.
    #[must_use]
    pub fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolver().resolve_attribute(name)
    }

    /// The namespace `prefix` is bound to, as
    /// [`XmlReader::resolve_prefix`](crate::XmlReader::resolve_prefix)
    /// resolves it.
    #[must_use]
    pub fn resolve_prefix(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() || prefix.contains(':') {
            return None;
        }
        // quick-xml resolves a prefix only as part of a name.
        let name = format!("{prefix}:_");
        match self.resolve_attribute(QName(&name)).0 {
            ResolveResult::Bound(Namespace(namespace)) => Some(namespace),
            _ => None,
        }
    }
}
