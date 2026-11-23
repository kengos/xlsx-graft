//! `StylesEdit::rewrite`: writes the styles part again with the items that
//! were added, and writes every other event as it was read, as `XmlRewriter`
//! does, so a part that nothing was added to comes out byte for byte.
//!
//! The items go before the end tag of their list, and the list's `count`
//! (when it has one) is written again. A list that the part lacks is
//! written whole before the first element that follows it in the schema's
//! order of the `styleSheet`'s children (ECMA-376-1:2016 §18.8.39), or
//! before the end of the `styleSheet`. The items are written with the prefix
//! of their list's name, and a new list with the prefix of the `styleSheet`'s.

mod tag;

use super::StyleError;
use super::list::Shape;
use super::scan::{Scan, refuse};
use crate::canonical::write::{write_list, write_number_format};
use std::io::{BufRead, Write};
use xlsx_graft_diagnostics::SchemaReason;
use xlsx_graft_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

pub(super) use tag::counted;
use tag::name_of;

/// The children of the `styleSheet` in the order of the schema.
const ORDER: [&str; 11] = [
    "numFmts",
    "fonts",
    "fills",
    "borders",
    "cellStyleXfs",
    "cellXfs",
    "cellStyles",
    "dxfs",
    "tableStyles",
    "colors",
    "extLst",
];

/// What is added to a list the part has: the bytes of the items, how many
/// items the list then has, and whether its element has a `count`.
struct Addition {
    items: Vec<u8>,
    total: usize,
    count: bool,
}

/// The lists the part has that are added to, each with its position in
/// [`ORDER`].
type Present = Vec<(usize, Addition)>;

/// The lists the part lacks that are added to, each with its position in
/// [`ORDER`] and the bytes of the whole list element.
type Missing = Vec<(usize, Vec<u8>)>;

/// Writes the part `original` to `out` with what `scan` has added.
pub(super) fn rewrite(
    scan: &Scan,
    limits: &XmlLimits,
    original: &mut dyn BufRead,
    out: &mut dyn Write,
) -> Result<(), StyleError> {
    let (present, missing) = edits(scan);
    let mut pass = Pass {
        rewriter: XmlRewriter::new(original, out, limits),
        depth: 0,
        namespace: None,
        present,
        missing,
        appending: None,
    };
    let mut buf = Vec::new();
    while let Some(event) = pass.rewriter.next(&mut buf)? {
        pass.event(event)?;
    }
    // A list `scan` read that `original` lacks has no place to write what is
    // added to it: `original` is not the part that was read.
    match pass.present.first() {
        Some((order, _)) => Err(refuse(
            pass.rewriter.offset(),
            SchemaReason::MissingElement,
            ORDER[*order].as_bytes(),
        )),
        None => Ok(()),
    }
}

/// The lists that have additions, in the order of [`ORDER`], each with its
/// position there: those the part has, and those it lacks (written whole,
/// as the bytes of the list element).
fn edits(scan: &Scan) -> (Present, Missing) {
    let numbers = &scan.numbers;
    let prefix = |shape: &Shape| {
        if shape.present {
            shape.prefix.clone()
        } else {
            scan.prefix.clone()
        }
    };
    let mut formats = Vec::new();
    for (id, code) in &numbers.added {
        write_number_format(&prefix(&numbers.shape), *id, code, &mut formats);
    }
    let mut lists = vec![(0, &numbers.shape, formats, numbers.count())];
    let others = [
        (1, &scan.fonts),
        (2, &scan.fills),
        (3, &scan.borders),
        (5, &scan.cell_xfs),
    ];
    for (order, list) in others {
        let mut items = Vec::new();
        for node in list.added() {
            node.write(&prefix(&list.shape), &mut items);
        }
        lists.push((order, &list.shape, items, list.items.len()));
    }
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for (order, shape, items, total) in lists.into_iter().filter(|list| !list.2.is_empty()) {
        if shape.present {
            let count = shape.count;
            present.push((
                order,
                Addition {
                    items,
                    total,
                    count,
                },
            ));
        } else {
            let mut whole = Vec::new();
            write_list(&scan.prefix, ORDER[order], total, &items, &mut whole);
            missing.push((order, whole));
        }
    }
    (present, missing)
}

/// The rewriting in progress.
struct Pass<R, W> {
    rewriter: XmlRewriter<R, W>,
    /// How many elements are open at the event being written.
    depth: usize,
    /// The namespace of the root element.
    namespace: Option<String>,
    /// The lists the part has that are added to, not yet reached.
    present: Present,
    /// The lists the part lacks, not yet written, as bytes.
    missing: Missing,
    /// The items to write before the end tag of the list whose start tag was
    /// written last.
    appending: Option<Vec<u8>>,
}

impl<R: BufRead, W: Write> Pass<R, W> {
    /// Writes `event`, with what is added where it asks.
    fn event(&mut self, event: Event<'_>) -> Result<(), StyleError> {
        match event {
            Event::Start(start) => self.start(start, true),
            Event::Empty(start) => self.start(start, false),
            Event::End(end) => self.end(end),
            other => Ok(self.rewriter.write(other)?),
        }
    }

    /// Writes the start tag (or, when `opened` is false, the empty element)
    /// `start`.
    fn start(&mut self, start: BytesStart<'_>, opened: bool) -> Result<(), StyleError> {
        let depth = self.depth;
        self.depth += usize::from(opened);
        if depth == 0 {
            self.namespace = self.namespace_of(&start);
            if opened || self.missing.is_empty() {
                return self.pass(start, opened);
            }
            // `<styleSheet/>`: the lists go inside it.
            let end = BytesEnd::new(name_of(&start));
            self.rewriter.write(Event::Start(start))?;
            self.write_missing(usize::MAX)?;
            return Ok(self.rewriter.write(Event::End(end))?);
        }
        let order = (depth == 1).then(|| self.order_of(&start)).flatten();
        let Some(order) = order else {
            return self.pass(start, opened);
        };
        self.write_missing(order)?;
        if self.missing.iter().any(|(held, _)| *held == order) {
            // `original` has a list that `read` did not find.
            let name = ORDER[order].as_bytes();
            let offset = self.rewriter.offset();
            return Err(refuse(offset, SchemaReason::UnexpectedElement, name));
        }
        let at = self.present.iter().position(|(held, _)| *held == order);
        let Some(addition) = at.map(|at| self.present.remove(at).1) else {
            return self.pass(start, opened);
        };
        let start = if addition.count {
            counted(&start, addition.total)
        } else {
            start
        };
        if opened {
            self.appending = Some(addition.items);
            return self.pass(start, true);
        }
        // `<numFmts/>`: the items go inside it.
        let end = BytesEnd::new(name_of(&start));
        self.rewriter.write(Event::Start(start))?;
        self.write(&addition.items)?;
        Ok(self.rewriter.write(Event::End(end))?)
    }

    /// Writes the end tag `end`, after the items added to the list it closes
    /// or the lists still to add to the `styleSheet`.
    fn end(&mut self, end: BytesEnd<'_>) -> Result<(), StyleError> {
        if self.depth == 2
            && let Some(items) = self.appending.take()
        {
            self.write(&items)?;
        }
        if self.depth == 1 {
            self.write_missing(usize::MAX)?;
        }
        self.depth = self.depth.saturating_sub(1);
        Ok(self.rewriter.write(Event::End(end))?)
    }

    /// Writes `start` as it is.
    fn pass(&mut self, start: BytesStart<'_>, opened: bool) -> Result<(), StyleError> {
        let event = if opened {
            Event::Start(start)
        } else {
            Event::Empty(start)
        };
        Ok(self.rewriter.write(event)?)
    }

    /// Writes the lists the part lacks that come before the child at
    /// `order` of [`ORDER`].
    fn write_missing(&mut self, order: usize) -> Result<(), StyleError> {
        while self.missing.first().is_some_and(|(held, _)| *held < order) {
            let (_, bytes) = self.missing.remove(0);
            self.write(&bytes)?;
        }
        Ok(())
    }

    /// Writes `bytes`, XML that is already written, as it is.
    fn write(&mut self, bytes: &[u8]) -> Result<(), StyleError> {
        let text = BytesText::from_escaped(String::from_utf8_lossy(bytes));
        Ok(self.rewriter.write(Event::Text(text))?)
    }

    /// The namespace of the element `start`.
    fn namespace_of(&self, start: &BytesStart<'_>) -> Option<String> {
        let (resolved, _) = self.rewriter.resolve_element(start.name());
        bound(resolved).map(str::to_owned)
    }

    /// The position in [`ORDER`] of the element `start` when it is a child
    /// of the `styleSheet` named there, in the root's namespace.
    fn order_of(&self, start: &BytesStart<'_>) -> Option<usize> {
        let (resolved, local) = self.rewriter.resolve_element(start.name());
        (bound(resolved) == self.namespace.as_deref())
            .then(|| ORDER.iter().position(|&name| name == local.as_ref()))
            .flatten()
    }
}

/// The namespace `resolved` binds, if it binds one.
fn bound(resolved: ResolveResult<'_>) -> Option<&str> {
    match resolved {
        ResolveResult::Bound(Namespace(ns)) => Some(ns),
        _ => None,
    }
}
