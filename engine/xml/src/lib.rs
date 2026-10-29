//! xlsx-graft xml: the one way the engine reads a package's untrusted XML.
//!
//! Two readers hand back quick-xml [`events`] one at a time, with the
//! [`name`]s in them resolved to namespaces on request, under the
//! [`XmlLimits`] in force: [`XmlReader::open`] over a part's bytes already
//! in memory, and [`XmlStreamReader::new`] over a `BufRead`, for a part too
//! large to hold (a large shared-strings table, a worksheet). The in-memory
//! reader is the streaming one over a slice, so they read a part the same
//! way. Every problem comes back as a [`Diagnostic`] for the caller to push
//! with the part it read, since only the caller knows which part that is.
//! What they guard, while the caller reads:
//!
//! - **Size**: the in-memory reader refuses a part larger than `max_bytes`
//!   before a byte of it is parsed (`xml_too_large`). Both refuse an event
//!   longer than `max_event_bytes` (`xml_event_too_large`): a tag with its
//!   attributes and prefixes, a run of text, a CDATA section, a comment, a
//!   processing instruction, the declaration, a DOCTYPE or a reference, as
//!   read, in UTF-8. The caller's buffer holds one event whole, so this
//!   bounds its length; reading stops one byte past the limit, before the
//!   rest of the event is read. Read that far, the event is too large
//!   whatever else is wrong with it; a byte before that which does not
//!   decode stops reading first, as below.
//! - **Encoding**: package XML is UTF-8 or UTF-16 and nothing else
//!   (ECMA-376-2:2021 §6.2.5; 4th edition §9.1.4, M1.17). The byte order mark
//!   and the XML declaration decide which; UTF-16 is decoded to UTF-8 with
//!   `char::decode_utf16`, so quick-xml only ever sees UTF-8. Any other
//!   encoding, a declaration that disagrees with the bytes, and bytes that do
//!   not decode are `xml_encoding_unsupported`.
//! - **DTDs**: a DOCTYPE, in any letter case, is `xml_doctype` (§6.2.5; M1.18
//!   makes it an error). quick-xml never reads a DTD or expands an entity: it
//!   reports a DOCTYPE as an event, which the reader refuses, and leaves
//!   every reference to the caller. The reader admits only the five
//!   predefined entities and character references to characters XML allows.
//! - **Shape**: nesting deeper than `max_depth` is `xml_too_deep`; an element
//!   with more than `max_attributes` attributes is `xml_too_many_attributes`;
//!   more than `max_namespaces` namespace declarations in scope is
//!   `xml_too_many_namespaces`; a declaration whose value, as written, is
//!   longer than `max_namespace_bytes` is `xml_namespace_too_long`, so a
//!   URI a reader holds and compares is bounded.
//! - **Well-formedness**: whatever quick-xml refuses, plus what it lets
//!   through and the reader checks: a character XML does not allow, an
//!   unclosed element at the end, no root or a second one, content outside
//!   the root, a declaration that is not first, a duplicate or unescapable
//!   attribute. Each is `xml_malformed` with the byte offset where reading
//!   stopped. Not checked: names against the `Name` production, a `<` in an
//!   attribute value, `]]>` in text, the declaration's version and the order
//!   or repetition of its attributes, a processing
//!   instruction named `XML` in another letter case, and
//!   undeclared namespace prefixes, which resolve to
//!   [`ResolveResult::Unknown`](name::ResolveResult::Unknown) for the
//!   caller to refuse.
//!
//! **Which problem a part gets**: the first that stops reading, in reading
//! order. A byte that does not decode, a character XML does not allow and a
//! second byte order mark stop it at that byte, so the event they sit in is
//! never judged; the declaration is judged once it is read to its `?>`, and
//! every other event once it is read whole. A part is read well only once
//! [`Event::Eof`](events::Event::Eof) comes back: a caller that stops at the
//! root's end tag has not seen what follows it, the end of the encoding, or
//! a zip entry's CRC. After an error the reader is spent: every later call
//! returns the same error, so a caller cannot read past a problem by
//! accident.
//!
//! The two readers differ only in what follows; anything else is a bug.
//!
//! - The streaming reader does not check `max_bytes`, since it never holds
//!   the part: it holds one event at a time, read into the caller's buffer,
//!   and a chunk of the part being decoded, 64 KiB of it at most.
//!   What bounds the bytes it reads is the zip limits on what an entry
//!   inflates to (`maxEntryBytes`, `maxTotalBytes`); what bounds the event
//!   it holds is `max_event_bytes`.
//! - The in-memory reader reads on past a too-deep element to report the
//!   deepest nesting in `xml_too_deep`; the streaming reader reports
//!   `max_depth + 1`, since in a large worksheet reading on would read the
//!   rest of it after the error.
//! - The streaming reader hands back a host failure of the reader it reads
//!   from as [`XmlStreamError::Io`], apart from the diagnostics.
//!
//! The [`events`], [`name`] and [`XmlVersion`] it hands out are quick-xml's
//! own, re-exported so a caller needs no dependency of its own; a quick-xml
//! upgrade that changes them changes this crate's API too.

mod chars;
mod encoding;
mod guard;
mod limits;
mod reader;
mod stream;

pub use limits::XmlLimits;
pub use quick_xml::XmlVersion;
pub use quick_xml::events;
pub use quick_xml::name;
pub use reader::XmlReader;
pub use stream::{XmlStreamError, XmlStreamReader};

use xlsx_graft_diagnostics::{Diagnostic, Echo};

/// `xml_malformed` at `offset`, saying `detail`.
pub(crate) fn malformed(offset: u64, detail: &str) -> Diagnostic {
    Diagnostic::XmlMalformed {
        offset,
        detail: Echo::new(detail),
    }
}

#[cfg(test)]
mod tests;
