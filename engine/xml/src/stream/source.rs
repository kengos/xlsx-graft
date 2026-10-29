//! [`Source`]: the `BufRead` quick-xml reads a part through, pumping the
//! reader underneath into a [`Decoder`] a chunk at a time, and holding each
//! event to `max_event_bytes`.
//!
//! quick-xml reads an event by asking for bytes and copying every byte it
//! takes into the caller's buffer, until the event ends; it takes no byte
//! of the next event, only looks at the `<` or `&` that ends a run of text.
//! So the bytes taken since an event began are the event as read so far,
//! and capping what the source hands out caps the buffer. The source hands
//! out at most one byte past the limit, enough for text that ends at it to
//! see the `<` after it, and once more than the limit has been taken it
//! refuses to go on with `xml_event_too_large`. A tag that ends on that
//! byte past the limit is read whole, so the reader checks every event it
//! reads against the same count, [`Source::too_large`]. Whatever reads an
//! event through the source calls [`Source::begin`] first, or the count
//! runs on from the event before.

use super::decoder::Decoder;
use std::io::{self, BufRead, Read};
use xlsx_graft_diagnostics::{Diagnostic, Limit};

/// The most bytes of the inner reader's buffer decoded at once, so a reader
/// that hands out a whole part in one buffer is not copied whole.
const CHUNK: usize = 64 * 1024;

/// `R` decoded to checked UTF-8 for quick-xml, an event at a time.
///
/// An error of `R` itself passes through unchanged, so quick-xml retries an
/// interrupted read and the reader hands any other back to its caller.
#[derive(Debug)]
pub(crate) struct Source<R> {
    inner: R,
    decoder: Decoder,
    /// The most bytes one event may hold.
    limit: Limit,
    /// How many bytes of text have been taken: the offset of the next.
    taken: u64,
    /// Where the event being read starts.
    start: u64,
    /// The event refused for holding more than `limit`.
    refused: Option<Diagnostic>,
}

impl<R: BufRead> Source<R> {
    /// A source decoding `inner`, which has not been read yet, holding each
    /// event to `limit` bytes.
    pub(crate) fn new(inner: R, limit: Limit) -> Self {
        Self {
            inner,
            decoder: Decoder::default(),
            limit,
            taken: 0,
            start: 0,
            refused: None,
        }
    }

    /// The decoder, to ask what it found.
    pub(crate) fn decoder(&self) -> &Decoder {
        &self.decoder
    }

    /// The problem the source stopped at: an event past the limit, or what
    /// the decoder stopped at.
    pub(crate) fn failed(&self) -> Option<&Diagnostic> {
        self.refused.as_ref().or_else(|| self.decoder.failed())
    }

    /// Starts the next event where the last one ended.
    pub(crate) fn begin(&mut self) {
        self.start = self.taken;
    }

    /// `xml_event_too_large` for the event being read, when it holds more
    /// bytes than the limit.
    pub(crate) fn too_large(&self) -> Option<Diagnostic> {
        let size = self.taken.saturating_sub(self.start);
        (size > self.limit.value).then_some(Diagnostic::XmlEventTooLarge {
            offset: self.start,
            size,
            limit: self.limit,
        })
    }

    /// The reader the source decodes.
    pub(crate) fn get_ref(&self) -> &R {
        &self.inner
    }

    /// The reader the source decodes, as far as it was read.
    pub(crate) fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: BufRead> Read for Source<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pending = self.fill_buf()?;
        let taken = pending.len().min(buf.len());
        buf[..taken].copy_from_slice(&pending[..taken]);
        self.consume(taken);
        Ok(taken)
    }
}

impl<R: BufRead> BufRead for Source<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if let Some(diagnostic) = self.too_large() {
            self.refused = Some(diagnostic);
            return Err(io::Error::other("the XML event is too large"));
        }
        while self.decoder.wants_more() {
            let chunk = self.inner.fill_buf()?;
            let taken = chunk.len().min(CHUNK);
            self.decoder.push(&chunk[..taken]);
            self.inner.consume(taken);
        }
        // The event holds at most `limit` bytes here, so this is at least 1.
        let room = self
            .limit
            .value
            .saturating_sub(self.taken.saturating_sub(self.start))
            .saturating_add(1);
        let room = usize::try_from(room).unwrap_or(usize::MAX);
        let pending = self.decoder.pending()?;
        Ok(&pending[..pending.len().min(room)])
    }

    fn consume(&mut self, amount: usize) {
        self.decoder.consume(amount);
        let amount = u64::try_from(amount).unwrap_or(u64::MAX);
        self.taken = self.taken.saturating_add(amount);
    }
}

#[cfg(test)]
mod tests;
