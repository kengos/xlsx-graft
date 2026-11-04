//! The defined names' checks, each reported at the workbook part:
//!
//! - **A name a formula cannot name** (error: Excel for Mac repairs the
//!   workbook and drops the defined name). ECMA-376-1:2016 §18.17.2.5 as
//!   MS-OI29500 writes it for Office (on §18.17.2.5, note a): a letter,
//!   `_` or `\` first, then letters, digits, `_`, `\` and `.`, and `?`,
//!   which the grammar does not list but Excel opens as it is. A letter or
//!   digit of ASCII is one the grammar lists; past ASCII, MS-OI29500 admits
//!   whatever Unicode calls a character, so only white space, a control
//!   character, and a digit first are refused there (Excel repairs `１a`
//!   and `a　b`, and opens `売上・前年` and `a１` as they are). A name of
//!   the form of a cell reference (`A1`), and `\` and one character, which
//!   MS-OI29500 says Office does not load (note c), Excel opens as they
//!   are, so neither is checked.
//! - **A `localSheetId` past the last sheet** (error: Excel repairs the
//!   workbook and drops the defined name).
//! - **Two names of one scope the same, letter case aside** (notice: Excel
//!   keeps the later without a word, MS-OI29500 on §18.2.5, note n). Each
//!   one before the last is reported, naming the last. A name already
//!   reported as one a formula cannot name, or scoped past the last sheet,
//!   is left out: Excel drops it first.
//! - **A reference to a sheet the workbook does not have** (notice: Excel
//!   opens the workbook without a word and reads the reference as one to
//!   a sheet of another workbook). Every sheet a reference or a name in the
//!   formula is prefixed with counts, each of a 3-D reference's two, once
//!   per defined name; one prefixed with another workbook does not. A
//!   formula the formula reader cannot read whole is not looked into.

use super::{NameEntry, SheetEntry, fold};
use std::collections::{BTreeMap, BTreeSet};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};
use xlsx_graft_formula::Location as Prefix;

/// Pushes at `at` each problem of `names` the module lists; `sheets` are
/// the workbook's.
pub(super) fn check(
    names: &[NameEntry],
    sheets: &[SheetEntry],
    at: &Location,
    diagnostics: &mut Diagnostics,
) {
    let known: BTreeSet<String> = sheets.iter().map(|sheet| fold(&sheet.name)).collect();
    // The index of the last defined name of each scope and folded name.
    let mut last: BTreeMap<(Option<u32>, String), usize> = BTreeMap::new();
    let mut kept = Vec::with_capacity(names.len());
    for (index, name) in names.iter().enumerate() {
        let valid = is_valid(&name.name);
        if !valid {
            let diagnostic = Diagnostic::DefinedNameInvalid {
                name: Echo::new(&name.name),
            };
            diagnostics.push(diagnostic, at.clone());
        }
        let scoped = is_in_scope(name.local_sheet_id, sheets.len());
        if let Some(id) = name.local_sheet_id.filter(|_| !scoped) {
            let diagnostic = Diagnostic::DefinedNameSheetIndexInvalid {
                name: Echo::new(&name.name),
                local_sheet_id: u64::from(id),
                sheets: u64::try_from(sheets.len()).unwrap_or(u64::MAX),
            };
            diagnostics.push(diagnostic, at.clone());
        }
        let key = (valid && scoped).then(|| (name.local_sheet_id, fold(&name.name)));
        if let Some(key) = &key {
            last.insert(key.clone(), index);
        }
        kept.push(key);
        for sheet in missing_sheets(&name.formula, &known) {
            let diagnostic = Diagnostic::DefinedNameSheetMissing {
                name: Echo::new(&name.name),
                sheet: Echo::new(&sheet),
            };
            diagnostics.push(diagnostic, at.clone());
        }
    }
    for (index, (name, key)) in names.iter().zip(kept).enumerate() {
        let later = key.and_then(|key| last.get(&key).copied());
        if let Some(later) = later.filter(|&later| later != index) {
            let other = names.get(later).map_or("", |other| other.name.as_str());
            let diagnostic = Diagnostic::DefinedNameDuplicate {
                name: Echo::new(&name.name),
                other: Echo::new(other),
            };
            diagnostics.push(diagnostic, at.clone());
        }
    }
}

/// Whether Excel keeps `name` as it opens a workbook of `sheets` sheets:
/// its name is one a formula can name and its scope is the workbook or a
/// sheet it has. Of two of one scope with the same name, it keeps the
/// later.
pub(super) fn is_kept(name: &NameEntry, sheets: usize) -> bool {
    is_valid(&name.name) && is_in_scope(name.local_sheet_id, sheets)
}

/// Whether `scope` is the workbook's, or a sheet's of `sheets` sheets.
fn is_in_scope(scope: Option<u32>, sheets: usize) -> bool {
    scope.is_none_or(|id| usize::try_from(id).is_ok_and(|id| id < sheets))
}

/// Whether `name` is one a formula can name, as the module says.
fn is_valid(name: &str) -> bool {
    let mut characters = name.chars();
    characters.next().is_some_and(starts)
        && characters.all(|c| starts(c) || c.is_numeric() || c == '.' || c == '?')
}

/// Whether `c` may start a name.
fn starts(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_alphabetic() || c == '_' || c == '\\'
    } else {
        !(c.is_whitespace() || c.is_control() || c.is_numeric())
    }
}

/// The sheets `formula` refers to that are not among `known`, folded, as
/// the module says, each once and as first written.
fn missing_sheets(formula: &str, known: &BTreeSet<String>) -> Vec<String> {
    let facts = xlsx_graft_formula::analyze(formula).facts;
    if !facts.malformed.is_empty() {
        return Vec::new();
    }
    let prefixes = (facts.references.iter().map(|reference| &reference.location))
        .chain(facts.names.iter().map(|name| &name.location));
    let mut seen = BTreeSet::new();
    let mut missing = Vec::new();
    for Prefix { book, sheet } in prefixes {
        let Some(sheet) = sheet.as_deref().filter(|_| book.is_none()) else {
            continue;
        };
        // A sheet's name holds no `:`, so it parts a 3-D reference's two;
        // one that does is `sheet_name_invalid` already.
        if known.contains(&fold(sheet)) {
            continue;
        }
        for sheet in sheet.split(':') {
            let folded = fold(sheet);
            if !known.contains(&folded) && seen.insert(folded) {
                missing.push(sheet.to_owned());
            }
        }
    }
    missing
}
