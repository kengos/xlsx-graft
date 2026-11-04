//! The sheets' checks: a name Excel does not take, and a `sheetId` two
//! sheets share. Excel for Mac repairs the workbook for each, renaming the
//! sheet or giving it another `sheetId`.
//!
//! A name is checked as MS-OI29500 says Excel takes one (on ECMA-376-1:2016
//! §18.2.19, note a): at most 31 characters, counted in UTF-16 code units
//! as Excel counts them, and the characters of a quoted sheet name in a
//! formula (§18.17.2.3, `sheet-name-special`): none of `[ ] : * ? / \`,
//! and no `'` first or last. Each way a name breaks it is reported.
//! `History`, which Excel's own screens refuse, Excel opens as it is, so
//! it is not checked.

use super::SheetEntry;
use std::collections::BTreeMap;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, SheetNameReason};

/// The most UTF-16 code units of a sheet's name.
const MAX_NAME: usize = 31;

/// The characters a sheet's name may not hold.
const FORBIDDEN: [char; 7] = ['[', ']', ':', '*', '?', '/', '\\'];

/// Pushes at `at` each problem of `sheets` the module lists.
pub(super) fn check(sheets: &[SheetEntry], at: &Location, diagnostics: &mut Diagnostics) {
    let mut ids: BTreeMap<u32, &str> = BTreeMap::new();
    for sheet in sheets {
        let name = sheet.name.as_str();
        for reason in faults(name) {
            let diagnostic = Diagnostic::SheetNameInvalid {
                name: Echo::new(name),
                reason,
            };
            diagnostics.push(diagnostic, at.clone());
        }
        if let Some(other) = ids.get(&sheet.sheet_id) {
            let diagnostic = Diagnostic::SheetIdDuplicate {
                name: Echo::new(name),
                other: Echo::new(other),
                sheet_id: u64::from(sheet.sheet_id),
            };
            diagnostics.push(diagnostic, at.clone());
        } else {
            ids.insert(sheet.sheet_id, name);
        }
    }
}

/// Each way `name` breaks the rules the module gives.
fn faults(name: &str) -> impl Iterator<Item = SheetNameReason> {
    [
        (name.is_empty(), SheetNameReason::Empty),
        (
            name.encode_utf16().nth(MAX_NAME).is_some(),
            SheetNameReason::TooLong,
        ),
        (name.contains(FORBIDDEN), SheetNameReason::Character),
        (
            name.starts_with('\'') || name.ends_with('\''),
            SheetNameReason::Apostrophe,
        ),
    ]
    .into_iter()
    .filter_map(|(broken, reason)| broken.then_some(reason))
}
