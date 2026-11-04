//! [`check`]: every string of a workbook's shared strings table held to
//! the 32,767 characters, counted in UTF-16 code units, Excel keeps in a
//! cell (`shared_string_too_long`).
//!
//! A string's text is what [`SharedStrings`] reads it as: its `t`, or the
//! `t` of each of its runs together (ECMA-376-1:2016 §18.4.8), escapes
//! undone. Its phonetic runs (`rPh`, §18.4.6) are a hint about how the
//! text is read, not part of it, and Excel does not count them with it:
//! Excel for Mac opens a string of 32,767 characters with ten more in a
//! phonetic run without a repair. An escape of an unpaired surrogate
//! (`_xD83D_`) is kept as written, as the escape of its `_` (`_x005F_`)
//! would leave it, so it counts as seven where Excel counts one: Excel for
//! Mac opens 4,682 of them, 32,774 characters as written, without a
//! repair, and this reports them.
//!
//! The strings are looked up one at a time, so the memory used is the
//! table's, spilled as it is, and room for twice the longest string.

use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_read::SharedStrings;
use xlsx_graft_spool::SpoolError;

/// The most UTF-16 code units of a string Excel keeps in a cell.
const MAX_UNITS: usize = 32_767;

/// Pushes `shared_string_too_long`, at the part `part`, for every string of
/// `strings` longer than Excel keeps. A table that is absent or unknown
/// holds no string to check: why it is unknown was reported as it was
/// read.
///
/// # Errors
///
/// When the spool the table spilled to fails, or memory for a string
/// cannot be allocated.
pub(crate) fn check(
    strings: &mut SharedStrings,
    part: &str,
    diagnostics: &mut Diagnostics,
) -> Result<(), SpoolError> {
    let SharedStrings::Table(table) = strings else {
        return Ok(());
    };
    let mut text = String::new();
    for index in 0..table.len() {
        // A string the table holds is found; were the spool to come back
        // short, the string is left empty, and is short enough.
        let _found = table.get(index, &mut text)?;
        // A string is never more UTF-16 code units than UTF-8 bytes.
        if text.len() <= MAX_UNITS {
            continue;
        }
        let length = text.encode_utf16().count();
        if length > MAX_UNITS {
            let length = u64::try_from(length).unwrap_or(u64::MAX);
            let location = Location::default().with_part(part);
            diagnostics.push(Diagnostic::SharedStringTooLong { index, length }, location);
        }
    }
    Ok(())
}
