//! [`Worksheets`]: the parts of a workbook's sheets, each checked as a
//! worksheet ([`SheetCheck`]) as its part is read, against what the
//! workbook holds that its cells refer to, read before it ([`Counts`]).
//!
//! A part is checked when the workbook part's sheets name it, whatever
//! kind of sheet they say it is: the checks read a `worksheet` root alone,
//! so a chartsheet, a dialogsheet or a macro sheet is passed over.

use crate::sheet::{Counts, SheetCheck};
use std::collections::{BTreeMap, BTreeSet};
use xlsx_graft_diagnostics::{Diagnostics, Limit};

/// The sheets' parts to check, and what their checks found of their
/// tables.
#[derive(Debug)]
pub(crate) struct Worksheets {
    /// What the cells are checked against.
    pub(crate) counts: Counts,
    max_merged_cells: Limit,
    /// By the index of each sheet's part: the `Id`s of its relationships
    /// to table parts.
    tables: BTreeMap<usize, BTreeSet<String>>,
    /// By the index of each sheet's part read: those of the `Id`s its
    /// `tableParts` names, in its order.
    pub(crate) named: BTreeMap<usize, Vec<String>>,
}

impl Worksheets {
    /// No part to check yet, those added to keep `max_merged_cells`
    /// merged ranges each, and to check their cells against nothing known
    /// until the counts are.
    pub(crate) fn new(max_merged_cells: Limit) -> Self {
        Self {
            counts: Counts::default(),
            max_merged_cells,
            tables: BTreeMap::new(),
            named: BTreeMap::new(),
        }
    }

    /// Checks the part at `index`, whose relationships to table parts have
    /// the `Id`s `tables`.
    pub(crate) fn add(&mut self, index: usize, tables: BTreeSet<String>) {
        self.tables.insert(index, tables);
    }

    /// The checks of the part at `index`, named `part`, pushing into
    /// `diagnostics`, if it is one to check.
    pub(crate) fn check<'a>(
        &self,
        index: usize,
        part: &str,
        diagnostics: &'a mut Diagnostics,
    ) -> Option<SheetCheck<'a>> {
        let tables = self.tables.get(&index)?.clone();
        let max = self.max_merged_cells;
        Some(SheetCheck::new(part, self.counts, max, tables, diagnostics))
    }

    /// Notes that the part at `index` was read, its `tableParts` naming
    /// the relationships `named`, in order.
    pub(crate) fn read(&mut self, index: usize, named: Vec<String>) {
        self.named.insert(index, named);
    }
}
