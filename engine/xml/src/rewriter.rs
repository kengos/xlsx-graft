//! [`XmlRewriter`]: an [`XmlStreamReader`] over a part, and a quick-xml
//! `Writer` that writes the events read, or others in their place, as UTF-8.

use crate::{XmlLimits, XmlStreamError, XmlStreamReader};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesStart, Event};
use quick_xml::name::{LocalName, QName, ResolveResult};
use std::fmt;
use std::io::{self, BufRead, Write};

/// The UTF-8 byte order mark.
const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";

/// Reads a part's XML as [`XmlStreamReader`] does, under the same limits.
/// Writes to `W` the events passed to [`write`](Self::write): those read,
/// unchanged or changed, and new ones.
///
/// For a UTF-8 part, the bytes written for events passed to `write` as
/// [`next`](Self::next) returned them equal the bytes those events were read
/// from: the declaration with its spaces and quotes, whitespace, comments,
/// the quotes, order and newlines of attributes, whether an empty element is
/// written `<a/>` or `<a />`, CDATA, processing instructions, references,
/// and a byte order mark. The one exception is whitespace after the name in
/// a closing tag, which quick-xml's reader removes from the end tag's name
/// (`trim_markup_names_in_closing_tags`): `</a >` is written as `</a>`.
///
/// The output is always UTF-8. A UTF-16 part is written as UTF-8, without
/// its byte order mark. Its declaration, if it names an encoding other than
/// `UTF-8`, is written with only the value of its `encoding` attribute
/// replaced by `UTF-8`: every other byte of the declaration (the quotes, the
/// version, `standalone`, other pseudo-attributes and the spaces) is written
/// as it was.
///
/// The caller owns the buffer an event is read into, as with
/// [`XmlStreamReader::read_event_into`], so that [`write`](Self::write) can
/// be called while the caller still borrows the event `next` returned.
///
/// # Examples
///
/// ```
/// use xlsx_graft_xml::events::Event;
/// use xlsx_graft_xml::{XmlLimits, XmlRewriter};
///
/// let part: &[u8] = b"<a x='1'><b /></a>";
/// let mut rewriter = XmlRewriter::new(part, Vec::new(), &XmlLimits::default());
/// let mut buf = Vec::new();
/// while let Some(event) = rewriter.next(&mut buf)? {
///     rewriter.write(event)?;
/// }
/// assert_eq!(rewriter.into_inner(), part);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct XmlRewriter<R, W> {
    reader: XmlStreamReader<R>,
    writer: Writer<W>,
    /// Whether `write` has been called: only the first call writes the
    /// original's UTF-8 byte order mark.
    started: bool,
}

impl<R: fmt::Debug, W> fmt::Debug for XmlRewriter<R, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // quick-xml's writer has no `Debug`, so the writer and its output
        // are not printed.
        f.debug_struct("XmlRewriter")
            .field("reader", &self.reader)
            .field("started", &self.started)
            .finish_non_exhaustive()
    }
}

impl<R: BufRead, W: Write> XmlRewriter<R, W> {
    /// A rewriter reading `original`, a part not yet read, under `limits`,
    /// and writing to `out`.
    pub fn new(original: R, out: W, limits: &XmlLimits) -> Self {
        Self {
            reader: XmlStreamReader::new(original, limits),
            writer: Writer::new(out),
            started: false,
        }
    }

    /// The next event, read into `buf` after clearing it; `None` where
    /// [`XmlStreamReader::read_event_into`] returns [`Event::Eof`]: the part
    /// was read to its end and passed every check.
    ///
    /// # Errors
    ///
    /// As [`XmlStreamReader::read_event_into`]: every call after an error
    /// returns it again.
    pub fn next<'b>(&mut self, buf: &'b mut Vec<u8>) -> Result<Option<Event<'b>>, XmlStreamError> {
        match self.reader.read_event_into(buf)? {
            Event::Eof => Ok(None),
            event => Ok(Some(event)),
        }
    }

    /// Writes `event` to the output. The first call to `write` writes the
    /// original's byte order mark first, when the original is UTF-8 with
    /// one. `next` knows whether the original has a byte order mark only
    /// once it has read the part's first bytes, so a first `write` before
    /// any `next` writes no mark. A declaration that names an encoding other
    /// than `UTF-8` (in any letter case) is written with only the value of
    /// its `encoding` attribute replaced by `UTF-8`; every other byte of it
    /// is written as it was. Everything else is written as received.
    ///
    /// # Errors
    ///
    /// The error `W` returns; or [`io::ErrorKind::InvalidData`] for a
    /// declaration the caller built whose attributes quick-xml cannot read
    /// up to its `encoding` attribute, or to its end when it has none. A
    /// declaration `next` returned never fails that way, since the reader
    /// read it with the same attribute parser. What the output holds after
    /// `write` returns an error is not specified.
    pub fn write(&mut self, event: Event<'_>) -> io::Result<()> {
        if !self.started {
            self.started = true;
            if self.reader.decoder().utf8_bom() {
                self.writer.get_mut().write_all(UTF8_BOM)?;
            }
        }
        match event {
            Event::Decl(decl) => match utf8_decl(&decl)? {
                Some(rewritten) => self.writer.write_event(Event::Decl(rewritten)),
                None => self.writer.write_event(Event::Decl(decl)),
            },
            event => self.writer.write_event(event),
        }
    }

    /// Where the event read last starts, as
    /// [`XmlStreamReader::offset`] says.
    #[must_use]
    pub fn offset(&self) -> u64 {
        self.reader.offset()
    }

    /// The namespace and local name of the element name `name`, as
    /// [`XmlStreamReader::resolve_element`] resolves it.
    #[must_use]
    pub fn resolve_element<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolve_element(name)
    }

    /// The namespace and local name of the attribute name `name`, as
    /// [`XmlStreamReader::resolve_attribute`] resolves it.
    #[must_use]
    pub fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        self.reader.resolve_attribute(name)
    }

    /// The namespace `prefix` is bound to, as
    /// [`XmlStreamReader::resolve_prefix`] resolves it.
    #[must_use]
    pub fn resolve_prefix(&self, prefix: &str) -> Option<&str> {
        self.reader.resolve_prefix(prefix)
    }

    /// The reader the part is read from, to ask it why it failed after an
    /// [`XmlStreamError::Io`]. There is no `get_mut`: reading from it would
    /// skip part of the XML.
    #[must_use]
    pub fn get_ref(&self) -> &R {
        self.reader.get_ref()
    }

    /// The output, to write raw bytes between events (an element built
    /// elsewhere). Call it after the first [`write`](Self::write), so the
    /// byte order mark comes first. What is written through it is not
    /// checked: the caller keeps the XML well formed.
    pub fn output_mut(&mut self) -> &mut W {
        self.writer.get_mut()
    }

    /// The output, given back once everything is written.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

/// `error` as [`io::ErrorKind::InvalidData`]: for a declaration whose
/// `encoding` value cannot be read or cannot be found in its text.
fn invalid(error: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

/// The declaration to write in place of `decl`, or `None` to write `decl`
/// as it is: for one that names an encoding other than `UTF-8`, `decl` with
/// the value of its `encoding` attribute replaced by `UTF-8` and every other
/// byte as it was.
fn utf8_decl(decl: &BytesDecl<'_>) -> io::Result<Option<BytesDecl<'static>>> {
    let Some(value) = decl.encoding().transpose().map_err(invalid)? else {
        return Ok(None);
    };
    if value.eq_ignore_ascii_case("UTF-8") {
        return Ok(None);
    }
    let content: &str = decl;
    let (before, after) = around(content, &value)?;
    let rewritten = format!("{before}UTF-8{after}");
    // 3: the length of the name `xml`, where the attributes start.
    Ok(Some(BytesDecl::from_start(BytesStart::from_content(
        rewritten, 3,
    ))))
}

/// The text of `outer` before `inner` and the text after it, for an `inner`
/// that is a slice of `outer` (a value quick-xml borrows from a
/// declaration's text). The position of `inner` in `outer` is the difference
/// of their addresses.
///
/// # Errors
///
/// [`io::ErrorKind::InvalidData`] when `inner` is not a slice of `outer`.
fn around<'a>(outer: &'a str, inner: &str) -> io::Result<(&'a str, &'a str)> {
    let start = inner.as_ptr().addr().checked_sub(outer.as_ptr().addr());
    let end = start.and_then(|start| start.checked_add(inner.len()));
    start
        .and_then(|start| outer.get(..start))
        .zip(end.and_then(|end| outer.get(end..)))
        .ok_or_else(|| invalid("the encoding value is not part of the declaration"))
}

#[cfg(test)]
mod tests;
