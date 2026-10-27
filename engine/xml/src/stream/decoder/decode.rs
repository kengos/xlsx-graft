//! How a [`Decoder`] decodes a chunk: UTF-8 or UTF-16 to UTF-8, carrying a
//! character split across chunks, and up to the first character XML does
//! not allow.

use super::Decoder;
use crate::chars::{describe, first_invalid};
use crate::encoding::unsupported;
use crate::malformed;
use xlsx_graft_diagnostics::EncodingReason;

impl Decoder {
    /// Decodes UTF-8, carrying a character split at the end of `bytes`.
    pub(super) fn utf8(&mut self, bytes: Vec<u8>) {
        let (valid, invalid) = match std::str::from_utf8(&bytes) {
            Ok(_) => (bytes.len(), false),
            Err(error) => (
                error.valid_up_to(),
                error.error_len().is_some() || self.ended,
            ),
        };
        let (text, rest) = bytes.split_at(valid);
        // `valid_up_to` ends where the bytes stop being UTF-8.
        let text = std::str::from_utf8(text).unwrap_or_default();
        if self.emit(text) && invalid {
            self.failed = Some(unsupported("UTF-8", EncodingReason::InvalidUtf8));
        }
        self.carry = rest.to_vec();
    }

    /// Decodes UTF-16 with code units read by `unit`, carrying an odd byte
    /// or a high surrogate at the end of `bytes`.
    pub(super) fn utf16(&mut self, bytes: Vec<u8>, unit: fn([u8; 2]) -> u16, name: &str) {
        let (pairs, _) = bytes.as_chunks::<2>();
        let mut whole = pairs.len();
        if !self.ended && pairs.last().is_some_and(|&pair| is_high(unit(pair))) {
            whole -= 1;
        }
        let mut text = String::new();
        let mut invalid = false;
        for c in char::decode_utf16(pairs[..whole].iter().copied().map(unit)) {
            match c {
                Ok(c) => text.push(c),
                Err(_) => {
                    invalid = true;
                    break;
                }
            }
        }
        let rest = &bytes[whole * 2..];
        invalid |= self.ended && !rest.is_empty();
        if self.emit(&text) && invalid {
            self.failed = Some(unsupported(name, EncodingReason::InvalidUtf16));
        }
        self.carry = rest.to_vec();
    }

    /// Hands out `text` up to its first character XML does not allow, and
    /// whether it all went out. A byte order mark that starts the text is a
    /// second one, which quick-xml would drop, so it counts as content
    /// before the root.
    fn emit(&mut self, text: &str) -> bool {
        if self.written == 0 && text.starts_with('\u{feff}') {
            let outside = "the XML has content outside the root element";
            self.failed = Some(malformed(0, outside));
            return false;
        }
        let (end, all) = match first_invalid(text) {
            None => (text.len(), true),
            Some((at, c)) => {
                let offset = self.written.saturating_add(as_u64(at));
                self.failed = Some(malformed(offset, &describe(c)));
                (at, false)
            }
        };
        self.out.extend_from_slice(&text.as_bytes()[..end]);
        self.written = self.written.saturating_add(as_u64(end));
        all
    }
}

/// `n` as a `u64`, saturating on a platform where it would not fit.
fn as_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Whether `unit` is a high (leading) surrogate.
fn is_high(unit: u16) -> bool {
    (0xd800..=0xdbff).contains(&unit)
}
