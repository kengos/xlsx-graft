//! [`Source`]: the `BufRead` quick-xml reads a part through, pumping the
//! reader underneath into a [`Decoder`] a chunk at a time.

use super::decoder::Decoder;
use std::io::{self, BufRead, Read};

/// The most bytes of the inner reader's buffer decoded at once, so a reader
/// that hands out a whole part in one buffer is not copied whole.
const CHUNK: usize = 64 * 1024;

/// `R` decoded to checked UTF-8 for quick-xml.
///
/// An error of `R` itself passes through unchanged, so quick-xml retries an
/// interrupted read and the reader hands any other back to its caller.
#[derive(Debug)]
pub(crate) struct Source<R> {
    inner: R,
    decoder: Decoder,
}

impl<R: BufRead> Source<R> {
    /// A source decoding `inner`, which has not been read yet.
    pub(crate) fn new(inner: R) -> Self {
        Self {
            inner,
            decoder: Decoder::default(),
        }
    }

    /// The decoder, to ask what it found.
    pub(crate) fn decoder(&self) -> &Decoder {
        &self.decoder
    }

    /// The reader the source decodes.
    pub(crate) fn get_ref(&self) -> &R {
        &self.inner
    }
}

impl<R: BufRead> Read for Source<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fill_buf()?;
        Ok(self.decoder.take_into(buf))
    }
}

impl<R: BufRead> BufRead for Source<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        while self.decoder.wants_more() {
            let chunk = self.inner.fill_buf()?;
            let taken = chunk.len().min(CHUNK);
            self.decoder.push(&chunk[..taken]);
            self.inner.consume(taken);
        }
        self.decoder.pending()
    }

    fn consume(&mut self, amount: usize) {
        self.decoder.consume(amount);
    }
}

#[cfg(test)]
mod tests;
