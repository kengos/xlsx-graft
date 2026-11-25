//! [`StructureReport`]: what [`StructureEdit::apply`](super::StructureEdit::apply)
//! read and left as it was, for the caller to handle.
//!
//! A rename or a delete of a sheet rewrites the formulas of the places
//! `xlsx_graft_formula::places` marks `Rewrite`, in the parts of the kinds
//! `apply` rewrites. A formula it does not rewrite (one at another place, a
//! malformed one that holds the sheet's name, one with markup, a structured
//! reference to a table removed, one too long to read, one in which the end of
//! the operand of a deleted sheet's prefix is unknown, or one in a part that
//! cannot be read) is kept as it is, and each is a [`KeptReference`] with
//! the [`KeptReason`] it was kept for. The report adds no diagnostic
//! (the diagnostic codes are closed): a caller that wants one reads this.
//!
//! What a report holds is bounded by [`MAX_KEPT`] references and, for each,
//! by [`MAX_AT_BYTES`] bytes of its `at`. Every other field of a reference
//! is shared with a value `apply` already holds (the name of the part, of a
//! sheet changed and of a table removed, and the element or attribute of
//! `xlsx_graft_formula::places` it is at),
//! so the input's size does not decide how much memory the report takes.

use std::sync::Arc;

/// The references a [`StructureEdit`](super::StructureEdit) left as they
/// were, in the order of the parts of the package, then the workbook part's
/// defined names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructureReport {
    kept: Vec<KeptReference>,
    omitted: u64,
}

/// The limit on the references a report holds: at most this many; the ones
/// beyond it are counted in [`StructureReport::omitted`].
pub(super) const MAX_KEPT: usize = 16_384;

/// The byte limit of a reference's `at`.
pub(super) const MAX_AT_BYTES: usize = 1_024;

impl StructureReport {
    /// A report of `kept`, and `omitted` references past [`MAX_KEPT`].
    pub(super) fn new(kept: Vec<KeptReference>, omitted: u64) -> Self {
        Self { kept, omitted }
    }

    /// How many references were left as they were and are not in
    /// [`kept`](Self::kept): a report holds at most 16,384.
    #[must_use]
    pub fn omitted(&self) -> u64 {
        self.omitted
    }

    /// The references left as they were, in the order they are read.
    #[must_use]
    pub fn kept(&self) -> &[KeptReference] {
        &self.kept
    }
}

/// A formula, or a part, left as it was.
///
/// A reference shares what it holds with the report's other references and
/// with `apply`: [`part`](Self::part), [`sheet`](Self::sheet) and
/// [`table`](Self::table) are one allocation each for all the references
/// that name the same part, sheet or table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptReference {
    part: Arc<str>,
    place: &'static str,
    at: Option<Arc<str>>,
    sheet: Option<Arc<str>>,
    table: Option<Arc<str>>,
    reason: KeptReason,
}

impl KeptReference {
    /// A kept reference in `part` at `place`.
    pub(super) fn new(
        part: Arc<str>,
        place: &'static str,
        at: Option<Arc<str>>,
        sheet: Option<Arc<str>>,
        table: Option<Arc<str>>,
        reason: KeptReason,
    ) -> Self {
        Self {
            part,
            place,
            at,
            sheet,
            table,
            reason,
        }
    }

    /// The name of the part (`/xl/charts/chart1.xml`).
    #[must_use]
    pub fn part(&self) -> &str {
        &self.part
    }

    /// Where in the part: the local name of the element whose text is the
    /// formula (`f`, `oldFormula`), or `element@attribute` (`cfvo@val`) as
    /// `xlsx_graft_formula::places` writes it; `""` for a part that could not
    /// be read. It is always a name of that table, so it is short.
    #[must_use]
    pub fn place(&self) -> &str {
        self.place
    }

    /// Where the place is: the `r` of the cell (for an `f`), the `sqref` of
    /// the data validation or conditional format it is in, or
    /// `definedName <name>` for a defined name; `None` when there is none.
    /// Each is cut at 1,024 bytes (`definedName ` included), at a character
    /// boundary.
    #[must_use]
    pub fn at(&self) -> Option<&str> {
        self.at.as_deref()
    }

    /// The sheet the reference named: the name the workbook gave the sheet
    /// before the first change that touched the reference (a rename or a
    /// delete), however the reference spells it; `None` for a part that could
    /// not be read, a reference to a table, and a formula too long to read.
    #[must_use]
    pub fn sheet(&self) -> Option<&str> {
        self.sheet.as_deref()
    }

    /// The table a structured reference named, for
    /// [`KeptReason::DeletedTable`]: the table's `name` or `displayName` as
    /// the table's part writes it, whichever the formula matched; `None`
    /// for any other reason.
    #[must_use]
    pub fn table(&self) -> Option<&str> {
        self.table.as_deref()
    }

    /// Why it was left as it was.
    #[must_use]
    pub fn reason(&self) -> KeptReason {
        self.reason
    }
}

/// Why a reference was left as it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptReason {
    /// The formula is at a place outside the closed list the edit rewrites
    /// (`cfvo@val`, `sp@textlink`, a revision's `oldFormula`...), and it
    /// names a sheet that was renamed or deleted.
    Outside,
    /// The formula is at a place the edit rewrites, `xlsx_graft_formula::analyze`
    /// reports a malformed place in it, and its text holds the name of a sheet
    /// that was renamed or deleted.
    Malformed,
    /// The element holds another element beside its text, and which of the
    /// two Excel reads depends on markup compatibility, and its text holds the
    /// name of a sheet that was renamed or deleted.
    Markup,
    /// The part could not be read as XML to its end (its zip entry or its
    /// XML is damaged), so it was not searched and not changed.
    Unreadable,
    /// A structured reference to a table whose part was removed with a
    /// deleted sheet: it is left as written. The table is
    /// [`KeptReference::table`].
    DeletedTable,
    /// A pivot cache's `worksheetSource` names a deleted sheet. The cache
    /// is kept as it is.
    DeletedSource,
    /// The text of the formula, and the comments and processing instructions
    /// inside it, add up to more than 64 KiB (65,536 bytes; verify counts
    /// only the text against its `max_formula_bytes`, which has the same
    /// value), so it was not read and is not rewritten, whatever it names.
    TooLong,
    /// A delete of a sheet found a prefix that names only that sheet
    /// (`Sheet1!`) in a formula of a place the edit rewrites, and the prefix
    /// has no operand (`xlsx_graft_formula::SheetPrefix::operand` is `None`):
    /// the token after the prefix is none of a cell, a range, a table
    /// reference, a name, an error value, or a call whose `(` is closed
    /// (`Sheet1!(A1)`, `Sheet1!1`, `Sheet1!TRUE`, `Sheet1!"x"`,
    /// `Sheet1!_xlfn.X` with no `(`), the formula ends or goes on with an
    /// operator (`Sheet1!`, `Sheet1!+1`), the call or `(` is never closed
    /// (`Sheet1!MyFn(1`), or `analyze` reads the prefix as the start of a
    /// 3-D prefix (`Sheet1!A1:Sheet1!B2`, whose `A1:Sheet1!` it takes as the
    /// prefix). The prefix is left as written, because `#REF!` in place of
    /// the prefix alone would leave an operand Excel cannot read. It still
    /// names the deleted sheet, so a sheet of the same name added later is
    /// the sheet it names. The formula's other prefixes are rewritten as
    /// usual. At a place outside the edit's list the formula is
    /// [`KeptReason::Outside`] instead.
    UnknownOperand,
}

#[cfg(test)]
mod tests;
