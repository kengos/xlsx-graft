//! Reading a styles part into the lists an edit adds to: `numFmts`, `fonts`,
//! `fills`, `borders` and `cellXfs` item by item (`scan/item.rs`), whether
//! `cellStyleXfs` has a cell style, and how each list is written. The root is the
//! `styleSheet` of SpreadsheetML, Transitional or Strict. Everything else is
//! skipped (read and not kept), except that the greatest `numFmtId`
//! attribute of any element anywhere is noted (`scan/events.rs`).
//!
//! A list with a child that is not its item (`mc:AlternateContent`, an
//! unknown name) is refused whatever is added later: the number of the next
//! item would not be known. Text that is not white space in a list or an
//! item is refused too, and so is a list that is there twice, and a list
//! inside an element of another namespace that is a child of the
//! `styleSheet` (an `mc:AlternateContent` holding `fonts`): which list the
//! items are numbered in would not be known. What is inside the other
//! children of the `styleSheet` in the namespace of the styles part
//! (`dxfs`, `extLst`) is not looked at.

mod events;
mod item;

use super::list::{List, NumberFormats, Shape};
use super::{StyleError, StyleKind, StyleLimits, limit};
use crate::canonical::Node;
pub(super) use events::refuse;
use events::{Ev, Tag, Walk, uint};
use xlsx_graft_diagnostics::SchemaReason;

/// A styles part read.
#[derive(Debug, Default)]
pub(super) struct Scan {
    /// The prefix of the `styleSheet` element's name; empty when it has none.
    pub(super) prefix: Vec<u8>,
    pub(super) numbers: NumberFormats,
    pub(super) fonts: List,
    pub(super) fills: List,
    pub(super) borders: List,
    pub(super) cell_xfs: List,
    /// Whether the part's `cellStyleXfs` has at least one `xf`, which a
    /// new cell format can refer to as its `xfId` 0.
    pub(super) cell_style_xfs: bool,
}

/// Reads the styles part `bytes` under `limits`.
pub(super) fn scan(bytes: &[u8], limits: &StyleLimits) -> Result<Scan, StyleError> {
    let mut walk = Walk::open(bytes, &limits.xml)?;
    let root = root(&mut walk)?;
    let mut scan = Scan {
        prefix: root.prefix(),
        ..Scan::default()
    };
    // The root is the only element at the top, so what follows its start is
    // its children, and then its end and the end of the part.
    while let Some(event) = walk.next()? {
        match event {
            Ev::Start(tag) => scan.top(&mut walk, tag, true, limits)?,
            Ev::Empty(tag) => scan.top(&mut walk, tag, false, limits)?,
            _ => {}
        }
    }
    scan.numbers.max_id = walk.max_id;
    Ok(scan)
}

/// Reads up to the start of the root element, which must be a `styleSheet`
/// in the namespace of SpreadsheetML; gives its tag.
fn root(walk: &mut Walk<'_>) -> Result<Tag, StyleError> {
    let mut found = None;
    while found.is_none()
        && let Some(event) = walk.next()?
    {
        found = match event {
            Ev::Start(tag) | Ev::Empty(tag) => Some(tag),
            _ => None,
        };
    }
    let root = found.and_then(|tag| {
        let ns = tag.ns.filter(|_| tag.local == b"styleSheet");
        ns.map(|ns| (tag, ns))
    });
    let (tag, ns) =
        root.ok_or_else(|| refuse(0, SchemaReason::UnexpectedElement, b"styleSheet"))?;
    walk.namespace = ns;
    Ok(tag)
}

impl Scan {
    /// Reads the child `tag` of the `styleSheet`: a list that is edited, or
    /// another element, which is skipped (an element of another namespace
    /// is refused when a list is inside it).
    fn top(
        &mut self,
        walk: &mut Walk<'_>,
        tag: Tag,
        opened: bool,
        limits: &StyleLimits,
    ) -> Result<(), StyleError> {
        let max = limits.max_items_read;
        let main = tag.ns == Some(walk.namespace);
        match (main, tag.local.as_slice()) {
            (true, b"numFmts") => self.read_numbers(walk, &tag, opened, max),
            (true, b"fonts") => {
                let list = &mut self.fonts;
                read_list(walk, list, ("font", StyleKind::Font), &tag, opened, max)
            }
            (true, b"fills") => {
                let list = &mut self.fills;
                read_list(walk, list, ("fill", StyleKind::Fill), &tag, opened, max)
            }
            (true, b"borders") => {
                let list = &mut self.borders;
                read_list(walk, list, ("border", StyleKind::Border), &tag, opened, max)
            }
            (true, b"cellXfs") => {
                let list = &mut self.cell_xfs;
                read_list(walk, list, ("xf", StyleKind::CellFormat), &tag, opened, max)
            }
            (true, b"cellStyleXfs") => {
                let xfs = &mut self.cell_style_xfs;
                members(walk, &tag, opened, |walk, member, opened| {
                    *xfs |= member.ns == Some(walk.namespace) && member.local == b"xf";
                    walk.skip(opened)
                })
            }
            (true, _) => walk.skip(opened),
            (false, _) => walk.skip_foreign(opened),
        }
    }

    /// Reads the `numFmts` list `tag`.
    fn read_numbers(
        &mut self,
        walk: &mut Walk<'_>,
        tag: &Tag,
        opened: bool,
        max: u32,
    ) -> Result<(), StyleError> {
        let numbers = &mut self.numbers;
        begin(&mut numbers.shape, tag)?;
        members(walk, tag, opened, |walk, item, opened| {
            if item.ns != Some(walk.namespace) || item.local != b"numFmt" {
                return Err(item.unexpected());
            }
            let id = item.attribute("numFmtId").and_then(uint);
            if let (Some(id), Some(code)) = (id, item.attribute("formatCode")) {
                numbers.define(id, code.to_owned());
            }
            numbers.read += 1;
            limit(StyleKind::NumberFormat, numbers.read, max)?;
            walk.skip(opened)
        })
    }
}

/// Reads the list `tag` into `list`, whose items are the elements named
/// `kind.0`, and are `kind.1` for an error.
fn read_list(
    walk: &mut Walk<'_>,
    list: &mut List,
    kind: (&'static str, StyleKind),
    tag: &Tag,
    opened: bool,
    max: u32,
) -> Result<(), StyleError> {
    begin(&mut list.shape, tag)?;
    members(walk, tag, opened, |walk, member, opened| {
        let node = (member.ns == Some(walk.namespace) && member.local == kind.0.as_bytes())
            .then(|| Node::root(kind.0.as_bytes()))
            .flatten();
        let node = node.ok_or_else(|| member.unexpected())?;
        let node = walk.build(node, &member, opened)?;
        list.push(node);
        limit(kind.1, list.items.len(), max)
    })
}

/// Notes that the list `tag` is there and how it is written; refuses a list
/// that is there already.
fn begin(shape: &mut Shape, tag: &Tag) -> Result<(), StyleError> {
    if shape.present {
        return Err(tag.unexpected());
    }
    *shape = Shape {
        present: true,
        prefix: tag.prefix(),
        count: tag.attribute("count").is_some(),
    };
    Ok(())
}

/// Reads the children of the list `tag`, calling `each` with each one;
/// refuses text that is not white space.
fn members(
    walk: &mut Walk<'_>,
    tag: &Tag,
    opened: bool,
    mut each: impl FnMut(&mut Walk<'_>, Tag, bool) -> Result<(), StyleError>,
) -> Result<(), StyleError> {
    let mut open = opened;
    while open && let Some(event) = walk.next()? {
        match event {
            Ev::Start(member) => each(walk, member, true)?,
            Ev::Empty(member) => each(walk, member, false)?,
            Ev::Text { blank: false, .. } => return Err(walk.unexpected_text(&tag.name)),
            Ev::End(_) => open = false,
            Ev::Text { .. } | Ev::Other(_) => {}
        }
    }
    Ok(())
}
