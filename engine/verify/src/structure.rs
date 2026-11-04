//! [`Structure`]: what Excel repairs, or reads otherwise than it says, as
//! it opens a workbook, found in the workbook part's sheets and defined
//! names and the table parts' tables. It reads no XML: its caller hands it
//! what those parts say.
//!
//! What Excel does was found by opening a workbook damaged one way at a
//! time in Excel for Mac 16.114.3. What makes it repair the workbook, its
//! repair dialog shown, is an error; what it changes without a word is a
//! notice; what it opens as it is is not checked.
//!
//! - **Sheets** (`sheets.rs`): a name Excel does not take, and a `sheetId`
//!   two sheets share. A name two sheets share is the workbook reader's
//!   `sheet_name_duplicate`, not checked again here.
//! - **Defined names** (`names.rs`): a name a formula cannot name, a
//!   `localSheetId` past the last sheet, a name two of one scope share
//!   (Excel keeps the later), and a reference to a sheet the workbook does
//!   not have (Excel reads it as one to another workbook).
//! - **Tables** (`tables.rs`): two of one sheet that overlap, a
//!   `displayName` another table's or a defined name's, and an `id` two
//!   tables share.
//!
//! Names are compared as Excel compares them, letter case aside: lowered
//! as Unicode lowers them, as the workbook reader compares sheet names.
//! Each check keeps a lowered copy or two of each name it compares, and a
//! few bytes per sheet, defined name or table beside it, so what it keeps
//! grows with what its caller holds already; two tables are compared only
//! where their rows meet, so the work is quadratic only in the tables of
//! one sheet whose rows all meet, which a package's entry limit bounds.

mod names;
mod sheets;
mod tables;

use xlsx_graft_diagnostics::{Diagnostics, Location};

/// What a workbook's parts say of its sheets, defined names and tables,
/// for [`Structure::check`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Structure {
    /// The name of the workbook part, where a problem of a sheet or a
    /// defined name is reported.
    pub workbook_part: String,
    /// The workbook part's sheets, in its order.
    pub sheets: Vec<SheetEntry>,
    /// The workbook part's defined names, in its order.
    pub defined_names: Vec<NameEntry>,
    /// The tables, one per table part: those of each sheet in the sheets'
    /// order, each sheet's in the order its `tableParts` lists them, then
    /// those of no one sheet. Of two tables, the later is the one a
    /// problem they share is reported at.
    pub tables: Vec<TableEntry>,
}

/// One sheet of the workbook part (`sheets/sheet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetEntry {
    /// Its `name`, its `_xHHHH_` escapes undone.
    pub name: String,
    /// Its `sheetId`.
    pub sheet_id: u32,
}

/// One defined name of the workbook part (`definedNames/definedName`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntry {
    /// Its `name`.
    pub name: String,
    /// Its `localSheetId`: the index of the sheet it is scoped to, or
    /// `None` for the workbook's.
    pub local_sheet_id: Option<u32>,
    /// Its formula, as the part stores it.
    pub formula: String,
}

/// The table of a table part (`table`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableEntry {
    /// The name of the table part, where a problem of the table is
    /// reported.
    pub part: String,
    /// The index of the sheet whose part relates to it, as a
    /// `localSheetId` counts sheets; `None` when no one sheet does.
    pub sheet: Option<u32>,
    /// Its `id`.
    pub id: u32,
    /// Its `displayName`.
    pub display_name: String,
    /// Its `ref`, as written (`A1:S2`).
    pub range: String,
}

impl Structure {
    /// Pushes into `diagnostics` every problem the module lists, each
    /// once.
    pub fn check(&self, diagnostics: &mut Diagnostics) {
        let at = Location::default().with_part(&self.workbook_part);
        sheets::check(&self.sheets, &at, diagnostics);
        names::check(&self.defined_names, &self.sheets, &at, diagnostics);
        tables::check(
            &self.tables,
            &self.defined_names,
            self.sheets.len(),
            diagnostics,
        );
    }
}

/// `name` as Excel compares it, letter case aside.
fn fold(name: &str) -> String {
    name.to_lowercase()
}

#[cfg(test)]
mod tests;
