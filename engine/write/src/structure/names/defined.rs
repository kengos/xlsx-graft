//! The check of a defined name.

use super::{first, is_a1, is_r1c1, same_name};
use crate::structure::{DefinedNameFault, StructureError};

/// The most UTF-16 code units of a defined name.
const MAX_DEFINED_NAME: usize = 255;

/// The built-in names ECMA-376-1:2016 §18.2.5 lists that a workbook
/// writes, after `_xlnm.`. The eighth, `Database`, names a range that comes
/// from a database data source, which nothing the writer writes holds
/// (the verifier refuses data connections), so `_xlnm.Database` is refused
/// like an unknown `_xlnm.` name.
const BUILT_IN: [&str; 7] = [
    "Print_Area",
    "Print_Titles",
    "_FilterDatabase",
    "Criteria",
    "Extract",
    "Consolidate_Area",
    "Sheet_Title",
];

/// Checks that `name` can be a defined name, scoped to a sheet when
/// `sheet_scoped` and to the workbook otherwise, beside the tables
/// `tables` (their names).
///
/// # Errors
///
/// [`StructureError::DefinedNameInvalid`] with the first of
/// [`DefinedNameFault`]'s variants, in their order, that `name` breaks, or
/// [`StructureError::DefinedNameTable`] when a table of `tables` has it,
/// letter case aside.
pub fn check_defined_name<'a>(
    name: &str,
    sheet_scoped: bool,
    tables: impl IntoIterator<Item = &'a str>,
) -> Result<(), StructureError> {
    defined_name(name, sheet_scoped, &mut tables.into_iter())
}

/// [`check_defined_name`], over a single type of iterator.
fn defined_name(
    name: &str,
    sheet_scoped: bool,
    tables: &mut dyn Iterator<Item = &str>,
) -> Result<(), StructureError> {
    let built_in = after_prefix(name, "_xlnm.");
    let is_built_in = built_in.is_some_and(|rest| {
        BUILT_IN
            .iter()
            .any(|known| known.eq_ignore_ascii_case(rest))
    });
    // The reader reads a word with such a prefix as a function (`_xlpm.` as
    // a LAMBDA parameter), so the engine would not see a defined name that
    // starts with one.
    let function = !xlsx_graft_formula::function_prefix(name).is_empty();
    let fault = first(&[
        (!can_be_named(name), DefinedNameFault::Character),
        (!name.chars().all(is_xml_char), DefinedNameFault::Xml),
        (
            name.encode_utf16().nth(MAX_DEFINED_NAME).is_some(),
            DefinedNameFault::TooLong,
        ),
        (is_a1(name, 1) || is_r1c1(name), DefinedNameFault::Reference),
        (
            name.eq_ignore_ascii_case("TRUE") || name.eq_ignore_ascii_case("FALSE"),
            DefinedNameFault::Boolean,
        ),
        (
            (built_in.is_some() && !is_built_in) || function,
            DefinedNameFault::ReservedPrefix,
        ),
        (is_built_in && !sheet_scoped, DefinedNameFault::BuiltinScope),
    ]);
    if let Some(reason) = fault {
        return Err(StructureError::DefinedNameInvalid {
            name: name.to_owned(),
            reason,
        });
    }
    match same_name(name, tables) {
        Some(table) => Err(StructureError::DefinedNameTable {
            name: name.to_owned(),
            table: table.to_owned(),
        }),
        None => Ok(()),
    }
}

/// The spelling a built-in name is written with: `_xlnm.` and the name of
/// [`BUILT_IN`] it is, however `name` writes its letters (`_XLNM.print_area`
/// is `_xlnm.Print_Area`). `None` for a name that is not a built-in one.
pub(crate) fn builtin_spelling(name: &str) -> Option<String> {
    let rest = after_prefix(name, "_xlnm.")?;
    BUILT_IN
        .iter()
        .find(|known| known.eq_ignore_ascii_case(rest))
        .map(|known| format!("_xlnm.{known}"))
}

/// What follows `prefix` at the start of `name`, ASCII letter case aside.
fn after_prefix<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    name.get(..prefix.len())
        .filter(|start| start.eq_ignore_ascii_case(prefix))
        .and_then(|_| name.get(prefix.len()..))
}

/// Whether XML 1.0 can hold `c` (its `Char` production, §2.2).
fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}')
}

/// Whether a formula can name `name`: the rule of `xlsx-graft-verify`'s
/// defined name check, minus two forms it lets through. ECMA-376-1:2016
/// §18.17.2.5 as MS-OI29500 writes it (note a): a letter, `_` or `\` first,
/// then letters, digits, `_`, `\` and `.`. Past ASCII, MS-OI29500 admits
/// whatever Unicode calls a character, so only white space, a control
/// character, and a digit first are refused.
///
/// The two forms are refused here and not there, because the writer does not
/// write a form admitted only because Excel opens it (README, Writing): a
/// name with a `?`, which the grammar does not list but Excel opens as it is,
/// and `\` alone or with one character, which MS-OI29500 says Office does
/// not load (note c).
fn can_be_named(name: &str) -> bool {
    let mut characters = name.chars();
    let backslash = name.starts_with('\\') && name.chars().count() <= 2;
    !backslash
        && characters.next().is_some_and(starts)
        && characters.all(|c| starts(c) || c.is_numeric() || c == '.')
}

/// Whether `c` may start a defined name.
fn starts(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_alphabetic() || c == '_' || c == '\\'
    } else {
        !(c.is_whitespace() || c.is_control() || c.is_numeric())
    }
}
