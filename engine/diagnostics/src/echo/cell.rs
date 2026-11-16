//! [`CellEcho`]: the text of a cell quoted for display. A cell's text may
//! be long and may hold line ends and tabs a table or a JSON string
//! carries, so it is kept longer than an [`Echo`](super::Echo) and keeps
//! those three characters; it strips everything an `Echo` strips otherwise.

use super::is_unsafe;

/// The most characters a [`CellEcho`] keeps.
pub const MAX_CELL_ECHO: usize = 1_000;

/// The text of a cell, quoted for display: the tab, the line feed and the
/// carriage return kept; every other control character, and every
/// character [`Echo`](super::Echo) strips as bidirectional formatting or as
/// one that breaks or hides a line (including U+2028 and U+2029), removed;
/// then cut at [`MAX_CELL_ECHO`] characters, [`truncated`](Self::truncated)
/// saying whether it was cut. No ellipsis is added to the text.
///
/// The characters removed are not counted: the cap counts the characters
/// kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellEcho {
    text: String,
    truncated: bool,
}

impl CellEcho {
    /// Strips the characters `text` may not carry, then cuts it at
    /// [`MAX_CELL_ECHO`] characters. Counting characters, not bytes, never
    /// cuts a multi-byte character.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut kept = text
            .chars()
            .filter(|c| matches!(c, '\t' | '\n' | '\r') || !is_unsafe(*c));
        let text: String = kept.by_ref().take(MAX_CELL_ECHO).collect();
        Self {
            text,
            truncated: kept.next().is_some(),
        }
    }

    /// The stripped text, cut if it was long.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Whether the text was cut at [`MAX_CELL_ECHO`] characters.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

#[cfg(test)]
mod tests;
