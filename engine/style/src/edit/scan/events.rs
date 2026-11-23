//! The events of a styles part, read one at a time as values that own their
//! bytes: an element with its resolved namespace, name, attributes and the
//! bytes it was written in, and the text and the other events as written.
//! The `numFmtId` attribute (with no prefix) of every element read, at any
//! depth and in any namespace (`numFmt`, `xf`, `dxf`, an element of an
//! extension), is held as the greatest seen.

use crate::edit::StyleError;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlReader, XmlVersion};

/// The namespaces of SpreadsheetML, Transitional and Strict: the ones a
/// `styleSheet` may be in.
const NAMESPACES: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// A start tag or an empty element.
#[derive(Debug)]
pub(super) struct Tag {
    /// The element's namespace when it is one of [`NAMESPACES`].
    pub(super) ns: Option<&'static str>,
    /// The local name.
    pub(super) local: Vec<u8>,
    /// The name as written, with its prefix.
    pub(super) name: Vec<u8>,
    /// The attributes: the name as written and the value with its
    /// references resolved and white space normalized.
    pub(super) attributes: Vec<(Vec<u8>, String)>,
    /// The tag as written, from `<` to `>`.
    pub(super) raw: Vec<u8>,
    /// Where the tag starts.
    pub(super) offset: u64,
}

impl Tag {
    /// The value of the attribute written `name`.
    pub(super) fn attribute(&self, name: &str) -> Option<&str> {
        (self.attributes.iter())
            .find(|(held, _)| held == name.as_bytes())
            .map(|(_, value)| value.as_str())
    }

    /// The prefix of the name, without the colon; empty when it has none.
    pub(super) fn prefix(&self) -> Vec<u8> {
        let colon = self.name.iter().position(|&b| b == b':');
        colon.map_or_else(Vec::new, |at| self.name[..at].to_vec())
    }

    /// `xml_schema_invalid` at the tag, saying an element stands where the
    /// schema has none.
    pub(super) fn unexpected(&self) -> StyleError {
        refuse(self.offset, SchemaReason::UnexpectedElement, &self.name)
    }
}

/// One event of the part.
#[derive(Debug)]
pub(super) enum Ev {
    /// A start tag.
    Start(Tag),
    /// An empty element.
    Empty(Tag),
    /// An end tag, with the name as written.
    End(Vec<u8>),
    /// Text, a CDATA section or a reference, as written; `blank` when it is
    /// white space alone.
    Text { blank: bool, raw: Vec<u8> },
    /// A comment or a processing instruction, as written, or the
    /// declaration (no bytes).
    Other(Vec<u8>),
}

/// A styles part read event by event.
pub(super) struct Walk<'a> {
    reader: XmlReader<'a>,
    buf: Vec<u8>,
    /// The namespace of the root element, which the elements of the styles
    /// part are in; empty until the root is read.
    pub(super) namespace: &'static str,
    /// The greatest `numFmtId` attribute of the elements read so far.
    pub(super) max_id: u32,
}

impl<'a> Walk<'a> {
    /// A walk over `bytes` under `limits`.
    pub(super) fn open(bytes: &'a [u8], limits: &XmlLimits) -> Result<Self, StyleError> {
        Ok(Self {
            reader: XmlReader::open(bytes, limits)?,
            buf: Vec::new(),
            namespace: "",
            max_id: 0,
        })
    }

    /// The next event; `None` at the end of the part.
    pub(super) fn next(&mut self) -> Result<Option<Ev>, StyleError> {
        let Self {
            reader,
            buf,
            max_id,
            ..
        } = self;
        let event = reader.read_event_into(buf)?;
        let offset = reader.offset();
        Ok(Some(match event {
            Event::Start(start) => Ev::Start(tag(reader, max_id, &start, offset, b">")),
            Event::Empty(start) => Ev::Empty(tag(reader, max_id, &start, offset, b"/>")),
            Event::End(end) => Ev::End(end.name().as_ref().as_bytes().to_vec()),
            Event::Text(text) => Ev::Text {
                blank: text.xml10_content().bytes().all(is_space),
                raw: text.as_bytes().to_vec(),
            },
            Event::CData(data) => Ev::Text {
                blank: data.is_empty(),
                raw: join(&[b"<![CDATA[", data.as_bytes(), b"]]>"]),
            },
            Event::GeneralRef(reference) => Ev::Text {
                blank: false,
                raw: join(&[b"&", reference.as_bytes(), b";"]),
            },
            Event::Comment(comment) => Ev::Other(join(&[b"<!--", comment.as_bytes(), b"-->"])),
            Event::PI(pi) => Ev::Other(join(&[b"<?", pi.as_bytes(), b"?>"])),
            Event::Eof => return Ok(None),
            _ => Ev::Other(Vec::new()),
        }))
    }

    /// `xml_schema_invalid` at the event read last, saying text stands in
    /// the element `name`, which has none.
    pub(super) fn unexpected_text(&self, name: &[u8]) -> StyleError {
        refuse(self.reader.offset(), SchemaReason::UnexpectedText, name)
    }

    /// Skips the element just started (opened when `opened`, else empty): it
    /// and what is in it are read, and nothing of them is kept.
    pub(super) fn skip(&mut self, opened: bool) -> Result<(), StyleError> {
        let mut depth = usize::from(opened);
        while depth > 0
            && let Some(event) = self.next()?
        {
            match event {
                Ev::Start(_) => depth += 1,
                Ev::End(_) => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    /// Skips the element just started, which is not in the namespace of the
    /// styles part, as [`skip`](Self::skip) does; but an element inside it
    /// that is one of the lists an edit adds to (`numFmts`, `fonts`, `fills`,
    /// `borders`, `cellXfs`) in that namespace is refused, as the numbers of
    /// its items could not be known.
    pub(super) fn skip_foreign(&mut self, opened: bool) -> Result<(), StyleError> {
        let mut depth = usize::from(opened);
        while depth > 0
            && let Some(event) = self.next()?
        {
            match event {
                Ev::Start(inner) | Ev::Empty(inner) if self.is_list(&inner) => {
                    return Err(inner.unexpected());
                }
                Ev::Start(_) => depth += 1,
                Ev::End(_) => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    /// Whether `tag` is one of the lists an edit adds to, in the namespace of
    /// the styles part.
    fn is_list(&self, tag: &Tag) -> bool {
        let named = matches!(
            tag.local.as_slice(),
            b"numFmts" | b"fonts" | b"fills" | b"borders" | b"cellXfs"
        );
        named && tag.ns == Some(self.namespace)
    }

    /// Reads the element `tag` and what is in it, and gives the bytes they
    /// were written in.
    pub(super) fn raw(&mut self, tag: Tag, opened: bool) -> Result<Vec<u8>, StyleError> {
        let mut bytes = tag.raw;
        let mut depth = usize::from(opened);
        while depth > 0
            && let Some(event) = self.next()?
        {
            match event {
                Ev::Start(inner) => {
                    depth += 1;
                    bytes.extend_from_slice(&inner.raw);
                }
                Ev::Empty(inner) => bytes.extend_from_slice(&inner.raw),
                Ev::End(name) => {
                    depth -= 1;
                    bytes.extend_from_slice(&join(&[b"</", &name, b">"]));
                }
                Ev::Text { raw, .. } | Ev::Other(raw) => bytes.extend_from_slice(&raw),
            }
        }
        Ok(bytes)
    }
}

/// The tag `start`, an element starting at `offset` and ending with `close`;
/// and notes its `numFmtId` in `max_id` when it has one.
fn tag(
    reader: &XmlReader<'_>,
    max_id: &mut u32,
    start: &BytesStart<'_>,
    offset: u64,
    close: &[u8],
) -> Tag {
    let (resolved, local) = reader.resolve_element(start.name());
    let ns = match resolved {
        ResolveResult::Bound(Namespace(ns)) => NAMESPACES.into_iter().find(|&n| n == ns),
        _ => None,
    };
    let attributes: Vec<(Vec<u8>, String)> = start
        .attributes()
        .flatten()
        .filter_map(|a| {
            let value = a.normalized_value(XmlVersion::Implicit1_0).ok()?;
            Some((a.key.as_ref().as_bytes().to_vec(), value.into_owned()))
        })
        .collect();
    let tag = Tag {
        ns,
        local: local.as_ref().as_bytes().to_vec(),
        name: start.name().as_ref().as_bytes().to_vec(),
        attributes,
        raw: join(&[b"<", start.as_bytes(), close]),
        offset,
    };
    if let Some(id) = tag.attribute("numFmtId").and_then(uint) {
        *max_id = (*max_id).max(id);
    }
    tag
}

/// The bytes of `parts`, one after the other.
fn join(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// The `xsd:unsignedInt` `text` is, its white space collapsed.
pub(super) fn uint(text: &str) -> Option<u32> {
    text.trim_matches([' ', '\t', '\n', '\r']).parse().ok()
}

/// Whether `byte` is XML white space.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

/// `xml_schema_invalid` at `offset`, for `reason`, naming `name`.
pub(in crate::edit) fn refuse(offset: u64, reason: SchemaReason, name: &[u8]) -> StyleError {
    StyleError::Invalid(Diagnostic::XmlSchemaInvalid {
        offset,
        reason,
        name: Echo::from_utf8_lossy(name),
    })
}
