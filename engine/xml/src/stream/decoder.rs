//! [`Decoder`]: a part's bytes, pushed a chunk at a time, as checked UTF-8
//! text. It sniffs the byte order mark from the first bytes, decodes UTF-8
//! or UTF-16, and stops before the first byte that does not decode, the
//! first character XML does not allow, or a second byte order mark. It
//! knows no reader, so every reader of a part decodes it the same way.

use crate::encoding::{Found, sniff};
use std::io;
use xlsx_graft_diagnostics::Diagnostic;

mod decode;

/// The bytes [`sniff`] needs to tell every encoding apart.
const SNIFFED: usize = 4;

/// Decodes a part pushed a chunk at a time, handing out the text up to the
/// first problem and then an `Err`, with [`Decoder::failed`] saying why.
#[derive(Debug, Default)]
pub(crate) struct Decoder {
    /// The encoding, once the first bytes are sniffed.
    found: Option<Found>,
    /// Bytes pushed but not decoded yet: the first bytes until there are
    /// enough to sniff, then the start of a character split across chunks.
    carry: Vec<u8>,
    /// Decoded text, handed out from `pos`.
    out: Vec<u8>,
    pos: usize,
    /// How many bytes of text have been decoded: the offset of the next.
    written: u64,
    /// Whether the part has ended.
    ended: bool,
    /// The problem decoding stopped at.
    failed: Option<Diagnostic>,
}

impl Decoder {
    /// The encoding the byte order mark showed; UTF-8 before the first bytes
    /// are pushed.
    pub(crate) fn found(&self) -> Found {
        self.found.unwrap_or(Found::Utf8)
    }

    /// The problem decoding stopped at.
    pub(crate) fn failed(&self) -> Option<&Diagnostic> {
        self.failed.as_ref()
    }

    /// Whether every byte of text has been handed out and decoding can go
    /// on: the next chunk is wanted.
    pub(crate) fn wants_more(&self) -> bool {
        self.pos == self.out.len() && self.failed.is_none() && !self.ended
    }

    /// Decodes `chunk`, the part's next bytes, once the text before it is
    /// all handed out; an empty chunk is the part's end.
    pub(crate) fn push(&mut self, chunk: &[u8]) {
        let mut bytes = std::mem::take(&mut self.carry);
        bytes.extend_from_slice(chunk);
        self.ended = chunk.is_empty();
        self.out.clear();
        self.pos = 0;
        self.decode(bytes);
    }

    /// The text not yet handed out, or an `Err` once it is all out and
    /// decoding has stopped at a problem.
    pub(crate) fn pending(&self) -> io::Result<&[u8]> {
        if self.pos == self.out.len() && self.failed.is_some() {
            return Err(io::Error::other("the XML stopped at a problem"));
        }
        Ok(&self.out[self.pos..])
    }

    /// Marks `amount` bytes of the pending text handed out.
    pub(crate) fn consume(&mut self, amount: usize) {
        self.pos = self.pos.saturating_add(amount).min(self.out.len());
    }

    /// Decodes `bytes`, the carry and the chunk after it.
    fn decode(&mut self, bytes: Vec<u8>) {
        let Some(found) = self.found else {
            return self.sniff(bytes);
        };
        match found {
            Found::Utf8 => self.utf8(bytes),
            Found::Utf16Le => self.utf16(bytes, u16::from_le_bytes, "UTF-16LE"),
            Found::Utf16Be => self.utf16(bytes, u16::from_be_bytes, "UTF-16BE"),
        }
    }

    /// Settles the encoding from the first bytes, once there are enough of
    /// them or the part has ended, and decodes what follows the byte order
    /// mark.
    fn sniff(&mut self, bytes: Vec<u8>) {
        if bytes.len() < SNIFFED && !self.ended {
            self.carry = bytes;
            return;
        }
        match sniff(&bytes) {
            Err(diagnostic) => self.failed = Some(diagnostic),
            Ok((found, body)) => {
                let body = body.to_vec();
                self.found = Some(found);
                self.decode(body);
            }
        }
    }
}

#[cfg(test)]
mod tests;
