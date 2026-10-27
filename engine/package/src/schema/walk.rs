//! The walk [`read`](super::read) takes through a stream: where it is, and
//! what each element and text does there.

use super::{Element, Shape};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, SchemaReason};
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlReader, XmlVersion};

/// An element's name, resolved.
pub(super) struct Named {
    /// The name as written.
    qname: String,
    /// Its namespace, when it is bound to one.
    namespace: Option<String>,
    /// Its local name.
    local: String,
    /// Where it starts.
    offset: u64,
}

impl Named {
    /// The name of `start`, the event `reader` read last, at `offset`.
    pub(super) fn of(reader: &XmlReader<'_>, start: &BytesStart<'_>, offset: u64) -> Self {
        let (namespace, local) = reader.resolve_element(start.name());
        let namespace = match namespace {
            ResolveResult::Bound(Namespace(namespace)) => Some(namespace.to_owned()),
            _ => None,
        };
        Self {
            qname: start.name().as_ref().to_owned(),
            namespace,
            local: local.as_ref().to_owned(),
            offset,
        }
    }
}

/// Where the walk is.
pub(super) struct Walk<'a> {
    pub(super) shape: &'a Shape,
    pub(super) location: &'a Location,
    /// How many elements are open.
    pub(super) depth: u64,
    /// The child element open now, unless it was refused.
    pub(super) current: Option<Element>,
    /// Whether a child element was refused, and so left out.
    pub(super) lost: bool,
}

impl Walk<'_> {
    /// Reads an element: the root, a child of it, or one inside a child.
    /// Returns false when the root is not the shape's.
    pub(super) fn element(
        &mut self,
        named: &Named,
        start: &BytesStart<'_>,
        opened: bool,
        diagnostics: &mut Diagnostics,
        visit: &mut impl FnMut(Element, &mut Diagnostics),
    ) -> bool {
        let depth = self.depth;
        if opened {
            self.depth = self.depth.saturating_add(1);
        }
        let ours = named.namespace.as_deref() == Some(self.shape.namespace);
        let unexpected = SchemaReason::UnexpectedElement;
        match depth {
            0 if !ours || named.local != self.shape.root => {
                self.refuse(named.offset, unexpected, &named.qname, diagnostics);
                return false;
            }
            0 => {
                for (name, _) in attributes(start) {
                    let reason = SchemaReason::UnexpectedAttribute;
                    self.refuse(named.offset, reason, &name, diagnostics);
                }
            }
            1 => {
                let children = self.shape.children;
                let child = children.iter().position(|c| ours && c.name == named.local);
                self.current = child.and_then(|child| self.child(child, named, start, diagnostics));
                self.lost |= self.current.is_none();
                if child.is_none() {
                    self.refuse(named.offset, unexpected, &named.qname, diagnostics);
                }
                if !opened && let Some(element) = self.current.take() {
                    visit(element, diagnostics);
                }
            }
            2 if self.current.take().is_some() => {
                self.lost = true;
                self.refuse(named.offset, unexpected, &named.qname, diagnostics);
            }
            _ => {}
        }
        true
    }

    /// The child element `child` of the shape, unless an attribute it may
    /// not have refuses it.
    fn child(
        &self,
        child: usize,
        named: &Named,
        start: &BytesStart<'_>,
        diagnostics: &mut Diagnostics,
    ) -> Option<Element> {
        let allowed = self.shape.children.get(child)?.attributes;
        let mut values = vec![None; allowed.len()];
        let mut refused = false;
        for (name, value) in attributes(start) {
            match allowed.iter().position(|&a| a == name) {
                Some(index) => {
                    if let Some(slot) = values.get_mut(index) {
                        *slot = Some(value);
                    }
                }
                None => {
                    refused = true;
                    let reason = SchemaReason::UnexpectedAttribute;
                    self.refuse(named.offset, reason, &name, diagnostics);
                }
            }
        }
        (!refused).then_some(Element {
            child,
            offset: named.offset,
            values,
        })
    }

    /// Reads text, `blank` when it is white space.
    pub(super) fn text(&mut self, blank: bool, offset: u64, diagnostics: &mut Diagnostics) {
        let shape = self.shape;
        if blank {
            return;
        }
        if self.depth == 1 {
            self.refuse(
                offset,
                SchemaReason::UnexpectedText,
                shape.root,
                diagnostics,
            );
        } else if self.depth == 2
            && let Some(child) = (self.current.as_ref()).and_then(|e| shape.children.get(e.child))
            && !child.text
        {
            self.current = None;
            self.lost = true;
            self.refuse(
                offset,
                SchemaReason::UnexpectedText,
                child.name,
                diagnostics,
            );
        }
    }

    /// Pushes `xml_schema_invalid` at `offset`.
    #[track_caller]
    fn refuse(&self, offset: u64, reason: SchemaReason, name: &str, diagnostics: &mut Diagnostics) {
        let name = Echo::new(name);
        let diagnostic = Diagnostic::XmlSchemaInvalid {
            offset,
            reason,
            name,
        };
        diagnostics.push(diagnostic, self.location.clone());
    }
}

/// The attributes of `start` but its namespace declarations, each as its
/// name as written and its value. A name with a prefix keeps it, so it
/// matches no unprefixed name a shape allows.
fn attributes(start: &BytesStart<'_>) -> Vec<(String, String)> {
    start
        .attributes()
        .flatten()
        .filter_map(|attribute| {
            let key = attribute.key.as_ref();
            if key == "xmlns" || key.starts_with("xmlns:") {
                return None;
            }
            let value = attribute.normalized_value(XmlVersion::Implicit1_0);
            Some((key.to_owned(), value.unwrap_or_default().into_owned()))
        })
        .collect()
}

/// Whether `text` is all XML white space.
pub(super) fn is_blank(text: &str) -> bool {
    text.bytes()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}
