//! [`Echo`]: a bounded, display-safe quote of text the workbook chose.
//!
//! A part name, a relationship target or a content type comes from whoever
//! made the workbook. Quoting it back can go wrong three ways: it can run
//! unbounded; it can carry CONTROL characters (an escape sequence repaints a
//! terminal, a newline forges a second log line); and it can carry
//! BIDIRECTIONAL formatting characters, which reorder how the rest of the line
//! displays without changing a byte of it. `Echo` strips both families, and
//! the characters that break or hide a line the same way, then clips to
//! [`MAX_ECHO`] characters. It is the only string type a
//! [`Diagnostic`](crate::Diagnostic) argument or a
//! [`Location`](crate::Location) can hold.

use serde::{Serialize, Serializer};
use std::fmt;

/// The most characters an [`Echo`] keeps, not counting the `…` that marks a
/// clipped one.
pub const MAX_ECHO: usize = 200;

/// Text from a workbook, stripped of control and bidirectional formatting
/// characters and clipped to [`MAX_ECHO`] characters.
///
/// [`Echo::new`] is the only way to build one, so no unsanitized text can
/// reach a diagnostic. A clipped value ends in `…`, so a reader can tell a
/// short value from a truncated one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Echo(String);

impl Echo {
    /// Strips the unsafe characters from `text`, then clips it.
    ///
    /// Stripping comes first, so a hostile string cannot push an escape
    /// sequence past the cap and out of reach of the filter. The cap counts
    /// characters, not bytes, so a multi-byte script is never cut mid-scalar.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut kept = text.chars().filter(|c| !is_unsafe(*c));
        let mut echo: String = kept.by_ref().take(MAX_ECHO).collect();
        if kept.next().is_some() {
            echo.push('…');
        }
        Self(echo)
    }

    /// The sanitized text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `c` may not appear in an echo: a control character, a
/// bidirectional formatting character, or one that breaks or hides text the
/// same way (the line and paragraph separators, the tag characters).
///
/// Deliberately narrow: the zero-width joiner and non-joiner are format
/// characters too, but they are meaningful inside real text (Indic scripts,
/// Arabic, emoji sequences), so they stay.
fn is_unsafe(c: char) -> bool {
    c.is_control()
        || matches!(c,
            '\u{061c}'                  // arabic letter mark
            | '\u{200e}' | '\u{200f}'   // left-to-right / right-to-left mark
            | '\u{202a}'..='\u{202e}'   // embeddings and overrides
            | '\u{2066}'..='\u{2069}'   // isolates
            | '\u{2028}' | '\u{2029}'   // line / paragraph separator: a forged line break
            | '\u{e0000}'..='\u{e007f}' // tags: invisible text a reader cannot see
        )
}

impl fmt::Display for Echo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Echo {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
