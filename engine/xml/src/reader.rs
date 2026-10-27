//! [`XmlReader`]: an [`XmlStreamReader`] over a part already in memory,
//! behind the size check of [`XmlReader::open`], reading on past a too-deep
//! element to report the deepest nesting.

use crate::{XmlLimits, XmlStreamReader};
use quick_xml::events::Event;
use quick_xml::name::{LocalName, QName, ResolveResult};
use xlsx_graft_diagnostics::Diagnostic;

/// Reads one part's XML, already in memory, event by event, refusing what a
/// package's XML may not hold. The crate root lists what it checks; it
/// reads the part exactly as an [`XmlStreamReader`] does, so the two give
/// a part the same diagnostics, but for its size and the depth
/// `xml_too_deep` reports.
///
/// Each event is read into a buffer the caller owns, as quick-xml's
/// `read_event_into` does, so the reader stays free to resolve the
/// namespaces of the names in it.
///
/// # Examples
///
/// ```
/// use xlsx_graft_xml::events::Event;
/// use xlsx_graft_xml::name::{Namespace, ResolveResult};
/// use xlsx_graft_xml::{XmlLimits, XmlReader};
///
/// let xml = br#"<Types xmlns="urn:t"/>"#;
/// let mut reader = XmlReader::open(xml, &XmlLimits::default())?;
/// let mut buf = Vec::new();
/// let Event::Empty(types) = reader.read_event_into(&mut buf)? else {
///     panic!("expected an empty element");
/// };
/// let (namespace, local) = reader.resolve_element(types.name());
/// assert_eq!(namespace, ResolveResult::Bound(Namespace("urn:t")));
/// assert_eq!(local.as_ref(), "Types");
/// assert!(matches!(reader.read_event_into(&mut buf)?, Event::Eof));
/// # Ok::<(), xlsx_graft_diagnostics::Diagnostic>(())
/// ```
#[derive(Debug)]
pub struct XmlReader<'a> {
    reader: XmlStreamReader<&'a [u8]>,
    /// The diagnostic that stopped the reader, returned by every later call.
    failed: Option<Diagnostic>,
}

impl<'a> XmlReader<'a> {
    /// A reader over `bytes`, a whole part already in memory, under
    /// `limits`.
    ///
    /// `max_bytes` is checked against `bytes` as given: by then they are in
    /// memory. A caller inflating a part stops reading at `max_bytes + 1`
    /// bytes and reports `xml_too_large` itself, with the entry's declared
    /// size when that is larger, so a zip bomb never fills memory first.
    /// Nothing else is checked yet: a problem with the encoding or a
    /// character comes back from [`read_event_into`](Self::read_event_into)
    /// once reading reaches it.
    ///
    /// # Errors
    ///
    /// `xml_too_large` when `bytes` is longer than `max_bytes`.
    pub fn open(bytes: &'a [u8], limits: &XmlLimits) -> Result<Self, Diagnostic> {
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if size > limits.max_bytes.value {
            return Err(Diagnostic::XmlTooLarge {
                size,
                limit: limits.max_bytes,
            });
        }
        Ok(Self {
            reader: XmlStreamReader::new(bytes, limits),
            failed: None,
        })
    }

    /// The next event, read into `buf` after clearing it; [`Event::Eof`]
    /// once the document has ended well.
    ///
    /// # Errors
    ///
    /// The first problem found, as a diagnostic the caller pushes with the
    /// part it read: `xml_encoding_unsupported`, `xml_malformed`,
    /// `xml_doctype`, `xml_too_deep` (with the deepest nesting in the part),
    /// `xml_too_many_attributes` or `xml_too_many_namespaces`. Every call
    /// after it returns it again.
    pub fn read_event_into<'b>(&mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>, Diagnostic> {
        if let Some(failed) = &self.failed {
            return Err(failed.clone());
        }
        let stop = match self.reader.read_event_into(buf) {
            Ok(event) => return Ok(event),
            Err(error) => match error.into_diagnostic(self.reader.offset()) {
                Diagnostic::XmlTooDeep { offset, limit, .. } => Diagnostic::XmlTooDeep {
                    offset,
                    depth: self.reader.deepest(),
                    limit,
                },
                diagnostic => diagnostic,
            },
        };
        self.failed = Some(stop.clone());
        Err(stop)
    }

    /// Where the event read last starts: a byte offset in the part's text,
    /// read as UTF-8 after any byte order mark, as every `xml_*` code's
    /// `offset` is. A caller refusing what it finds in an event reports it
    /// here. After an `Err` the diagnostic's own offset is the one to use.
    #[must_use]
    pub fn offset(&self) -> u64 {
        self.reader.offset()
    }

    /// The namespace and local name of the element name `name`, under the
    /// bindings in scope at the event read last. A prefix with no binding
    /// resolves to [`ResolveResult::Unknown`]; refusing it is the caller's
    /// call.
    #[must_use]
    pub fn resolve_element<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolve_element(name)
    }

    /// The namespace and local name of the attribute name `name`, as for
    /// [`resolve_element`](Self::resolve_element). An unprefixed attribute is
    /// in no namespace, whatever the default.
    #[must_use]
    pub fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolve_attribute(name)
    }
}
