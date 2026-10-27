//! The reason enums: closed sets of causes a code takes as its `reason`
//! argument, written as their snake_case names.

use crate::arg::ArgValue;
use serde::Serialize;
use std::fmt;

/// Why a part's XML could not be read as UTF-8 or UTF-16, the only encodings
/// a package's XML may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EncodingReason {
    /// The byte order mark or the XML declaration names an encoding other
    /// than UTF-8 or UTF-16.
    OtherEncoding,
    /// The XML declaration names UTF-8 or UTF-16, but the bytes are the other
    /// one, or the other byte order.
    DeclarationMismatch,
    /// The bytes are read as UTF-8 and are not valid UTF-8.
    InvalidUtf8,
    /// The bytes are read as UTF-16 and are not valid UTF-16: an odd length
    /// or an unpaired surrogate.
    InvalidUtf16,
    /// The bytes look like UTF-16 but start without a byte order mark, which
    /// XML requires of UTF-16.
    Utf16WithoutBom,
}

impl EncodingReason {
    /// The wire string.
    fn as_str(self) -> &'static str {
        match self {
            Self::OtherEncoding => "other_encoding",
            Self::DeclarationMismatch => "declaration_mismatch",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::InvalidUtf16 => "invalid_utf16",
            Self::Utf16WithoutBom => "utf16_without_bom",
        }
    }
}

impl fmt::Display for EncodingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ArgValue for EncodingReason {}

#[cfg(test)]
mod tests;
