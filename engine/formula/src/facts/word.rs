//! What a word is: its prefixes, and whether it is a cell, a column or a
//! row (ECMA-376-1:2016 §18.17.2.3.1, in the bounds of a worksheet).

/// The prefixes Excel writes before a function or parameter name, compared
/// without regard to letter case: `_xlfn.` on a function newer than the
/// file format, `_xlws.` on one only a worksheet allows ([MS-XLSX] §2.2.2),
/// `_xludf.` on a user-defined one, `_xleta.` on a function passed as a
/// value, `_xlpm.` on a LAMBDA parameter (Excel's own files).
const PREFIXES: [&str; 5] = ["_xlfn.", "_xlws.", "_xludf.", "_xleta.", "_xlpm."];

/// The last column, `XFD`.
const MAX_COLUMN: u32 = 16_384;

/// The last row.
const MAX_ROW: u32 = 1_048_576;

/// `word` split into the prefixes it starts with, as written, and the rest.
pub(crate) fn split_prefix(word: &str) -> (&str, &str) {
    let mut base = word;
    while let Some(prefix) = PREFIXES.iter().find(|prefix| {
        base.get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    }) {
        base = base.get(prefix.len()..).unwrap_or("");
    }
    let taken = word.len().saturating_sub(base.len());
    (word.get(..taken).unwrap_or(""), base)
}

/// Whether `prefix` makes a LAMBDA parameter: `_xlpm.` and nothing else.
/// Mixed with a function's prefix (`_xlfn._xlpm.WEBSERVICE`) the word is a
/// function, which is the closed side.
pub(crate) fn is_param(prefix: &str) -> bool {
    prefix.eq_ignore_ascii_case("_xlpm.")
}

/// Whether `word` is one cell: `A1`, `$A$1`, up to `XFD1048576`.
pub(crate) fn is_cell(word: &str) -> bool {
    column(word).is_some_and(is_row)
}

/// Whether `word` is a whole column: `A`, `$XFD`.
pub(crate) fn is_column(word: &str) -> bool {
    column(word) == Some("")
}

/// Whether `word` is a whole row: `1`, `$1048576`.
pub(crate) fn is_row(word: &str) -> bool {
    let digits = word.strip_prefix('$').unwrap_or(word);
    (1..=7).contains(&digits.len())
        && digits.bytes().all(|b| b.is_ascii_digit())
        && digits
            .parse::<u32>()
            .is_ok_and(|row| (1..=MAX_ROW).contains(&row))
}

/// What follows the column `word` starts with, if it starts with one.
fn column(word: &str) -> Option<&str> {
    let word = word.strip_prefix('$').unwrap_or(word);
    let letters = word.bytes().take_while(u8::is_ascii_alphabetic).count();
    if !(1..=3).contains(&letters) {
        return None;
    }
    let (name, rest) = word.split_at_checked(letters)?;
    let number = name.bytes().fold(0u32, |n, b| {
        n.saturating_mul(26).saturating_add(
            u32::from(b.to_ascii_uppercase().saturating_sub(b'A')).saturating_add(1),
        )
    });
    (number <= MAX_COLUMN).then_some(rest)
}

/// `quoted` without its outer `quote`s, each doubled `quote` in it one.
pub(crate) fn unquote(quoted: &str, quote: char) -> String {
    let inner = quoted
        .strip_prefix(quote)
        .and_then(|s| s.strip_suffix(quote))
        .unwrap_or(quoted);
    let doubled = format!("{quote}{quote}");
    inner.replace(&doubled, quote.encode_utf8(&mut [0; 4]))
}
