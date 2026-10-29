//! Where an [`EntryStream`] takes its bytes from: the entry's stored bytes
//! read from the file, from where the stream is in them, copied as they are
//! or inflated.

use super::EntryStream;
use crate::zip::Entry;
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{MZError, MZFlush, MZStatus};
use std::io::{self, Read, Seek, SeekFrom};
use xlsx_graft_diagnostics::InflateReason;

/// How many stored bytes a deflated entry reads from the file at a time.
pub(super) const INPUT_BUFFER: usize = 32 * 1024;

impl EntryStream {
    /// Fills `buf` with the stored bytes of `entry` from where the stream
    /// is in them, seeking `reader` there first unless it is there already:
    /// a seek may drop what a buffered reader holds.
    fn fill<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        entry: &Entry,
        buf: &mut [u8],
    ) -> io::Result<()> {
        let read = entry.compressed_size.saturating_sub(self.remaining);
        let at = entry.data_offset.saturating_add(read);
        if reader.stream_position()? != at {
            reader.seek(SeekFrom::Start(at))?;
        }
        reader.read_exact(buf)?;
        self.remaining -= buf.len() as u64;
        Ok(())
    }

    /// Copies stored data into `out`; 0 at its end.
    pub(super) fn copy<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        entry: &Entry,
        out: &mut [u8],
    ) -> io::Result<Result<usize, InflateReason>> {
        let len = out
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        let (out, _) = out.split_at_mut(len);
        if len > 0 {
            self.fill(reader, entry, out)?;
        }
        Ok(Ok(len))
    }

    /// Inflates deflated data into `out` with `state`; 0 at the end of the
    /// stream.
    pub(super) fn inflate<R: Read + Seek>(
        &mut self,
        state: &mut InflateState,
        reader: &mut R,
        entry: &Entry,
        out: &mut [u8],
    ) -> io::Result<Result<usize, InflateReason>> {
        loop {
            if self.stream_ended {
                return Ok(Ok(0));
            }
            if self.start == self.end && self.remaining > 0 {
                let len = self
                    .input
                    .len()
                    .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
                let mut input = std::mem::take(&mut self.input);
                let (buf, _) = input.split_at_mut(len);
                let filled = self.fill(reader, entry, buf);
                self.input = input;
                filled?;
                (self.start, self.end) = (0, len);
            }
            let pending = self.input.get(self.start..self.end).unwrap_or_default();
            let result = inflate(state, pending, out, MZFlush::None);
            self.start += result.bytes_consumed;
            let progress = result.bytes_consumed > 0 || result.bytes_written > 0;
            match result.status {
                Ok(MZStatus::StreamEnd) => self.stream_ended = true,
                Ok(_) if progress => {}
                Ok(_) | Err(MZError::Buf) => return Ok(Err(InflateReason::EndedEarly)),
                Err(_) => return Ok(Err(InflateReason::CorruptData)),
            }
            if result.bytes_written > 0 {
                return Ok(Ok(result.bytes_written));
            }
        }
    }
}
