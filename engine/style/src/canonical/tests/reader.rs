//! A small XML reader for the tests: it reads one element of a styles part
//! into a [`Node`], calling the functions of `Node` as the reader of the next
//! pull request will. An element the table does not name, or with a prefix,
//! is kept as the bytes it was written in.

use crate::canonical::Node;
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlLimits, XmlReader, XmlVersion};

/// What one event of the part is.
enum Kind {
    Open,
    Empty,
    Close,
    Other,
}

/// An event with the bytes it spans (from where it starts to where the next
/// event starts) and, for an element, its name and attributes.
struct Tag {
    start: usize,
    end: usize,
    kind: Kind,
    name: String,
    attributes: Vec<(String, String)>,
}

/// The element `xml` holds, read into a [`Node`] that is finished.
pub(in crate::canonical) fn node(xml: &str) -> Node {
    let tags = tags(xml.as_bytes());
    let mut at = 0;
    let root = Node::root(tags[0].name.as_bytes()).unwrap();
    build(root, &tags, &mut at, xml.as_bytes())
}

fn tags(xml: &[u8]) -> Vec<Tag> {
    let mut reader = XmlReader::open(xml, &XmlLimits::default()).unwrap();
    let mut buf = Vec::new();
    let mut tags: Vec<Tag> = Vec::new();
    loop {
        let event = reader.read_event_into(&mut buf).unwrap();
        let start = usize::try_from(reader.offset()).unwrap();
        if let Some(last) = tags.last_mut() {
            last.end = start;
        }
        let (kind, name, attributes) = match &event {
            Event::Eof => break,
            Event::Start(e) => (Kind::Open, name(e), attributes(e)),
            Event::Empty(e) => (Kind::Empty, name(e), attributes(e)),
            Event::End(_) => (Kind::Close, String::new(), Vec::new()),
            _ => (Kind::Other, String::new(), Vec::new()),
        };
        let end = xml.len();
        tags.push(Tag {
            start,
            end,
            kind,
            name,
            attributes,
        });
    }
    tags
}

fn name(start: &BytesStart<'_>) -> String {
    start.name().as_ref().to_owned()
}

fn attributes(start: &BytesStart<'_>) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    for attribute in start.attributes() {
        let attribute = attribute.unwrap();
        let value = attribute.normalized_value(XmlVersion::Implicit1_0).unwrap();
        attributes.push((attribute.key.as_ref().to_owned(), value.into_owned()));
    }
    attributes
}

/// Reads the element that starts at `tags[*at]` into `node`, which is empty
/// and is the node of that element, and moves `*at` past it.
fn build(mut node: Node, tags: &[Tag], at: &mut usize, xml: &[u8]) -> Node {
    let tag = &tags[*at];
    for (name, value) in &tag.attributes {
        node.push_attribute(name.as_bytes(), value);
    }
    *at += 1;
    if matches!(tag.kind, Kind::Open) {
        while !matches!(tags[*at].kind, Kind::Close) {
            let child = &tags[*at];
            match child.kind {
                Kind::Open | Kind::Empty => child_of(&mut node, tags, at, xml),
                Kind::Close | Kind::Other => *at += 1,
            }
        }
        *at += 1;
    }
    node.finish()
}

/// Adds the child that starts at `tags[*at]` to `node`, a node when the table
/// lists it among the children of `node` and raw bytes when it does not, and
/// moves `*at` past it.
fn child_of(node: &mut Node, tags: &[Tag], at: &mut usize, xml: &[u8]) {
    let tag = &tags[*at];
    if let Some(child) = node.child_element(tag.name.as_bytes()) {
        node.push_child(build(child, tags, at, xml));
        return;
    }
    let first = *at;
    let mut depth = 0_usize;
    loop {
        match tags[*at].kind {
            Kind::Open => depth += 1,
            Kind::Close => depth -= 1,
            Kind::Empty | Kind::Other => {}
        }
        *at += 1;
        if depth == 0 {
            break;
        }
    }
    let bytes = xml[tags[first].start..tags[*at - 1].end].to_vec();
    let local = (!tag.name.contains(':')).then_some(tag.name.as_bytes());
    node.push_raw(local, bytes);
}
