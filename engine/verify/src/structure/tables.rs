//! The tables' checks, each reported at the table part: two tables of one
//! sheet that overlap, a `displayName` that is another table's or a
//! defined name's, and an `id` two tables share. ECMA-376-1:2016 §18.5.1.2
//! has a `displayName` unique among the tables' and the defined names',
//! and an `id` unique among the tables'; Excel for Mac repairs the
//! workbook for each of the three, dropping the later of two tables that
//! overlap or share an `id`, and renaming the later of two that share a
//! name. The later is the one reported, naming the earlier.
//!
//! A table's name is compared with every defined name's, of any scope, as
//! the standard has it, but for one Excel drops as it opens the workbook
//! (`names.rs`). A table with no sheet, or whose `ref` is no range
//! of cells, overlaps nothing.

use super::names::is_kept;
use super::{NameEntry, TableEntry, fold};
use std::collections::BTreeMap;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};
use xlsx_graft_read::CellRef;

/// The cells a table covers on its sheet, and the table's index.
#[derive(Debug, Clone, Copy)]
struct Area {
    /// The sheet's index.
    sheet: u32,
    /// The first and last rows.
    rows: (u32, u32),
    /// The first and last columns.
    columns: (u16, u16),
    /// The table's index among the tables.
    table: usize,
}

/// Pushes each problem of `tables` the module lists; `names` are the
/// defined names of the workbook, which has `sheets` sheets.
pub(super) fn check(
    tables: &[TableEntry],
    names: &[NameEntry],
    sheets: usize,
    diagnostics: &mut Diagnostics,
) {
    let overlaps = overlaps(tables);
    let defined: BTreeMap<String, &str> = (names.iter())
        .filter(|name| is_kept(name, sheets))
        .map(|name| (fold(&name.name), name.name.as_str()))
        .collect();
    let mut seen: BTreeMap<String, &str> = BTreeMap::new();
    let mut ids: BTreeMap<u32, &str> = BTreeMap::new();
    for (index, table) in tables.iter().enumerate() {
        let name = table.display_name.as_str();
        let at = || Location::default().with_part(&table.part);
        if let Some(&other) = overlaps.get(&index) {
            let other = tables.get(other).map_or("", |t| t.display_name.as_str());
            let diagnostic = Diagnostic::TableRangeOverlap {
                name: Echo::new(name),
                other: Echo::new(other),
            };
            diagnostics.push(diagnostic, at());
        }
        let folded = fold(name);
        let duplicate = match (defined.get(&folded), seen.get(&folded)) {
            (Some(other), _) => Some((*other, true)),
            (None, Some(other)) => Some((*other, false)),
            (None, None) => None,
        };
        if let Some((other, defined_name)) = duplicate {
            let diagnostic = Diagnostic::TableNameDuplicate {
                name: Echo::new(name),
                other: Echo::new(other),
                defined_name,
            };
            diagnostics.push(diagnostic, at());
        }
        seen.entry(folded).or_insert(name);
        if let Some(other) = ids.get(&table.id) {
            let diagnostic = Diagnostic::TableIdDuplicate {
                name: Echo::new(name),
                other: Echo::new(other),
                id: u64::from(table.id),
            };
            diagnostics.push(diagnostic, at());
        } else {
            ids.insert(table.id, name);
        }
    }
}

/// The first table before it that each table overlaps, by the index of
/// each that overlaps one.
fn overlaps(tables: &[TableEntry]) -> BTreeMap<usize, usize> {
    let mut areas: Vec<Area> = (tables.iter().enumerate())
        .filter_map(|(table, entry)| area(entry, table))
        .collect();
    areas.sort_by_key(|area| (area.sheet, area.rows.0));
    let mut first = BTreeMap::new();
    for (index, area) in areas.iter().enumerate() {
        // Sorted by sheet and first row: the areas after this one whose
        // rows meet it come next, and the first that does not ends them.
        let below = areas.iter().skip(index.saturating_add(1));
        let meeting = below.take_while(|b| b.sheet == area.sheet && b.rows.0 <= area.rows.1);
        for other in meeting {
            if other.columns.0 <= area.columns.1 && area.columns.0 <= other.columns.1 {
                let (earlier, later) = if area.table < other.table {
                    (area.table, other.table)
                } else {
                    (other.table, area.table)
                };
                let kept = first.entry(later).or_insert(earlier);
                *kept = (*kept).min(earlier);
            }
        }
    }
    first
}

/// The cells `table`, the `index`th, covers, if it has a sheet and its
/// `ref` is a cell or a range of cells.
fn area(table: &TableEntry, index: usize) -> Option<Area> {
    let sheet = table.sheet?;
    let (first, last) = table
        .range
        .split_once(':')
        .unwrap_or((&table.range, &table.range));
    let (first, last) = (CellRef::parse(first).ok()?, CellRef::parse(last).ok()?);
    Some(Area {
        sheet,
        rows: ordered(first.row(), last.row()),
        columns: ordered(first.column(), last.column()),
        table: index,
    })
}

/// `a` and `b`, the lesser first.
fn ordered<T: Ord>(a: T, b: T) -> (T, T) {
    if a <= b { (a, b) } else { (b, a) }
}
