//! `SheetEdit::apply` and the rewriter it gives `PackageEdit::replace`: the
//! worksheet is read once, and written with each root child that a call
//! changed written by its concern (`merges`, `columns`, `rows`, `panes`,
//! `protection`), the kinds the part lacks written where `ChildPlacer` says,
//! and every other byte as it was.

use super::SheetEdit;
use super::pass::{copy, through};
use super::refuse::refuse;
use super::tag::{name_of, write_end};
use super::wrapped::classify;
use crate::error::WriteError;
use crate::part::PartRewriter;
use crate::sheet::{ChildPlacer, Due, RootChild, Slot, WorksheetChild};
use crate::{PackageEdit, WriteLimits};
use std::collections::BTreeSet;
use std::io::{BufRead, Read, Seek, Write};
use xlsx_graft_xml::XmlRewriter;
use xlsx_graft_xml::events::{BytesStart, Event};

/// The rewriter every concern writes through: the part's input and output
/// as trait objects.
pub(super) type Rw<'a> = XmlRewriter<&'a mut dyn BufRead, &'a mut dyn Write>;

impl SheetEdit {
    /// Whether the calls change the part.
    fn changed(&self) -> bool {
        self.merges.changed()
            || !self.columns.is_empty()
            || !self.rows.is_empty()
            || self.panes.changed()
            || self.protection.changed()
    }

    /// The kinds of child the calls change.
    fn wanted(&self) -> Vec<WorksheetChild> {
        let kinds = [
            (self.merges.changed(), WorksheetChild::MergeCells),
            (!self.columns.is_empty(), WorksheetChild::Cols),
            (!self.rows.is_empty(), WorksheetChild::SheetData),
            (self.panes.changed(), WorksheetChild::SheetViews),
            (self.protection.changed(), WorksheetChild::SheetProtection),
        ];
        (kinds.into_iter())
            .filter_map(|(wanted, kind)| wanted.then_some(kind))
            .collect()
    }

    /// Hands `edit` a rewriter that makes the changes the calls recorded, or
    /// nothing when no call changes the part: when no call was made, or the
    /// only calls were `unprotect` on a sheet with no `sheetProtection` and
    /// `freeze_panes(0, 0)` on a sheet with no `pane`. A `merge` or an
    /// `unmerge` that returned `Ok` always changes the part, and so does a
    /// `set_column_width` or a `set_row_height` (even to the value the part
    /// has: the `col` and `row` elements of the part are not held, so a
    /// width or a height that is already there is not known).
    ///
    /// The part is read once as it is when the edit is written, and written
    /// with the elements the calls changed, in the order of the schema
    /// (ECMA-376-1:2016 §18.3.1.99), and every other byte as it was: the
    /// elements the engine does not know, an `mc:AlternateContent` and the
    /// cells included. Elements written new take the prefix of the root.
    /// A new `col` is written before the first `col` of the part whose `min` is
    /// past the width's columns, or before the end of the last `cols`; a part
    /// whose `col`s are not in ascending order of `min` gets no `col` for a
    /// width in columns below the greatest `max` read that no `col` covers
    /// (`columns/stream.rs`). A `row` whose number was
    /// already seen (rows out of order, or twice) and a `col` whose `min` or
    /// `max` is not a column are written as they are.
    ///
    /// # A part that is refused
    ///
    /// The part is refused when an element of a kind the calls changed
    /// (`mergeCells`, `cols`, `sheetData`, `sheetViews` or `sheetProtection`,
    /// only the ones this edit changes) is out of the schema's order (it
    /// comes after the place its kind was written), is wrapped in
    /// `mc:AlternateContent`, or is a second element of a kind that does not
    /// repeat (`sheetViews`, `sheetData`, `mergeCells`, `sheetProtection`; see
    /// `WorksheetChild::repeats`): the rewriter returns [`WriteError::Invalid`]
    /// (`xml_schema_invalid`, `unexpected_element`, naming the kind, at the
    /// offset of the element or of the wrapper in the part it reads), so
    /// [`PackageEdit::finish`] pushes the diagnostic at the part, writes no
    /// zip and returns `Ok(None)`. An element of a kind no call changed passes
    /// through wherever it is.
    ///
    /// # Call `StructureEdit::apply` first
    ///
    /// When the same `PackageEdit` also takes a
    /// [`StructureEdit`](crate::structure::StructureEdit), call its `apply`
    /// before `SheetEdit::apply`, as `PackageEdit::replace` says: the rewriters of a
    /// part run in the order of the `replace` calls, each reading the output
    /// of the one before. `StructureEdit::apply` decides from the original
    /// part whether to rewrite it, so with its rewriter first, the decision
    /// and the rewriter's input are the same bytes. The rewriter of
    /// `SheetEdit::apply` writes through the `tabSelected` and the formulas
    /// that `StructureEdit` wrote.
    /// In the reverse order the rewriter of `StructureEdit` would read what
    /// the rewriter of `SheetEdit::apply` wrote, and a formula that the
    /// rewriter of `SheetEdit::apply` (or a later edit that rewrites formulas)
    /// writes with a sheet name would be read as written before the renames,
    /// and rewritten again. The caller that makes both calls keeps
    /// that order.
    ///
    /// # Errors
    ///
    /// [`WriteError::PartMissing`] when the calls changed the part and it is
    /// not in `edit`: it was removed (`StructureEdit` deleted its sheet).
    /// While `edit` is written, [`WriteError::Invalid`] (an
    /// `xml_schema_invalid` with `invalid_value` for `r`) when any
    /// `set_row_height` call was made and a `row` has an `r` that is not 1 to
    /// 1,048,576, or has no `r` and follows row 1,048,576.
    pub fn apply<'r, R: Read + Seek>(
        self,
        edit: &mut PackageEdit<'r, R>,
    ) -> Result<(), WriteError> {
        if !self.changed() {
            return Ok(());
        }
        let part = self.part.clone();
        edit.replace(&part, Box::new(SheetRewriter::new(self)))
    }
}

/// Writes the part again with the edit's changes.
struct SheetRewriter {
    edit: SheetEdit,
    placer: ChildPlacer,
    /// The root's prefix and `:`, or `""`, for the elements written new.
    prefix: String,
    /// The kinds that do not repeat of which an element came so far.
    seen: BTreeSet<WorksheetChild>,
    /// How many `cols` came so far.
    col_elements: usize,
    /// The last column a `col` read covers (0 before the first).
    covered: u16,
}

impl SheetRewriter {
    fn new(edit: SheetEdit) -> Self {
        Self {
            placer: ChildPlacer::new(edit.wanted()),
            edit,
            prefix: String::new(),
            seen: BTreeSet::new(),
            col_elements: 0,
            covered: 0,
        }
    }

    /// The limits the part is read under.
    fn limits(&self) -> WriteLimits {
        self.edit.limits
    }

    /// Notes the root's prefix.
    fn root(&mut self, root: &BytesStart<'_>) {
        let name = name_of(root);
        self.prefix =
            (name.split_once(':')).map_or(String::new(), |(prefix, _)| format!("{prefix}:"));
    }

    /// The root ends (or is empty): writes what is due last.
    fn finish(&mut self, rw: &mut Rw<'_>) -> Result<(), WriteError> {
        for due in self.placer.finish() {
            self.write_due(rw, due)?;
        }
        Ok(())
    }

    /// Writes the kind `due.child` as a new element, when the part had none
    /// (and the kind is not edited where it stands).
    fn write_due(&self, rw: &mut Rw<'_>, due: Due) -> Result<(), WriteError> {
        let (edit, prefix) = (&self.edit, self.prefix.as_str());
        let out = rw.output_mut();
        match due.child {
            WorksheetChild::MergeCells if !due.found => edit.merges.write_new(out, prefix)?,
            WorksheetChild::Cols if !due.found => edit.columns.write_new(out, prefix)?,
            WorksheetChild::SheetData if !due.found => edit.rows.write_new(out, prefix)?,
            WorksheetChild::SheetViews if !due.found => edit.panes.write_new(out, prefix)?,
            WorksheetChild::SheetProtection => edit.protection.write_new(out, prefix)?,
            _ => {}
        }
        Ok(())
    }

    /// A child of the root: the element `start`, with an end tag of its own
    /// when `opened`.
    fn child(
        &mut self,
        rw: &mut Rw<'_>,
        start: BytesStart<'static>,
        opened: bool,
    ) -> Result<(), WriteError> {
        let offset = rw.offset();
        let (child, held) = classify(rw, &start, opened)?;
        let placement = self.placer.start(child);
        for due in placement.due {
            self.write_due(rw, due)?;
        }
        match (placement.slot, child) {
            (Slot::Wanted, RootChild::Table(kind)) => {
                // The placer gives `Wanted` to each of the elements of a kind
                // that come one after another.
                if !kind.repeats() && !self.seen.insert(kind) {
                    return Err(refuse(offset, kind));
                }
                self.wanted(rw, kind, start, opened)
            }
            (Slot::Late | Slot::Wrapped, RootChild::Table(kind) | RootChild::Wrapped(kind)) => {
                Err(refuse(offset, kind))
            }
            _ => copy(rw, start, opened, held),
        }
    }

    /// The element `start` of the wanted kind `kind`.
    fn wanted(
        &mut self,
        rw: &mut Rw<'_>,
        kind: WorksheetChild,
        start: BytesStart<'static>,
        opened: bool,
    ) -> Result<(), WriteError> {
        let (edit, prefix) = (&self.edit, self.prefix.as_str());
        match kind {
            WorksheetChild::MergeCells => edit.merges.stream(rw, prefix, (start, opened)),
            WorksheetChild::Cols => {
                let index = self.col_elements;
                self.col_elements += 1;
                let at = (index, self.covered);
                self.covered = edit.columns.stream(rw, prefix, (start, opened), at)?;
                Ok(())
            }
            WorksheetChild::SheetData => edit.rows.stream(rw, prefix, (start, opened)),
            WorksheetChild::SheetViews => edit.panes.stream(rw, prefix, (start, opened)),
            // The protection is written at its `Due`.
            _ => through(rw, usize::from(opened), false),
        }
    }
}

impl PartRewriter for SheetRewriter {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut rw: Rw<'_> = XmlRewriter::new(original, out, &self.limits().xml);
        let mut buf = Vec::new();
        // 0 outside the root, 1 inside it.
        let mut depth = 0;
        while let Some(event) = rw.next(&mut buf)? {
            match (depth, event) {
                (0, Event::Start(root)) => {
                    self.root(&root);
                    rw.write(Event::Start(root))?;
                    depth = 1;
                }
                (0, Event::Empty(root)) => {
                    self.root(&root);
                    let name = name_of(&root);
                    rw.write(Event::Start(root))?;
                    self.finish(&mut rw)?;
                    write_end(&mut rw, &name)?;
                }
                (1, Event::Start(child)) => self.child(&mut rw, child.into_owned(), true)?,
                (1, Event::Empty(child)) => self.child(&mut rw, child.into_owned(), false)?,
                (1, Event::End(root)) => {
                    self.finish(&mut rw)?;
                    rw.write(Event::End(root))?;
                    depth = 0;
                }
                (_, other) => rw.write(other)?,
            }
        }
        Ok(())
    }
}
