//! [`read`]: walks a package XML stream of one shape (ECMA-376-2:2021 Annex
//! C), a root element of no attributes holding child elements that each
//! have attributes of their own and no children, as both the content types
//! stream and a Relationships part are.
//!
//! What breaks the shape is `xml_schema_invalid`, and spoils only what it
//! is in: a child element that is not in the schema, or that holds an
//! attribute, an element or text it may not, is reported and left out; text
//! in the root and an attribute on it are reported. A root of another name
//! or namespace, and anything the XML reader refuses, end the reading.
//! Elements inside an element already reported are not reported again.
//! Attributes in another namespace, `xml:base` among them, are refused like
//! any other: no markup compatibility processing (ECMA-376-3) runs first.

mod walk;

use walk::{Named, Walk, is_blank};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, SchemaReason};
use xlsx_graft_xml::events::Event;
use xlsx_graft_xml::{XmlLimits, XmlReader};

/// The shape of a stream.
#[derive(Debug)]
pub(crate) struct Shape {
    /// The namespace of every element.
    pub(crate) namespace: &'static str,
    /// The root element's local name.
    pub(crate) root: &'static str,
    /// The child elements the root may hold, in any number and order.
    pub(crate) children: &'static [ChildShape],
}

/// The shape of a child element.
#[derive(Debug)]
pub(crate) struct ChildShape {
    /// Its local name.
    pub(crate) name: &'static str,
    /// The unprefixed attributes it may have.
    pub(crate) attributes: &'static [&'static str],
    /// Whether it may hold text.
    pub(crate) text: bool,
}

/// A child element as read, its shape kept.
#[derive(Debug)]
pub(crate) struct Element {
    /// Which of the shape's children it is.
    pub(crate) child: usize,
    /// Where it starts, as an `xml_*` offset.
    pub(crate) offset: u64,
    /// Its attributes' values, in the order of the shape's attributes.
    pub(crate) values: Vec<Option<String>>,
}

/// Reads `bytes` as a stream of `shape`, handing each child element that
/// keeps to it to `visit`, and pushing what breaks it at `location`.
///
/// Returns `None` when the stream was not read to its end, and the caller
/// drops the elements already visited; else whether every child element
/// was visited, none refused.
pub(crate) fn read(
    bytes: &[u8],
    limits: &XmlLimits,
    shape: &Shape,
    location: &Location,
    diagnostics: &mut Diagnostics,
    mut visit: impl FnMut(Element, &mut Diagnostics),
) -> Option<bool> {
    let mut reader = match XmlReader::open(bytes, limits) {
        Ok(reader) => reader,
        Err(diagnostic) => {
            diagnostics.push(diagnostic, location.clone());
            return None;
        }
    };
    let mut walk = Walk {
        shape,
        location,
        depth: 0,
        current: None,
        lost: false,
    };
    let mut buf = Vec::new();
    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event,
            Err(diagnostic) => {
                diagnostics.push(diagnostic, location.clone());
                return None;
            }
        };
        let offset = reader.offset();
        match &event {
            Event::Start(start) | Event::Empty(start) => {
                let opened = matches!(event, Event::Start(_));
                let named = Named::of(&reader, start, offset);
                if !walk.element(&named, start, opened, diagnostics, &mut visit) {
                    return None;
                }
            }
            Event::End(_) => {
                walk.depth = walk.depth.saturating_sub(1);
                if walk.depth == 1
                    && let Some(element) = walk.current.take()
                {
                    visit(element, diagnostics);
                }
            }
            Event::Text(text) => walk.text(is_blank(text), offset, diagnostics),
            Event::CData(data) => walk.text(data.is_empty(), offset, diagnostics),
            Event::GeneralRef(_) => walk.text(false, offset, diagnostics),
            Event::Eof => return Some(!walk.lost),
            _ => {}
        }
    }
}

/// `xml_schema_invalid` for the first attribute of `element` whose value
/// in `values` is missing.
pub(crate) fn missing_attribute(
    shape: &Shape,
    element: &Element,
    values: &[Option<String>],
) -> Diagnostic {
    let names = shape
        .children
        .get(element.child)
        .map_or(&[][..], |c| c.attributes);
    let missing = names.iter().zip(values).find(|(_, value)| value.is_none());
    Diagnostic::XmlSchemaInvalid {
        offset: element.offset,
        reason: SchemaReason::MissingAttribute,
        name: Echo::new(missing.map_or("", |(name, _)| name)),
    }
}

/// `xml_schema_invalid` for the value of the attribute `name` of the
/// element at `offset`.
pub(crate) fn invalid_value(offset: u64, name: &str) -> Diagnostic {
    Diagnostic::XmlSchemaInvalid {
        offset,
        reason: SchemaReason::InvalidValue,
        name: Echo::new(name),
    }
}

/// `value` collapsed as XML Schema's `whiteSpace` facet `collapse` has it:
/// runs of white space become one space, and none is left at either end.
pub(crate) fn collapse(value: &str) -> String {
    let words: Vec<&str> = (value.split([' ', '\t', '\r', '\n']))
        .filter(|word| !word.is_empty())
        .collect();
    words.join(" ")
}
