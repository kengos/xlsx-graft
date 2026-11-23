//! The check of a sheet name, and what it shares with `names/defined.rs`.
//! `is_a1` and `is_r1c1`, which say whether a name is of the form of a cell
//! reference or of an R1C1 one, are shared with `prefix.rs` too, which
//! quotes a sheet name of either form.

mod defined;

pub use defined::check_defined_name;

use super::{SheetNameFault, StructureError};

/// The most UTF-16 code units of a sheet's name.
const MAX_SHEET_NAME: usize = 31;

/// The characters a sheet's name may not hold.
const FORBIDDEN: [char; 7] = ['[', ']', ':', '*', '?', '/', '\\'];

/// Checks that `name` can be a sheet's name beside the sheets `sheets`
/// (the other sheets' names: when a sheet is renamed, not its own).
///
/// # Errors
///
/// [`StructureError::SheetNameInvalid`] with the first of
/// [`SheetNameFault`]'s variants, in their order, that `name` breaks, or
/// [`StructureError::SheetNameTaken`] when a sheet of `sheets` has it, letter
/// case aside.
pub fn check_sheet_name<'a>(
    name: &str,
    sheets: impl IntoIterator<Item = &'a str>,
) -> Result<(), StructureError> {
    sheet_name(name, &mut sheets.into_iter())
}

/// [`check_sheet_name`], over a single type of iterator.
fn sheet_name(name: &str, sheets: &mut dyn Iterator<Item = &str>) -> Result<(), StructureError> {
    let fault = first(&[
        (name.is_empty(), SheetNameFault::Empty),
        (
            name.encode_utf16().nth(MAX_SHEET_NAME).is_some(),
            SheetNameFault::TooLong,
        ),
        (name.contains(FORBIDDEN), SheetNameFault::Character),
        (
            name.starts_with('\'') || name.ends_with('\''),
            SheetNameFault::Apostrophe,
        ),
        // Excel's own screens do not take a control character, and a raw
        // tab, LF or CR in an attribute reads back as a space (XML 1.0
        // §3.3.3). U+FFFE and U+FFFF are no character of XML 1.0 (§2.2).
        (
            name.chars()
                .any(|c| c < ' ' || c == '\u{fffe}' || c == '\u{ffff}'),
            SheetNameFault::Control,
        ),
        (name.to_lowercase() == "history", SheetNameFault::Reserved),
    ]);
    if let Some(reason) = fault {
        return Err(StructureError::SheetNameInvalid {
            name: name.to_owned(),
            reason,
        });
    }
    match same_name(name, sheets) {
        Some(other) => Err(StructureError::SheetNameTaken {
            name: name.to_owned(),
            other: other.to_owned(),
        }),
        None => Ok(()),
    }
}

/// The rule of the first of `faults` that is broken.
pub(super) fn first<T: Copy>(faults: &[(bool, T)]) -> Option<T> {
    faults
        .iter()
        .find(|(broken, _)| *broken)
        .map(|&(_, fault)| fault)
}

/// The first of `others` that is `name`, letter case aside.
pub(super) fn same_name<'a>(
    name: &str,
    mut others: &mut dyn Iterator<Item = &'a str>,
) -> Option<&'a str> {
    let folded = name.to_lowercase();
    Iterator::find(&mut others, |other| other.to_lowercase() == folded)
}

/// Whether `name` is one to three ASCII letters and then at least
/// `digits` ASCII digits, nothing more: the form of a cell reference (a
/// column when `digits` is 0), with no limit on the column, so it refuses
/// more than Excel does.
pub(super) fn is_a1(name: &str, digits: usize) -> bool {
    let rest = name.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    let letters = name.len().saturating_sub(rest.len());
    (1..=3).contains(&letters) && rest.len() >= digits && rest.bytes().all(|b| b.is_ascii_digit())
}

/// Whether `name` is of the form of an R1C1 reference: `R`, `C`, `RC`,
/// `R1`, `C1`, `R1C1`, in either letter case.
pub(super) fn is_r1c1(name: &str) -> bool {
    let row = name.strip_prefix(['R', 'r']);
    let rest = row.map_or(name, |after| {
        after.trim_start_matches(|c: char| c.is_ascii_digit())
    });
    match rest.strip_prefix(['C', 'c']) {
        Some(columns) => columns.bytes().all(|b| b.is_ascii_digit()),
        None => row.is_some() && rest.is_empty(),
    }
}
