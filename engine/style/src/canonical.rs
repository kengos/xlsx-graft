//! The canonical form of an item of the styles part (a `font`, `fill`,
//! `border` or `xf`) and of the elements in it that the schema table names.
//! Two items that have the same effect have equal canonical forms: the order of
//! the children and of the attributes does not matter, an attribute that has
//! its default value is the same as no attribute, and an element that has no
//! effect (`<b val="0"/>`) is the same as no element. What the table does
//! not name (an attribute with a prefix, a `gradientFill`, an element of
//! another namespace) is not read: it is kept as it was written, and equal
//! only to the same bytes, so that overlaying a style never removes it.
//!
//! This comparison is the one for the lists that `cellXfs` refers to (the
//! fonts, fills, borders and `cellXfs` themselves). It does not hold for a
//! `dxf`, a differential format, where `<b val="0"/>` and no `b` differ.
//!
//! The form is built with the functions of [`Node`]: the reader of the
//! styles editor calls [`Node::push_attribute`], [`Node::push_child`] and
//! [`Node::push_raw`] as it reads, and [`overlay`] states a style with
//! [`Node::set_attribute`] and [`Node::set_child`]. A [`Node`] is compared
//! only after [`Node::finish`], which sorts the foreign attributes and the
//! children, once.
//!
//! An item is started with [`Node::root`], which admits only `font`, `fill`,
//! `border` and `xf`. A child is started with [`Node::child_element`], which
//! admits only an element the parent's table lists as a child. Anything else
//! is a [`Child::Raw`], whose inside is bytes and is not read. So [`Node`]s
//! nest at most three deep by construction, an item and two levels below it
//! (`fill` > `patternFill` > `fgColor`, `border` > `left` > `color`).

pub(crate) mod overlay;
mod schema;
mod tokens;
pub(crate) mod write;

use schema::{Absent, Element};

/// An element of the styles part: an item, or an element in it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Node {
    /// The element of the schema table.
    element: &'static Element,
    /// The attributes the table names, in the table's order, without those
    /// that have their default value.
    attributes: Vec<(&'static str, Value)>,
    /// The attributes the table does not name (with a prefix, `xmlns` or an
    /// unknown name): the name as written, and the value with its character
    /// references resolved, in the order of the names, then the values.
    foreign: Vec<(Vec<u8>, String)>,
    /// The children, in the order they are written.
    children: Vec<Child>,
}

/// A child of a [`Node`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Child {
    /// An element the table names.
    Node(Node),
    /// An element the table does not name (a `gradientFill`, an `extLst` or
    /// an element of another namespace), as the bytes it was written in.
    Raw {
        /// The position of the element's name in the parent's list of
        /// children, which is the order they are written in; `usize::MAX`
        /// when the list does not have the name. An element in another
        /// namespace has the rank of the child before it (0 when it is the
        /// first), so that it is written after the nodes of that rank and is
        /// not moved after the `extLst`.
        rank: usize,
        /// The element's bytes, from `<` to `>`.
        bytes: Vec<u8>,
        /// Whether the element is in a namespace other than the styles
        /// part's. [`Node::set_child`] keeps such an element when it replaces
        /// the child whose rank it has.
        other_namespace: bool,
    },
}

/// The value of an attribute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Value {
    /// xsd:boolean.
    Bool(bool),
    /// xsd:unsignedInt.
    Uint(u32),
    /// xsd:int.
    Int(i32),
    /// xsd:double, as the bits of the `f64`; -0.0 is 0.0.
    Double(u64),
    /// ST_UnsignedIntHex, written as 8 upper case digits.
    Hex(u32),
    /// A value of an enumeration.
    Token(&'static str),
    /// A string, white space included.
    Text(String),
    /// Text that is not of the attribute's type: compared as written, and
    /// written as it was.
    Raw(String),
}

impl Value {
    /// The double `number`, which must be finite.
    pub(crate) fn double(number: f64) -> Self {
        Self::Double(if number == 0.0 { 0 } else { number.to_bits() })
    }
}

impl Node {
    /// An empty node of `element`.
    pub(crate) fn new(element: &'static Element) -> Self {
        Self {
            element,
            attributes: Vec::new(),
            foreign: Vec::new(),
            children: Vec::new(),
        }
    }

    /// An empty node for an item of the styles part: `font`, `fill`, `border`
    /// or `xf`, named `local`. `None` for any other name.
    pub(crate) fn root(local: &[u8]) -> Option<Self> {
        schema::root(local).map(Self::new)
    }

    /// An empty `font`.
    pub(crate) fn font() -> Self {
        Self::new(&schema::FONT)
    }

    /// An empty `border`.
    pub(crate) fn border() -> Self {
        Self::new(&schema::BORDER)
    }

    /// An empty `xf`.
    pub(crate) fn xf() -> Self {
        Self::new(&schema::XF)
    }

    /// An empty node for the child named `local` of this node, when the
    /// table lists `local` among this element's children and defines it.
    /// `local` is the local name of an element in the namespace of the styles
    /// part. `None` otherwise: the caller passes that child to
    /// [`Node::push_raw`], an element in another namespace with `None` as its
    /// local name.
    pub(crate) fn child_element(&self, local: &[u8]) -> Option<Self> {
        let listed = self.element.children.iter().any(|c| c.as_bytes() == local);
        schema::lookup(local).filter(|_| listed).map(Self::new)
    }

    /// Reads the attribute `name` with the text `value`: by its type when the
    /// table names it (`name` has no prefix), and as a foreign attribute
    /// otherwise.
    pub(crate) fn push_attribute(&mut self, name: &[u8], value: &str) {
        let table = self.element.attributes;
        match table.iter().find(|a| a.name.as_bytes() == name) {
            Some(attribute) => {
                self.set_attribute(attribute.name, Some(attribute.kind.read(value)));
            }
            None => self.foreign.push((name.to_vec(), value.to_owned())),
        }
    }

    /// Sets the attribute `name` of the table to `value`, or removes it for
    /// `None`. The attribute is not held when it has its default value.
    pub(crate) fn set_attribute(&mut self, name: &'static str, value: Option<Value>) {
        self.attributes.retain(|(held, _)| *held != name);
        let table = self.element.attributes;
        let position = |name: &str| table.iter().position(|a| a.name == name);
        if let Some(at) = position(name)
            && let Some(value) = value
            && table.get(at).and_then(|a| a.default.as_ref()) != Some(&value)
        {
            let before = |(held, _): &&(&str, Value)| position(held) < Some(at);
            let place = self.attributes.iter().take_while(before).count();
            self.attributes.insert(place, (name, value));
        }
    }

    /// Adds `child`, unless it has no effect (is the same as no element).
    pub(crate) fn push_child(&mut self, child: Self) {
        let child = child.finish();
        if !child.is_absent() {
            self.children.push(Child::Node(child));
        }
    }

    /// Adds an element the table does not name, as the bytes it was written
    /// in. `local` is its local name when it is in the namespace of the
    /// styles part, and `None` when it is in another. An element in another
    /// namespace has the rank of the child pushed before it (0 when it is
    /// the first), so it is written after the nodes of that rank and does
    /// not move after the `extLst`.
    pub(crate) fn push_raw(&mut self, local: Option<&[u8]>, bytes: Vec<u8>) {
        let rank = match local {
            Some(local) => {
                let children = self.element.children;
                (children.iter().position(|c| c.as_bytes() == local)).unwrap_or(usize::MAX)
            }
            None => self.children.last().map_or(0, |last| match last {
                Child::Node(node) => self.rank_of(node.element.name),
                Child::Raw { rank, .. } => *rank,
            }),
        };
        let other_namespace = local.is_none();
        self.children.push(Child::Raw {
            rank,
            bytes,
            other_namespace,
        });
    }

    /// Replaces every child named `name` (the nodes of that name, and the raw
    /// children at the rank of that name, but not those in another
    /// namespace) with `child`, or removes them for `None`.
    pub(crate) fn set_child(&mut self, name: &str, child: Option<Self>) {
        let rank = self.rank_of(name);
        self.children.retain(|held| match held {
            Child::Node(node) => node.element.name != name,
            Child::Raw {
                rank: held,
                other_namespace,
                ..
            } => *other_namespace || *held != rank || rank == usize::MAX,
        });
        if let Some(child) = child {
            self.push_child(child);
        }
        self.sort();
    }

    /// The value of the attribute `name` of the table, which the node holds
    /// unless it has its default value.
    pub(crate) fn attribute(&self, name: &str) -> Option<&Value> {
        (self.attributes.iter())
            .find(|(held, _)| *held == name)
            .map(|(_, value)| value)
    }

    /// The first child named `name`.
    pub(crate) fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find_map(|child| match child {
            Child::Node(node) if node.element.name == name => Some(node),
            _ => None,
        })
    }

    /// Puts the foreign attributes and the children in order. A node is
    /// compared and written only after this, and its children must have been
    /// through it already.
    #[must_use]
    pub(crate) fn finish(mut self) -> Self {
        self.sort();
        self
    }

    fn sort(&mut self) {
        let element = self.element;
        let rank = |child: &Child| match child {
            Child::Node(node) => rank_in(element, node.element.name),
            Child::Raw { rank, .. } => *rank,
        };
        self.foreign.sort();
        self.children
            .sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    }

    fn rank_of(&self, name: &str) -> usize {
        rank_in(self.element, name)
    }

    /// Whether the node has no effect, as the table's `absent` states.
    fn is_absent(&self) -> bool {
        let bare = self.foreign.is_empty() && self.children.is_empty();
        match &self.element.absent {
            Absent::Never => false,
            Absent::Empty => bare && self.attributes.is_empty(),
            Absent::When(name, value) => {
                bare && matches!(self.attributes.as_slice(), [(held, v)] if held == name && v == value)
            }
        }
    }
}

/// The rank of the child `name` of `element`.
fn rank_in(element: &Element, name: &str) -> usize {
    (element.children.iter().position(|c| *c == name)).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests;
