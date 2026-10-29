//! [`Ignorable`]: the namespaces markup compatibility's `mc:Ignorable`
//! names (ECMA-376-3), for a reader of a part that takes `mc:Ignorable`
//! and no other markup compatibility, as the crate root says.
//!
//! A namespace is compared as the XML reader resolves it, its declaration
//! as written.
//!
//! What a value decides is the set of namespaces passed over inside the
//! element that names them: each is one declared in scope, so the set is
//! bounded by the namespaces in scope (`maxXmlNamespaces`), not by the
//! value, and each URI by `maxXmlNamespaceBytes`, which the XML reader
//! holds a declaration to, so looking one up costs no more than the limits
//! allow however many elements refer to it. A prefix that does not resolve
//! refuses the value there and then, so only prefixes that resolve are
//! remembered, and each is resolved once.
//!
//! # Examples
//!
//! A reader of either XML reader keeps one for the part:
//!
//! ```text
//! let mut ignorable = Ignorable::new(&NAMESPACES);
//! // At each element read, before its other attributes:
//! ignorable.enter();
//! ignorable.take(start, &reader, offset)?;
//! // An element whose namespace `ignorable.contains(ns)`, and an
//! // attribute that `ignorable.passes_over(ns, local)`, are passed over.
//! // At the element's end:
//! ignorable.leave();
//! ```

use crate::schema::{namespace, refuse};
use std::collections::BTreeSet;
use std::io::BufRead;
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::name::{LocalName, QName, ResolveResult};
use xlsx_graft_xml::{XmlReader, XmlStreamReader, XmlVersion};

/// The namespace of markup compatibility.
pub(crate) const MARKUP_COMPATIBILITY: &str =
    "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// The namespaces every reader understands: XML's own, bound to `xml`
/// and `xmlns` without a declaration.
const XML: [&str; 2] = [
    "http://www.w3.org/XML/1998/namespace",
    "http://www.w3.org/2000/xmlns/",
];

/// The namespaces named ignorable by the elements open.
#[derive(Debug)]
pub(crate) struct Ignorable {
    /// The namespaces the reader understands besides XML's own: named
    /// ignorable, they are still read, as markup compatibility has an
    /// ignorable namespace only ignored by a reader that does not
    /// understand it.
    understood: &'static [&'static str],
    /// Each namespace named, once.
    namespaces: BTreeSet<String>,
    /// Each of `namespaces` with the depth of the element that named it
    /// first, in the order named: the depth never falls from one to the
    /// next, so leaving an element pops what it named off the end.
    named: Vec<(u64, String)>,
    /// How many elements are entered.
    depth: u64,
}

/// A `mc:Ignorable` value that cannot be taken: a prefix that is not
/// declared, is not a prefix, or names markup compatibility, whose own
/// elements and attributes it would let through.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Refused;

/// What resolves the names of the element read: either XML reader.
pub(crate) trait Resolve {
    /// The namespace and local name of the attribute `name`.
    fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>);
    /// The namespace `prefix` is bound to.
    fn resolve_prefix(&self, prefix: &str) -> Option<&str>;
}

impl Resolve for XmlReader<'_> {
    fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        XmlReader::resolve_attribute(self, name)
    }

    fn resolve_prefix(&self, prefix: &str) -> Option<&str> {
        XmlReader::resolve_prefix(self, prefix)
    }
}

impl<B: BufRead> Resolve for XmlStreamReader<B> {
    fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult<'_>, LocalName<'n>) {
        XmlStreamReader::resolve_attribute(self, name)
    }

    fn resolve_prefix(&self, prefix: &str) -> Option<&str> {
        XmlStreamReader::resolve_prefix(self, prefix)
    }
}

impl Ignorable {
    /// None named yet, for a reader that understands `understood`.
    pub(crate) fn new(understood: &'static [&'static str]) -> Self {
        Self {
            understood,
            namespaces: BTreeSet::new(),
            named: Vec::new(),
            depth: 0,
        }
    }

    /// Whether `namespace` and `local` name the `mc:Ignorable` attribute.
    pub(crate) fn is_attribute(namespace: Option<&str>, local: &str) -> bool {
        namespace == Some(MARKUP_COMPATIBILITY) && local == "Ignorable"
    }

    /// Enters an element read, before its attributes.
    pub(crate) fn enter(&mut self) {
        self.depth = self.depth.saturating_add(1);
    }

    /// Takes in the namespaces of the prefixes in `value`, the entered
    /// element's `mc:Ignorable`, each prefix resolved by `resolve` once.
    /// After an `Err` the part is refused, so `self` is not used again.
    ///
    /// # Errors
    ///
    /// [`Refused`] at the first prefix that holds a `:`, does not resolve,
    /// or resolves to markup compatibility. One that resolves to a
    /// namespace understood is taken and changes nothing.
    pub(crate) fn name<'r>(
        &mut self,
        value: &str,
        mut resolve: impl FnMut(&str) -> Option<&'r str>,
    ) -> Result<(), Refused> {
        // Only prefixes that resolved, so no more than are declared.
        let mut seen = BTreeSet::new();
        for prefix in value.split_ascii_whitespace() {
            if seen.contains(prefix) {
                continue;
            }
            if prefix.contains(':') {
                return Err(Refused);
            }
            let namespace = resolve(prefix).ok_or(Refused)?;
            if namespace == MARKUP_COMPATIBILITY {
                return Err(Refused);
            }
            seen.insert(prefix);
            let understood = self.understood.contains(&namespace) || XML.contains(&namespace);
            if !understood && !self.contains(namespace) {
                self.namespaces.insert(namespace.to_owned());
                self.named.push((self.depth, namespace.to_owned()));
            }
        }
        Ok(())
    }

    /// Takes in the `mc:Ignorable` of `start`, the element entered, which
    /// `reader` read at `offset`: the attribute covers the ones beside it,
    /// so it is taken before them.
    ///
    /// # Errors
    ///
    /// `xml_schema_invalid` (an invalid value) naming the attribute, when
    /// [`Ignorable::name`] refuses its value.
    pub(crate) fn take(
        &mut self,
        start: &BytesStart<'_>,
        reader: &impl Resolve,
        offset: u64,
    ) -> Result<(), Diagnostic> {
        for attribute in start.attributes().flatten() {
            if attribute.key.local_name().as_ref() != "Ignorable" {
                continue;
            }
            let (resolved, local) = reader.resolve_attribute(attribute.key);
            if Self::is_attribute(namespace(&resolved), local.as_ref()) {
                let value = attribute.normalized_value(XmlVersion::Implicit1_0);
                let key = attribute.key.as_ref();
                let resolve = |prefix: &str| reader.resolve_prefix(prefix);
                (self.name(&value.unwrap_or_default(), resolve))
                    .map_err(|_| refuse(offset, SchemaReason::InvalidValue, key))?;
            }
        }
        Ok(())
    }

    /// Whether an attribute in `namespace` named `local` is passed over:
    /// `mc:Ignorable`, taken already, or one in a namespace it names.
    pub(crate) fn passes_over(&self, namespace: Option<&str>, local: &str) -> bool {
        Self::is_attribute(namespace, local) || namespace.is_some_and(|ns| self.contains(ns))
    }

    /// Whether an element or attribute in `namespace` is passed over.
    pub(crate) fn contains(&self, namespace: &str) -> bool {
        self.namespaces.contains(namespace)
    }

    /// Leaves the element entered last, dropping what it named.
    pub(crate) fn leave(&mut self) {
        let depth = self.depth;
        while let Some((_, namespace)) = self.named.pop_if(|(named, _)| *named >= depth) {
            self.namespaces.remove(&namespace);
        }
        self.depth = depth.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests;
