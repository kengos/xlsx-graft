//! xlsx-graft xml: the one way the engine reads a package's untrusted XML.
//!
//! [`XmlReader::open`] takes a part's bytes, already in memory, and the
//! [`XmlLimits`] in force, and hands back quick-xml [`events`] one at a time,
//! with the [`name`]s in them resolved to namespaces on request.
//! Every problem comes back as a [`Diagnostic`] for the caller to push with
//! the part it read, since only the caller knows which part that is. What it
//! guards, before or while the caller sees an event:
//!
//! - **Size**: a part larger than `max_bytes` is refused before a byte of it
//!   is parsed (`xml_too_large`).
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
//!   `xml_too_many_namespaces`.
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
//! After an error the reader is spent: every later call returns the same
//! diagnostic, so a caller cannot read past a problem by accident.
//!
//! **Streaming** (the design for a worksheet, which is too large to hold in
//! memory): the per-event checks live in `guard.rs` and see only events, so a
//! reader over a `BufRead` wraps the same guard. It sniffs the byte order
//! mark from the first `fill_buf`, decodes UTF-16 through a transcoding
//! `BufRead`, and checks characters as each event's bytes arrive instead of
//! over the whole part up front. It does not read on past a too-deep
//! element to report the deepest nesting, as the in-memory reader does: in a
//! large worksheet that would read the rest of it after the error. Only the
//! in-memory entry exists until a caller needs that one.
//!
//! The [`events`], [`name`] and [`XmlVersion`] it hands out are quick-xml's
//! own, re-exported so a caller needs no dependency of its own; a quick-xml
//! upgrade that changes them changes this crate's API too.

mod chars;
mod encoding;
mod guard;
mod limits;
mod reader;

pub use limits::XmlLimits;
pub use quick_xml::XmlVersion;
pub use quick_xml::events;
pub use quick_xml::name;
pub use reader::XmlReader;

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
