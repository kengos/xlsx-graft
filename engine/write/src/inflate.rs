//! [`Inflater`]: the raw deflate stream held in a [`SpillBuffer`], read
//! back as inflated bytes, by [`Inflater::fill`] or as a [`Read`].

use crate::error::WriteError;
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZFlush, MZStatus};
use std::io::{self, Read};
use xlsx_graft_spool::SpillBuffer;

/// How many deflated bytes are read from the buffer at once.
const CHUNK: usize = 16_384;

/// Inflates the deflate stream (RFC 1951) in a [`SpillBuffer`], from its
/// first byte, as the caller asks for bytes.
pub(crate) struct Inflater<'a> {
    buffer: &'a mut SpillBuffer,
    state: Box<InflateState>,
    /// The last chunk read from the buffer.
    input: Vec<u8>,
    /// Where in the buffer the next chunk is read from.
    offset: u64,
    /// `input[start..end]` is the part of the last chunk the decompressor
    /// has not consumed.
    start: usize,
    end: usize,
    /// Whether the decompressor reached the end of the stream.
    done: bool,
}

impl<'a> Inflater<'a> {
    /// An inflater at the start of the stream `buffer` holds.
    pub(crate) fn new(buffer: &'a mut SpillBuffer) -> Self {
        Self {
            buffer,
            state: InflateState::new_boxed(DataFormat::Raw),
            input: vec![0; CHUNK],
            offset: 0,
            start: 0,
            end: 0,
            done: false,
        }
    }

    /// Writes inflated bytes to the start of `out` and returns how many, at
    /// least one unless the stream has ended: 0 after the end of the stream.
    /// `out` must not be empty: the decompressor has no room to write, which
    /// it reports in the middle of a stream as an error
    /// ([`io::ErrorKind::InvalidData`]), and 0 could not be told from the end.
    ///
    /// When the buffer's bytes end (a read returns 0), the decompressor is
    /// still called with empty input. It writes any output still pending,
    /// and returns the end of the stream for a whole stream, or an error when
    /// no progress is possible (a stream cut short).
    ///
    /// # Errors
    ///
    /// [`WriteError::Spool`] when the scratch storage cannot be read.
    /// [`WriteError::Io`] of kind [`io::ErrorKind::InvalidData`] when the
    /// bytes are not a whole deflate stream.
    pub(crate) fn fill(&mut self, out: &mut [u8]) -> Result<usize, WriteError> {
        while !self.done {
            if self.start == self.end {
                self.end = self.buffer.read_at(self.offset, &mut self.input)?;
                self.offset = self.offset.saturating_add(self.end as u64);
                self.start = 0;
            }
            let pending = self.input.get(self.start..self.end).unwrap_or_default();
            let result = inflate(&mut self.state, pending, out, MZFlush::None);
            self.start = self.start.saturating_add(result.bytes_consumed);
            match result.status {
                Ok(MZStatus::StreamEnd) => self.done = true,
                Ok(_) => {}
                // A stream that ends before its last block, or bytes that are not deflate.
                Err(_) => return Err(io::Error::from(io::ErrorKind::InvalidData).into()),
            }
            if result.bytes_written > 0 {
                return Ok(result.bytes_written);
            }
        }
        Ok(0)
    }
}

/// Reads the inflated bytes. A failure of [`fill`](Inflater::fill) is the
/// [`io::Error`] it carries: the one of [`WriteError::Io`] as it is, and for
/// [`WriteError::Spool`] an error that carries the `WriteError`. A caller that
/// passes the `io::Error` on with `?` in a function that returns
/// `Result<_, WriteError>` gets [`WriteError::Spool`] back, as
/// `WriteError::from` returns the carried error. A caller that reads through
/// an `XmlRewriter` does not: the XML error keeps the kind and the message of
/// the `io::Error` only, so the failure comes out as [`WriteError::Io`].
impl Read for Inflater<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // The decompressor is not called with no room for output, which it
        // reports as a stream that cannot go on.
        if buf.is_empty() {
            return Ok(0);
        }
        self.fill(buf).map_err(|error| match error {
            WriteError::Io(error) => error,
            other => io::Error::other(other),
        })
    }
}

#[cfg(test)]
mod tests;
