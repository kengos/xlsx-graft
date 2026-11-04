//! Tests for [`Structure::check`], each on a workbook shaped like the one
//! opened in Excel for Mac 16.114.3 to find what it repairs, damaged one
//! way: `sheets.rs` covers the sheets, `names.rs` the defined names and
//! `tables.rs` the tables.

mod names;
mod sheets;
mod tables;

use super::{NameEntry, SheetEntry, Structure, TableEntry};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};

/// The workbook part's name.
const WORKBOOK: &str = "/xl/workbook.xml";

/// The location of a problem of a sheet or a defined name.
fn workbook() -> Location {
    Location::default().with_part(WORKBOOK)
}

/// The location of a problem of the table of the part `part`.
fn table_at(part: &str) -> Location {
    Location::default().with_part(part)
}

/// A sheet named `name` with the `sheetId` `id`.
fn sheet(name: &str, id: u32) -> SheetEntry {
    SheetEntry {
        name: name.to_owned(),
        sheet_id: id,
    }
}

/// A defined name `name`, scoped to the sheet at `scope` if any, holding
/// `formula`.
fn name(name: &str, scope: Option<u32>, formula: &str) -> NameEntry {
    NameEntry {
        name: name.to_owned(),
        local_sheet_id: scope,
        formula: formula.to_owned(),
    }
}

/// The table `name` of the part `/xl/tables/table<id>.xml`, with the `id`
/// `id`, on the sheet at `sheet` if any, covering `range`.
fn table(id: u32, name: &str, sheet: Option<u32>, range: &str) -> TableEntry {
    TableEntry {
        part: format!("/xl/tables/table{id}.xml"),
        sheet,
        id,
        display_name: name.to_owned(),
        range: range.to_owned(),
    }
}

/// A sound workbook: the sheets データ, 集計 and Extra, the defined name
/// `Base_Name` on データ!$A$1, and the table `T_Data` on データ!A1:S2.
fn book() -> Structure {
    Structure {
        workbook_part: WORKBOOK.to_owned(),
        sheets: vec![sheet("データ", 1), sheet("集計", 2), sheet("Extra", 8)],
        defined_names: vec![name("Base_Name", None, "データ!$A$1")],
        tables: vec![table(1, "T_Data", Some(0), "A1:S2")],
    }
}

/// What checking `structure` reports.
fn check(structure: &Structure) -> Vec<(Diagnostic, Location)> {
    let mut diagnostics = Diagnostics::new();
    structure.check(&mut diagnostics);
    diagnostics
        .finish()
        .into_iter()
        .map(|record| (record.diagnostic, record.location))
        .collect()
}

#[test]
fn a_sound_workbook_reports_nothing() {
    assert_eq!(check(&book()), []);
    assert_eq!(check(&Structure::default()), []);
}

#[test]
fn every_problem_is_reported_not_only_the_first() {
    let mut book = book();
    book.sheets.push(sheet("x/y", 8));
    book.defined_names
        .push(name("Bad Name", None, "データ!$A$1"));
    book.tables.push(table(1, "t_data", Some(0), "A2:B3"));
    let codes: Vec<&str> = (check(&book).iter())
        .map(|(diagnostic, _)| diagnostic.code().as_str())
        .collect();
    assert_eq!(
        codes,
        [
            "sheet_name_invalid",
            "sheet_id_duplicate",
            "defined_name_invalid",
            "table_range_overlap",
            "table_name_duplicate",
            "table_id_duplicate",
        ]
    );
}
