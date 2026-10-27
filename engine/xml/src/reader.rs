//! [`XmlReader`]: quick-xml behind the size, encoding and character checks
//! of [`XmlReader::open`] and the per-event guard.

use crate::guard::{Guard, Stop};
use crate::{XmlLimits, chars, encoding, malformed};
use quick_xml::NsReader;
use quick_xml::events::Event;
use quick_xml::name::{LocalName, QName, ResolveResult};
use std::borrow::Cow;
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostic;

/// The quick-xml reader under an [`XmlReader`]. It reads from a cursor and
/// copies each event into the caller's buffer even for borrowed input: the
/// UTF-16 case owns its decoded text, and borrowing events from that would
/// make the reader refer to itself.
type Inner<'a> = NsReader<Cursor<Cow<'a, [u8]>>>;

/// Reads one part's XML, event by event, refusing what a package's XML may
/// not hold. The crate root lists what it checks.
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
    reader: Inner<'a>,
    guard: Guard,
    /// Where the event read last starts.
    offset: u64,
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
    ///
    /// # Errors
    ///
    /// `xml_too_large` when `bytes` is longer than `max_bytes`,
    /// `xml_encoding_unsupported` when it is not UTF-8 or UTF-16 or does not
    /// decode, and `xml_malformed` when it holds a character XML does not
    /// allow.
    pub fn open(bytes: &'a [u8], limits: &XmlLimits) -> Result<Self, Diagnostic> {
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if size > limits.max_bytes.value {
            return Err(Diagnostic::XmlTooLarge {
                size,
                limit: limits.max_bytes,
            });
        }
        let text = encoding::decode(bytes)?;
        if let Some((offset, c)) = chars::first_invalid(&text) {
            let offset = u64::try_from(offset).unwrap_or(u64::MAX);
            return Err(malformed(offset, &chars::describe(c)));
        }
        // quick-xml drops a UTF-8 byte order mark at the start of its input,
        // so a second one, after the mark `decode` removed, would vanish
        // instead of counting as content before the root.
        if text.starts_with('\u{feff}') {
            return Err(malformed(0, "the XML has content outside the root element"));
        }
        let bytes = match text {
            Cow::Borrowed(text) => Cow::Borrowed(text.as_bytes()),
            Cow::Owned(text) => Cow::Owned(text.into_bytes()),
        };
        let mut reader = NsReader::from_reader(Cursor::new(bytes));
        reader.config_mut().check_comments = true;
        // The guard enforces `max_namespaces` with its own diagnostic.
        reader.resolver_mut().set_max_namespace_bindings(usize::MAX);
        Ok(Self {
            reader,
            guard: Guard::new(*limits),
            offset: 0,
            failed: None,
        })
    }

    /// The next event, read into `buf` after clearing it; [`Event::Eof`]
    /// once the document has ended well.
    ///
    /// # Errors
    ///
    /// The first problem found, as a diagnostic the caller pushes with the
    /// part it read: `xml_malformed`, `xml_doctype`, `xml_too_deep`,
    /// `xml_too_many_attributes` or `xml_too_many_namespaces`. Every call after it returns it again.
    pub fn read_event_into<'b>(&mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>, Diagnostic> {
        if let Some(failed) = &self.failed {
            return Err(failed.clone());
        }
        buf.clear();
        let offset = self.reader.buffer_position();
        self.offset = offset;
        let limits = self.guard.limits();
        let stop = match self.reader.read_event_into(buf) {
            Err(error) => malformed(self.reader.error_position(), &error.to_string()),
            Ok(event) => match self.guard.check(&event) {
                Ok(()) => return Ok(event),
                Err(Stop::Malformed(detail)) => malformed(offset, &detail),
                Err(Stop::Doctype) => Diagnostic::XmlDoctype { offset },
                Err(Stop::TooManyAttributes(count)) => Diagnostic::XmlTooManyAttributes {
                    offset,
                    count,
                    limit: limits.max_attributes,
                },
                Err(Stop::TooManyNamespaces(count)) => Diagnostic::XmlTooManyNamespaces {
                    offset,
                    count,
                    limit: limits.max_namespaces,
                },
                Err(Stop::TooDeep { opened }) => Diagnostic::XmlTooDeep {
                    offset,
                    depth: deepest(&mut self.reader, limits.max_depth.value, opened),
                    limit: limits.max_depth,
                },
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
        self.offset
    }

    /// The namespace and local name of the element name `name`, under the
    /// bindings in scope at the event read last. A prefix with no binding
    /// resolves to [`ResolveResult::Unknown`]; refusing it is the caller's
    /// call.
    #[must_use]
    pub fn resolve_element<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolver().resolve_element(name)
    }

    /// The namespace and local name of the attribute name `name`, as for
    /// [`resolve_element`](Self::resolve_element). An unprefixed attribute is
    /// in no namespace, whatever the default.
    #[must_use]
    pub fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolver().resolve_attribute(name)
    }
}

/// How deep the document nests, reading on from an element one past `limit`
/// (left open when `opened`). Only counts: it stops at the end or at the
/// first error, so a malformed rest reports the deepest before it.
fn deepest(reader: &mut Inner<'_>, limit: u64, opened: bool) -> u64 {
    let mut deepest = limit.saturating_add(1);
    let mut depth = if opened { deepest } else { limit };
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(_)) => {
                depth = depth.saturating_add(1);
                deepest = deepest.max(depth);
            }
            Ok(Event::Empty(_)) => deepest = deepest.max(depth.saturating_add(1)),
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Eof) | Err(_) => return deepest,
            Ok(_) => {}
        }
    }
}
