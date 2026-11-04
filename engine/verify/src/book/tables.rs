//! The tables of a workbook, read from the first `table` of each table
//! part: their names, which a formula's words are resolved against, and
//! what [`Structure`] checks of them, put in its order once the sheets'
//! parts are read ([`Book::structure`]).
//!
//! A table's sheet is the one whose part relates to its part, unless none
//! or two do, whether or not the sheet's `tableParts` names it: whether
//! Excel reads a table it does not name was not tried. Its `id`,
//! `displayName` and `ref` are what [`Structure`] checks, as long as the
//! formulas' reading keeps an attribute; a table without all three, or
//! whose `id` is not an `xsd:unsignedInt`, is not checked.

use super::Book;
use crate::collect::{FormulaSink, TableTag};
use crate::count::Unsigned;
use crate::worksheets::Worksheets;
use crate::{Names, Structure, TableEntry};
use std::collections::BTreeMap;
use std::io::{Read, Seek};
use xlsx_graft_package::{Package, Target};

/// The media type of a table part (ECMA-376-1:2016 §12.3.21).
pub(super) const TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";

/// The first table of a table part.
#[derive(Debug)]
pub(super) struct Table {
    /// The index of its part.
    part: usize,
    /// The index of its sheet's part, if it has a sheet.
    sheet_part: Option<usize>,
    /// What [`Structure`] checks of it.
    entry: TableEntry,
}

impl Table {
    /// Where it stands in [`Structure::tables`]: its sheet's index, then
    /// where its sheet's `tableParts` names it (`positions`, by its
    /// sheet's part and its own), each past every other when there is
    /// none.
    fn order(&self, positions: &BTreeMap<(usize, usize), usize>) -> (u32, usize) {
        let (Some(sheet), Some(at)) = (self.entry.sheet, self.sheet_part) else {
            return (u32::MAX, usize::MAX);
        };
        let position = positions.get(&(at, self.part)).copied();
        (sheet, position.unwrap_or(usize::MAX))
    }
}

impl Book {
    /// What [`Structure`] checks, its tables in its order now that the
    /// sheets' parts, held in `worksheets`, are read from `package`;
    /// `None` when the workbook part was not read by its schema.
    pub(crate) fn structure<R: Read + Seek>(
        &mut self,
        package: &Package<R>,
        worksheets: &Worksheets,
    ) -> Option<Structure> {
        let mut structure = self.structure.take()?;
        let mut positions = BTreeMap::new();
        for (&at, named) in &worksheets.named {
            let relationships =
                (package.parts().get(at)).map_or(&[][..], |part| part.relationships());
            let mut targets = BTreeMap::new();
            for relationship in relationships {
                if let Target::Internal(target) = relationship.target() {
                    targets.insert(relationship.id(), *target);
                }
            }
            for (position, id) in named.iter().enumerate() {
                if let Some(&target) = targets.get(id.as_str()) {
                    positions.entry((at, target)).or_insert(position);
                }
            }
        }
        let mut tables = std::mem::take(&mut self.tables);
        tables.sort_by_key(|table| table.order(&positions));
        structure.tables = tables.into_iter().map(|table| table.entry).collect();
        Some(structure)
    }
}

/// A sink that keeps the names of the first table of a table part, and
/// what [`Structure`] checks of it.
#[derive(Debug)]
pub(super) struct Tables {
    /// The names kept so far.
    names: Names,
    /// The tables kept so far.
    tables: Vec<Table>,
    /// The part being read, and its sheet and that sheet's part, while it
    /// has given no table: only its first counts.
    open: Option<(usize, Option<(u32, usize)>)>,
}

impl Tables {
    /// A sink adding to `names`.
    pub(super) fn new(names: Names) -> Self {
        Self {
            names,
            tables: Vec::new(),
            open: None,
        }
    }

    /// Reads the part at `index` next, of the sheet `sheet` and its part.
    pub(super) fn part(&mut self, index: usize, sheet: Option<(u32, usize)>) {
        self.open = Some((index, sheet));
    }

    /// The names, and the tables in the parts' order.
    pub(super) fn finish(self) -> (Names, Vec<Table>) {
        (self.names, self.tables)
    }
}

impl FormulaSink for Tables {
    fn formula(&mut self, _part: &str, _at: &str, _text: &str) {}

    fn too_long(&mut self, _part: &str, _at: &str, _size: u64) {}

    fn markup(&mut self, _part: &str, _at: &str) {}

    fn defined_name(&mut self, _: &str, _: &str, _: &str, _: Option<&str>, _: &str) {}

    fn table(&mut self, part: &str, table: &TableTag) {
        let Some((index, sheet)) = self.open.take() else {
            return;
        };
        for name in [&table.name, &table.display_name].into_iter().flatten() {
            self.names = std::mem::take(&mut self.names).table(name);
        }
        let id = (table.id.as_deref()).and_then(Unsigned::parse);
        let (Some(id), Some(display_name), Some(range)) = (
            id.and_then(|id| u32::try_from(id).ok()),
            table.display_name.clone(),
            table.range.clone(),
        ) else {
            return;
        };
        self.tables.push(Table {
            part: index,
            sheet_part: sheet.map(|(_, at)| at),
            entry: TableEntry {
                part: part.to_owned(),
                sheet: sheet.map(|(sheet, _)| sheet),
                id,
                display_name,
                range,
            },
        });
    }
}
