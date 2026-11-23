//! The prefix a sheet is written with in a formula: ECMA-376-1:2016
//! §18.17.2.3 puts a sheet's name in quotes unless it reads as a name
//! alone.

use super::names::{is_a1, is_r1c1};

/// Whether `sheet` is written in quotes in a formula: unless it is a
/// letter, `_` or `\` and then letters, digits, `_` and `.`, and is not the
/// form of a cell or a column (`A1`, `AB`), of an R1C1 reference or
/// `TRUE` or `FALSE`, which a formula would read as that, and does not start
/// with a function prefix (`_xlfn.X`), so that a reader never takes a
/// prefix for a function's name.
#[must_use]
pub fn needs_quotes(sheet: &str) -> bool {
    let mut characters = sheet.chars();
    let plain = characters
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '\\')
        && characters.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    !plain
        || is_a1(sheet, 0)
        || is_r1c1(sheet)
        || sheet.eq_ignore_ascii_case("TRUE")
        || sheet.eq_ignore_ascii_case("FALSE")
        || !xlsx_graft_formula::function_prefix(sheet).is_empty()
}

/// The prefix of `first`, or of the 3-D reference of `first` and `last`,
/// without its `!`. One name that needs quotes quotes the two together
/// (`'S 1:S3'`), a `'` in it doubled. The names are not checked: see
/// [`check_sheet_name`](super::check_sheet_name).
#[must_use]
pub fn sheet_prefix(first: &str, last: Option<&str>) -> String {
    let joined = last.map_or_else(|| first.to_owned(), |last| format!("{first}:{last}"));
    if needs_quotes(first) || last.is_some_and(needs_quotes) {
        format!("'{}'", joined.replace('\'', "''"))
    } else {
        joined
    }
}
