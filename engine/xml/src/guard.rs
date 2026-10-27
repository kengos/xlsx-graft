//! The checks the reader runs on every event before the caller sees it:
//! DTDs, nesting depth, attribute and namespace counts and the
//! well-formedness rules
//! quick-xml leaves to its caller. It sees only events, so the in-memory
//! and the streaming reader run the same checks.

use crate::XmlLimits;
use crate::chars::{describe, is_xml_char};
use quick_xml::XmlVersion;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, BytesStart, Event};

mod stop;

pub(crate) use stop::Stop;

/// What the guard knows of the document so far.
#[derive(Debug, Clone)]
pub(crate) struct Guard {
    limits: XmlLimits,
    /// How many elements are open.
    depth: u64,
    /// How many namespaces each open element declares, outermost first.
    scopes: Vec<u64>,
    /// How many namespace declarations are in scope: the sum of `scopes`.
    namespaces: u64,
    /// Whether any event has been seen.
    started: bool,
    /// Whether the root element has been seen.
    rooted: bool,
}

impl Guard {
    /// A guard for a document read under `limits`.
    pub(crate) fn new(limits: XmlLimits) -> Self {
        Self {
            limits,
            depth: 0,
            scopes: Vec::new(),
            namespaces: 0,
            started: false,
            rooted: false,
        }
    }

    /// How many elements are open, the one a stop refused included when it
    /// was a start tag.
    pub(crate) fn depth(&self) -> u64 {
        self.depth
    }

    /// The limits the guard holds the document to.
    pub(crate) fn limits(&self) -> XmlLimits {
        self.limits
    }

    /// Checks `event`, the next one in the document.
    pub(crate) fn check(&mut self, event: &Event<'_>) -> Result<(), Stop> {
        let first = !self.started;
        self.started = true;
        match event {
            Event::Decl(_) if !first => malformed("the XML declaration is not at the start"),
            Event::DocType(_) => Err(Stop::Doctype),
            Event::Start(start) => {
                let checked = self.element(start, true);
                // Counted even when refused, so a start tag one too deep
                // leaves `depth` past `max_depth` and an empty element not.
                self.depth = self.depth.saturating_add(1);
                checked
            }
            Event::Empty(start) => self.element(start, false),
            Event::End(_) => {
                self.depth = self.depth.saturating_sub(1);
                let closed = self.scopes.pop().unwrap_or(0);
                self.namespaces = self.namespaces.saturating_sub(closed);
                Ok(())
            }
            Event::Text(text) if self.depth == 0 && !is_xml_space(text) => outside(),
            Event::CData(_) | Event::GeneralRef(_) if self.depth == 0 => outside(),
            Event::GeneralRef(reference) => check_reference(reference),
            Event::Eof if self.depth > 0 => malformed("an element is not closed"),
            Event::Eof if !self.rooted => malformed("the XML has no root element"),
            _ => Ok(()),
        }
    }

    /// Checks an element at `depth + 1`: a start tag when `opened`, else an
    /// empty element.
    fn element(&mut self, start: &BytesStart<'_>, opened: bool) -> Result<(), Stop> {
        if self.depth == 0 {
            if self.rooted {
                return malformed("the XML has a second root element");
            }
            self.rooted = true;
        }
        if self.depth >= self.limits.max_depth.value {
            return Err(Stop::TooDeep);
        }
        let mut count: u64 = 0;
        let mut declared: u64 = 0;
        // quick-xml checks for a duplicate name through a hash set past 32
        // attributes, so counting every attribute stays linear.
        for attribute in start.attributes() {
            let attribute = attribute.map_err(|error| Stop::Malformed(error.to_string()))?;
            count = count.saturating_add(1);
            let key = attribute.key.as_ref();
            if key == "xmlns" || key.starts_with("xmlns:") {
                declared = declared.saturating_add(1);
            }
            // Past the limit only the count matters: the element is refused.
            if count <= self.limits.max_attributes.value {
                let value = attribute
                    .normalized_value(XmlVersion::Implicit1_0)
                    .map_err(|error| Stop::Malformed(error.to_string()))?;
                if let Some(c) = value.chars().find(|&c| !is_xml_char(c)) {
                    return Err(Stop::Malformed(describe(c)));
                }
            }
        }
        if count > self.limits.max_attributes.value {
            return Err(Stop::TooManyAttributes(count));
        }
        let namespaces = self.namespaces.saturating_add(declared);
        if namespaces > self.limits.max_namespaces.value {
            return Err(Stop::TooManyNamespaces(namespaces));
        }
        if opened {
            self.scopes.push(declared);
            self.namespaces = namespaces;
        }
        Ok(())
    }
}

/// Admits a character reference to a character XML allows, or a reference
/// to one of the five predefined entities. With no DTD, any other entity is
/// undeclared.
fn check_reference(reference: &BytesRef<'_>) -> Result<(), Stop> {
    match reference.resolve_char_ref() {
        Ok(Some(c)) if is_xml_char(c) => Ok(()),
        Ok(Some(c)) => Err(Stop::Malformed(describe(c))),
        Ok(None) if resolve_predefined_entity(reference).is_some() => Ok(()),
        Ok(None) => malformed("a reference to an undeclared entity"),
        Err(error) => Err(Stop::Malformed(error.to_string())),
    }
}

/// Whether `text` is all XML white space (XML 1.0 §2.3, `S`).
fn is_xml_space(text: &str) -> bool {
    text.bytes()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

/// Text, a CDATA section or a reference outside the root element.
fn outside() -> Result<(), Stop> {
    malformed("the XML has content outside the root element")
}

/// [`Stop::Malformed`] for `reason`.
fn malformed(reason: &str) -> Result<(), Stop> {
    Err(Stop::Malformed(reason.to_owned()))
}
