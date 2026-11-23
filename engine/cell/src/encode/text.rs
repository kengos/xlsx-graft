//! The text of a cell or of a shared string as a `<t>` element: cut to
//! 32,767 UTF-16 code units, with `xml:space="preserve"` when it starts or
//! ends with a space, tab, line feed or carriage return, and escaped as
//! ST_Xstring and as XML.

use super::Written;
use crate::{CellError, escape, xml_text};
use std::io::Write;

/// The most UTF-16 code units Excel for Mac keeps in a string: it asks to
/// repair a workbook with a longer one and cuts it there
/// (`inline_string_too_long` and `shared_string_too_long` of
/// xlsx-graft-diagnostics). The standard sets no length.
const MAX_UNITS: usize = 32_767;

/// `text` cut before the code unit past [`MAX_UNITS`], at a character
/// boundary: a surrogate pair is never split. [`Written::Truncated`] when
/// something was cut.
pub(super) fn truncate(text: &str) -> (&str, Written) {
    let mut units = 0_usize;
    for (at, c) in text.char_indices() {
        units = units.saturating_add(c.len_utf16());
        if units > MAX_UNITS {
            return (text.get(..at).unwrap_or_default(), Written::Truncated);
        }
    }
    (text, Written::Whole)
}

/// Whether `text` needs `xml:space="preserve"`: `text` starts or ends with a
/// space, tab, line feed or carriage return. Without it, an XML reader may
/// remove that white space.
fn preserve(text: &str) -> bool {
    let white = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r');
    text.starts_with(white) || text.ends_with(white)
}

/// Writes `text`, already cut by [`truncate`], as a `<t>` element.
pub(super) fn write_t(out: &mut dyn Write, text: &str) -> Result<(), CellError> {
    let open: &[u8] = if preserve(text) {
        br#"<t xml:space="preserve">"#
    } else {
        b"<t>"
    };
    out.write_all(open)?;
    escape(text, |piece| {
        xml_text(piece, |xml| out.write_all(xml.as_bytes()))
    })?;
    out.write_all(b"</t>")?;
    Ok(())
}

#[cfg(test)]
mod tests;
