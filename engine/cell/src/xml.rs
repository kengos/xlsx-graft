//! [`xml_text`] and [`xml_attribute`]: text made safe to write in an XML
//! element or an attribute value, piece by piece.

/// Writes `text` as the character data of an XML element: `&`, `<` and `>`
/// as references, and a carriage return as `&#13;`, which an XML parser
/// would otherwise normalize to a line feed.
///
/// `text` goes to `out` in pieces, the text between the characters
/// replaced and each replacement. Use it on an ST_Xstring value after
/// [`escape`](crate::escape) has been applied, never before: it is the
/// XML's escape of the escaped text.
pub fn xml_text<E>(text: &str, out: impl FnMut(&str) -> Result<(), E>) -> Result<(), E> {
    replace(text, out, |c| match c {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '\r' => Some("&#13;"),
        _ => None,
    })
}

/// Writes `text` as the value of an XML attribute in double quotes: what
/// [`xml_text`] replaces, and `"` as `&quot;`, and a tab and a line feed as
/// `&#9;` and `&#10;`, which an XML parser would otherwise normalize to
/// spaces.
///
/// `text` goes to `out` in pieces, as for [`xml_text`], and an ST_Xstring
/// value goes through [`escape`](crate::escape) first.
pub fn xml_attribute<E>(text: &str, out: impl FnMut(&str) -> Result<(), E>) -> Result<(), E> {
    replace(text, out, |c| match c {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '"' => Some("&quot;"),
        '\t' => Some("&#9;"),
        '\n' => Some("&#10;"),
        '\r' => Some("&#13;"),
        _ => None,
    })
}

/// Hands `text` to `out` with each character `special` gives a replacement
/// for replaced.
fn replace<E>(
    text: &str,
    mut out: impl FnMut(&str) -> Result<(), E>,
    special: fn(char) -> Option<&'static str>,
) -> Result<(), E> {
    let mut start = 0;
    for (at, c) in text.char_indices() {
        if let Some(replacement) = special(c) {
            out(text.get(start..at).unwrap_or_default())?;
            out(replacement)?;
            start = at.saturating_add(c.len_utf8());
        }
    }
    out(text.get(start..).unwrap_or_default())
}

#[cfg(test)]
mod tests;
