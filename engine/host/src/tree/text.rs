//! [`TreeText`]: the tree as text, for a person. Not a contract: it may
//! change.

use super::{Linked, Tree, TreePart, TreeRelationship};
use crate::report::Counts;
use crate::text::DiagnosticsText;
use crate::view::View;
use std::fmt;

mod words;

use words::{grouped, short_content_type, short_type};

/// How many levels of relationships are written under the package, or under
/// a part written as a root. A part first found at the last level is written
/// as `(continued below)`, and is written again, with what it targets, as a
/// root after the tree unless a later line wrote it.
const MAX_DEPTH: usize = 32;

/// The view of a tree as text: the package at the top, under it each
/// relationship with the part it targets (a part's own relationships under
/// it), then the parts not written while following the relationships from
/// the package, the diagnostics as [`DiagnosticsText`] writes them, and a
/// last line of the number of parts, the counts of the diagnostics listed
/// and whether the view is `ok`.
///
/// ```text
/// / (package)
/// └── officeDocument rId1 → /xl/workbook.xml  [sheet.main+xml, 1,234 B]
///     ├── styles rId3 → /xl/styles.xml  [styles+xml, 400 B]
///     ├── worksheet rId1 → /xl/worksheets/sheet1.xml  [worksheet+xml, 8,201 B]
///     │   └── styles rId1 → /xl/styles.xml  (above)
///     └── hyperlink rId9 → https://example.com/  (external)
/// /xl/worksheets/sheet2.xml  [worksheet+xml, 300 B]  — not referenced
///
/// notice
///   part_unreferenced /xl/worksheets/sheet2.xml — no relationship targets the part, ...
/// 4 parts, 0 errors, 0 warnings, 1 notice — ok
/// ```
///
/// A part written a second time (also through a cycle) is `(above)` and its
/// relationships are not written again; a part whose relationships were not
/// all kept has `(some relationships not listed)` after its media type and
/// size. At most 32 levels are written under the package, the first with no
/// indent: a part first found at the last level ends at `(continued below)`.
/// After the tree come the parts nothing is known to reference, then the
/// parts that ended a line at `(continued below)` (in that order, the ones
/// no later line wrote), then the others a relationship targets that were
/// not written, each as a root with 32 levels under it (`(continued)` at the
/// end of the line of a part that ended a line at `(continued below)`).
///
/// The tree has at most one line per relationship and per part, and a line
/// for the package, and the indent of a line is at most 4 × 31 characters.
/// After the tree come a line saying that the relationships were not all
/// read (when they were not), the diagnostics, and the last line. When the
/// view has an `error`, the tree is replaced by
/// `could not read: <message>`. That the package's own relationships are not
/// all kept is shown as `(some relationships not listed)` after
/// `/ (package)`.
#[derive(Debug, Clone, Copy)]
pub struct TreeText<'a>(pub &'a View<Tree>);

impl fmt::Display for TreeText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let view = self.0;
        if let Some(error) = &view.error {
            writeln!(f, "could not read: {}", error.message)?;
        } else {
            Walk::new(f, &view.content).tree()?;
        }
        if !view.diagnostics.is_empty() {
            writeln!(f)?;
            write!(f, "{}", DiagnosticsText(&view.diagnostics))?;
        }
        let count = view.content.parts.len();
        let noun = if count == 1 { "part" } else { "parts" };
        let verdict = if view.ok { "ok" } else { "not ok" };
        let counts = Counts::tally(&view.diagnostics);
        writeln!(f, "{count} {noun}, {counts} — {verdict}")
    }
}

/// The relationships being written (the package's own or a part's) and how
/// many of them are written.
struct Frame<'a> {
    relationships: &'a [TreeRelationship],
    next: usize,
}

/// Writing `tree` to `f`.
struct Walk<'a, 'f, 'g> {
    f: &'f mut fmt::Formatter<'g>,
    tree: &'a Tree,
    /// Whether `tree.parts[i]` is written, for each `i`.
    visited: Vec<bool>,
    /// Whether `tree.parts[i]` ended a line at `(continued below)`, for each
    /// `i`; `queue` has the order.
    continued: Vec<bool>,
    /// The indexes of the parts that ended a line at `(continued below)`,
    /// in the order they did, each once.
    queue: Vec<usize>,
}

impl<'a, 'f, 'g> Walk<'a, 'f, 'g> {
    fn new(f: &'f mut fmt::Formatter<'g>, tree: &'a Tree) -> Self {
        Self {
            f,
            tree,
            visited: vec![false; tree.parts.len()],
            continued: vec![false; tree.parts.len()],
            queue: Vec::new(),
        }
    }

    /// The package and the parts found by following its relationships, then
    /// the parts not written by that.
    fn tree(&mut self) -> fmt::Result {
        let tree = self.tree;
        write!(self.f, "/ (package)")?;
        if !tree.relationships_known {
            write!(self.f, "  (some relationships not listed)")?;
        }
        writeln!(self.f)?;
        self.descend(&tree.relationships)?;
        // First the parts nothing is known to reference, then the parts that
        // ended a line at `(continued below)`, in that order, and last the
        // others a relationship targets that were not written (a cycle of
        // parts nothing else references).
        for (index, part) in tree.parts.iter().enumerate() {
            if part.referenced != Some(true) && self.first_visit(index) {
                self.root(index, part)?;
            }
        }
        self.roots_continued()?;
        for (index, part) in tree.parts.iter().enumerate() {
            if self.first_visit(index) {
                self.root(index, part)?;
                self.roots_continued()?;
            }
        }
        if !tree.complete && !tree.parts.is_empty() {
            let note =
                "the relationships were not all read, so whether a part is referenced is not known";
            writeln!(self.f, "{note}")?;
        }
        Ok(())
    }

    /// Marks the part at `index` as one that ended a line at
    /// `(continued below)`; false when it already was.
    fn first_continued(&mut self, index: usize) -> bool {
        (self.continued.get_mut(index)).is_some_and(|mark| !std::mem::replace(mark, true))
    }

    /// Whether the part at `index` is written.
    fn visited_at(&self, index: usize) -> bool {
        self.visited.get(index).copied().unwrap_or(true)
    }

    /// The parts that ended a line at `(continued below)`, in that order and
    /// the ones a later line did not write, each as a root, and then the
    /// ones that ended a line under those roots.
    fn roots_continued(&mut self) -> fmt::Result {
        let mut next = 0;
        while let Some(&index) = self.queue.get(next) {
            next = next.saturating_add(1);
            if let Some(part) = self.tree.parts.get(index)
                && self.first_visit(index)
            {
                self.root(index, part)?;
            }
        }
        self.queue.clear();
        Ok(())
    }

    /// Marks the part at `index` written; false when it already was.
    fn first_visit(&mut self, index: usize) -> bool {
        match self.visited.get_mut(index) {
            Some(seen) if !*seen => {
                *seen = true;
                true
            }
            _ => false,
        }
    }

    /// A part not written while following the relationships from the package,
    /// on a line of its own.
    fn root(&mut self, index: usize, part: &'a TreePart) -> fmt::Result {
        write!(self.f, "{}", part.name)?;
        self.describe(part)?;
        if self.continued.get(index).copied().unwrap_or(false) {
            write!(self.f, "  (continued)")?;
        }
        if part.referenced == Some(false) {
            write!(self.f, "  — not referenced")?;
        }
        writeln!(self.f)?;
        self.descend(&part.relationships)
    }

    /// The relationships of `first`, and the relationships of each part
    /// first found under them, depth first, down to [`MAX_DEPTH`] levels.
    ///
    /// `lasts` says, for each level above the one being written, whether the
    /// relationship that was followed is the last of its list, which gives
    /// what each line starts with.
    fn descend(&mut self, first: &'a [TreeRelationship]) -> fmt::Result {
        let mut lasts: Vec<bool> = Vec::new();
        let mut stack = vec![Frame {
            relationships: first,
            next: 0,
        }];
        while let Some(frame) = stack.last_mut() {
            let list = frame.relationships;
            let Some(relationship) = list.get(frame.next) else {
                stack.pop();
                if !stack.is_empty() {
                    lasts.pop();
                }
                continue;
            };
            frame.next = frame.next.saturating_add(1);
            let last = frame.next >= list.len();
            let mut line = String::new();
            for above in &lasts {
                line.push_str(if *above { "    " } else { "│   " });
            }
            line.push_str(if last { "└── " } else { "├── " });
            let kind = short_type(relationship.rel_type.as_str());
            let id = &relationship.id;
            write!(self.f, "{line}{kind} {id} → ")?;
            match &relationship.target.0 {
                Linked::External(text) => writeln!(self.f, "{text}  (external)")?,
                Linked::Part { index, name } => {
                    write!(self.f, "{name}")?;
                    let i = *index;
                    match self.tree.parts.get(i) {
                        Some(part) if stack.len() >= MAX_DEPTH && !self.visited_at(i) => {
                            self.describe(part)?;
                            writeln!(self.f, "  (continued below)")?;
                            if self.first_continued(i) {
                                self.queue.push(i);
                            }
                        }
                        Some(part) if self.first_visit(i) => {
                            self.describe(part)?;
                            writeln!(self.f)?;
                            lasts.push(last);
                            stack.push(Frame {
                                relationships: &part.relationships,
                                next: 0,
                            });
                        }
                        _ => writeln!(self.f, "  (above)")?,
                    }
                }
            }
        }
        Ok(())
    }

    /// `  [<media type>, <size> B]` after a part's name, with `, not
    /// readable` when its zip entry was refused, and `  (some relationships
    /// not listed)` after it when the part's relationships were not all kept.
    fn describe(&mut self, part: &TreePart) -> fmt::Result {
        let content_type = match (&part.content_type, part.content_type_known) {
            (Some(content_type), _) => short_content_type(content_type.as_str()),
            (None, true) => "no content type",
            (None, false) => "content type unknown",
        };
        let unreadable = if part.readable { "" } else { ", not readable" };
        let size = grouped(part.size);
        write!(self.f, "  [{content_type}, {size} B{unreadable}]")?;
        if !part.relationships_known {
            write!(self.f, "  (some relationships not listed)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
