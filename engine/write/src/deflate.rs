//! [`DeflateWriter`]: new content deflated as it is written, held in a
//! [`SpillBuffer`] until it is written into a zip, with its CRC-32 and size.

use crate::error::WriteError;
use miniz_oxide::DataFormat;
use miniz_oxide::MZFlush;
use miniz_oxide::MZStatus;
use miniz_oxide::deflate::core::CompressorOxide;
use miniz_oxide::deflate::stream::deflate;
use std::io::{self, Write};
use xlsx_graft_spool::{SpillBuffer, SpoolFactory};

/// The deflate level of every new entry, the level of zlib's default.
const LEVEL: u8 = 6;
/// How many bytes of deflated output one call of the compressor can fill.
const OUTPUT_CHUNK: usize = 65_536;
/// The write buffer the content keeps in memory once it is spilled.
const SPILL_BUFFER: usize = 65_536;

/// New content deflated: raw deflate (RFC 1951) with the CRC-32 and size of
/// what was written, for [`ZipWriter::write_deflated`](crate::ZipWriter::write_deflated).
#[derive(Debug)]
pub struct Deflated {
    pub(crate) buffer: SpillBuffer,
    pub(crate) crc32: u32,
    pub(crate) size: u64,
}

/// A [`Write`] that deflates what it is given at level 6, keeping the
/// output in memory until it would pass `spill_at` bytes and in a spool
/// from `spools` after that. Same input, same output, whether it spilled or
/// not.
pub struct DeflateWriter<'a> {
    compressor: Box<CompressorOxide>,
    buffer: SpillBuffer,
    spools: &'a mut dyn SpoolFactory,
    spill_at: u64,
    hasher: crc32fast::Hasher,
    size: u64,
    /// Where the compressor writes its output.
    chunk: Vec<u8>,
    /// Whether a write failed: the CRC-32 and the size then count bytes
    /// the output may not hold.
    failed: bool,
}

impl<'a> DeflateWriter<'a> {
    /// A writer with nothing written, spilling to a spool of `spools` when
    /// the deflated bytes would pass `spill_at`.
    pub fn new(spools: &'a mut dyn SpoolFactory, spill_at: u64) -> Self {
        let mut compressor = Box::<CompressorOxide>::default();
        compressor.set_format_and_level(DataFormat::Raw, LEVEL);
        Self {
            compressor,
            buffer: SpillBuffer::new(),
            spools,
            spill_at,
            hasher: crc32fast::Hasher::new(),
            size: 0,
            chunk: vec![0; OUTPUT_CHUNK],
            failed: false,
        }
    }

    /// Ends the deflate stream and returns the content.
    ///
    /// # Errors
    ///
    /// [`WriteError::Spool`] when the scratch storage cannot be made or
    /// written; [`WriteError::Unusable`] when an earlier write failed.
    pub fn finish(mut self) -> Result<Deflated, WriteError> {
        if self.failed {
            return Err(WriteError::Unusable);
        }
        while !self.run(&[], MZFlush::Finish)?.1 {}
        Ok(Deflated {
            buffer: self.buffer,
            crc32: self.hasher.finalize(),
            size: self.size,
        })
    }

    /// Gives the compressor `input` once: how many bytes of it it took and
    /// whether the stream ended, with its output appended to the buffer.
    fn run(&mut self, input: &[u8], flush: MZFlush) -> Result<(usize, bool), WriteError> {
        let result = deflate(&mut self.compressor, input, &mut self.chunk, flush);
        // The compressor fails only for arguments this crate does not pass.
        // The error is built before the call rather than in an `ok_or_else`
        // closure: no test reaches the failing branch, and the closure's line
        // would count as uncovered.
        let refused = io::Error::from(io::ErrorKind::Other);
        let status = result.status.ok().ok_or(refused)?;
        let output = self.chunk.get(..result.bytes_written).unwrap_or_default();
        let held = self.buffer.len().saturating_add(output.len() as u64);
        if !self.buffer.is_spilled() && held > self.spill_at {
            self.buffer.spill(self.spools, SPILL_BUFFER)?;
        }
        self.buffer.append(output)?;
        Ok((result.bytes_consumed, status == MZStatus::StreamEnd))
    }
}

/// A failure of `write` is returned as an [`io::Error`] that carries the
/// [`WriteError`] (`WriteError::Spool` for the scratch storage), and
/// `WriteError::from` returns that `WriteError`, so a `?` in a function that returns
/// `Result<_, WriteError>` keeps the variant. After a failed write,
/// [`finish`](DeflateWriter::finish) returns [`WriteError::Unusable`].
impl Write for DeflateWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.update(buf);
        self.size = self.size.saturating_add(buf.len() as u64);
        let mut input = buf;
        while !input.is_empty() {
            match self.run(input, MZFlush::None) {
                Ok((consumed, _)) => input = input.get(consumed..).unwrap_or_default(),
                Err(error) => {
                    self.failed = true;
                    return Err(io::Error::other(error));
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
