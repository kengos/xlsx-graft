//! [`rewrite`]: an XML part read and written again with some children of
//! its root left out and others added after them, everything else as it
//! was. The content types stream and the Relationships parts are both
//! this: a root with a child element for each entry.

use crate::error::WriteError;
use std::borrow::Cow;
use std::io::{BufRead, Write};
use xlsx_graft_xml::events::{BytesEnd, BytesStart, Event};
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlRewriter, XmlVersion};

/// The children of the root that an edit touches.
pub(super) struct Children<'a> {
    /// The namespace of the child elements (the root's too).
    pub(super) namespace: &'a str,
    /// The local name of the child elements.
    pub(super) name: &'a str,
}

/// Reads `original` and writes it to `out`, leaving out each child of the
/// root named as `children` says that `leave_out` is true for, and after
/// the last child of the root writing one more child for each list of
/// attributes in `add`, as an empty element named as the root is prefixed
/// (`<ct:Types>` has `<ct:Override .../>`). The attribute values are
/// escaped as XML needs.
///
/// # Errors
///
/// [`WriteError::Invalid`] for XML that is refused ([`XmlLimits`] included),
/// [`WriteError::Io`] or the error `original` or `out` carries.
pub(super) fn rewrite(
    original: &mut dyn BufRead,
    out: &mut dyn Write,
    limits: &XmlLimits,
    children: &Children<'_>,
    leave_out: impl Fn(&BytesStart<'_>) -> bool,
    add: &[Vec<(&str, &str)>],
) -> Result<(), WriteError> {
    let mut rewriter = XmlRewriter::new(original, out, limits);
    let mut buf = Vec::new();
    // How many elements are open, and the depth a child left out was
    // opened at while its end is not read yet.
    let (mut depth, mut leaving_out) = (0_usize, None);
    while let Some(event) = rewriter.next(&mut buf)? {
        let before = depth;
        match &event {
            Event::Start(_) => depth = depth.saturating_add(1),
            Event::End(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
        if leaving_out.is_some() {
            let closes = matches!(event, Event::End(_)) && Some(depth) == leaving_out;
            leaving_out = leaving_out.filter(|_| !closes);
            continue;
        }
        let child = match &event {
            Event::Start(start) | Event::Empty(start) if before == 1 => {
                is_child(&rewriter, start, children) && leave_out(start)
            }
            _ => false,
        };
        match &event {
            Event::Start(_) if child => leaving_out = Some(before),
            Event::Empty(_) if child => {}
            Event::End(end) if depth == 0 => {
                insert(&mut rewriter, end.name().as_ref(), children, add)?;
                rewriter.write(event)?;
            }
            Event::Empty(root) if before == 0 => {
                let name = root.name().as_ref().to_owned();
                rewriter.write(Event::Start(root.to_owned()))?;
                insert(&mut rewriter, &name, children, add)?;
                rewriter.write(Event::End(BytesEnd::new(name)))?;
            }
            _ => rewriter.write(event)?,
        }
    }
    Ok(())
}

/// The value of the attribute `key` of `start`, as an XML reader gives it:
/// line ends and entities normalized. `None` when `start` has no such
/// attribute or it cannot be read.
pub(super) fn attribute(start: &BytesStart<'_>, key: &str) -> Option<String> {
    let found = start
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)?;
    let value = found.normalized_value(XmlVersion::Implicit1_0).ok()?;
    Some(Cow::into_owned(value))
}

/// Whether `start` is an element of the kind `children` names.
fn is_child<R: BufRead, W: Write>(
    rewriter: &XmlRewriter<R, W>,
    start: &BytesStart<'_>,
    children: &Children<'_>,
) -> bool {
    let (namespace, local) = rewriter.resolve_element(start.name());
    let bound =
        matches!(namespace, ResolveResult::Bound(Namespace(ns)) if ns == children.namespace);
    bound && local.as_ref() == children.name
}

/// Writes the children `add` lists, named as the root `root` is prefixed.
fn insert<R: BufRead, W: Write>(
    rewriter: &mut XmlRewriter<R, W>,
    root: &str,
    children: &Children<'_>,
    add: &[Vec<(&str, &str)>],
) -> Result<(), WriteError> {
    let prefix = root
        .rfind(':')
        .map_or("", |at| root.get(..=at).unwrap_or_default());
    for attributes in add {
        let name = format!("{prefix}{}", children.name);
        let element = BytesStart::new(name).with_attributes(attributes.iter().copied());
        rewriter.write(Event::Empty(element))?;
    }
    Ok(())
}
