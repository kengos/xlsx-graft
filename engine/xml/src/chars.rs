//! The characters XML allows (XML 1.0 §2.2, the `Char` production).

/// Whether XML 1.0 allows `c` in a document: tab, line feed, carriage
/// return and everything from U+0020 except U+FFFE and U+FFFF (a `char` is
/// never a surrogate).
pub(crate) fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{fffd}' | '\u{10000}'..)
}

/// The byte offset and the character of the first one in `text` XML does
/// not allow.
pub(crate) fn first_invalid(text: &str) -> Option<(usize, char)> {
    text.char_indices().find(|&(_, c)| !is_xml_char(c))
}

/// The `xml_malformed` detail for the character `c`.
pub(crate) fn describe(c: char) -> String {
    format!("a character XML does not allow (U+{:04X})", u32::from(c))
}
