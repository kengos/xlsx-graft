//! [`EntryReader`]: an entry's inflated data, cut off at the size its header
//! declares and checked against its CRC-32 once it ends.

use super::{Entry, Method};
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};
use std::io::{self, Read, Seek, SeekFrom};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, InflateReason};

/// How many stored bytes a deflated entry reads from the file at a time.
const INPUT_BUFFER: usize = 32 * 1024;

/// Where a reader is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Reading,
    /// The data ended at the declared size with the declared CRC-32.
    Done,
    /// A diagnostic was pushed; every later read fails.
    Stopped,
    /// Reading the file failed partway; every later read fails, since where
    /// the reader is in the data is no longer known.
    Failed,
}

/// The inflated data of one readable entry, from
/// [`ZipArchive::open_entry`](crate::ZipArchive::open_entry).
///
/// It never yields more than the entry declares, so the limits checked on
/// the declared sizes hold for what it yields. When the data is not what the
/// header declares (it inflates to more or less, is not valid deflate, or
/// has another CRC-32), it pushes one diagnostic against the entry, which is
/// not readable from then on, and every read from then on fails with
/// [`io::ErrorKind::InvalidData`]; the bytes it yielded before cannot be
/// trusted. Only the read that returns `Ok(0)` has checked the CRC-32.
/// Compressed bytes after the end of the deflate stream, within the entry's
/// compressed size, are not read.
///
/// A failure of the host fails the read with its own error, and every read
/// after it too.
pub struct EntryReader<'a, R> {
    reader: &'a mut R,
    entry: &'a mut Entry,
    diagnostics: &'a mut Diagnostics,
    /// The inflater, for a deflated entry.
    inflater: Option<Box<InflateState>>,
    /// Stored bytes read from the file and not yet inflated:
    /// `input[start..end]`.
    input: Vec<u8>,
    start: usize,
    end: usize,
    /// Stored bytes not yet read from the file.
    remaining: u64,
    stream_ended: bool,
    /// Inflated bytes yielded so far.
    produced: u64,
    hasher: crc32fast::Hasher,
    state: State,
}

impl<'a, R: Read + Seek> EntryReader<'a, R> {
    /// A reader of `entry`'s data in `reader`, which it seeks to.
    pub(super) fn new(
        reader: &'a mut R,
        entry: &'a mut Entry,
        diagnostics: &'a mut Diagnostics,
    ) -> io::Result<Self> {
        reader.seek(SeekFrom::Start(entry.data_offset))?;
        let deflated = entry.method == Method::Deflate;
        let remaining = entry.compressed_size;
        let input = if deflated {
            let len = usize::try_from(remaining).unwrap_or(INPUT_BUFFER);
            vec![0; len.min(INPUT_BUFFER)]
        } else {
            Vec::new()
        };
        Ok(Self {
            reader,
            entry,
            diagnostics,
            inflater: deflated.then(|| InflateState::new_boxed(DataFormat::Raw)),
            input,
            start: 0,
            end: 0,
            remaining,
            stream_ended: false,
            produced: 0,
            hasher: crc32fast::Hasher::new(),
            state: State::Reading,
        })
    }
}

impl<R> EntryReader<'_, R> {
    /// Whether the reader stopped with a diagnostic, so an error it returned
    /// stands for a problem in the entry rather than a failure of the host.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.state == State::Stopped
    }

    /// Pushes `diagnostic` against the entry, which is not readable from
    /// then on, and fails this and every later read.
    #[track_caller]
    fn stop(&mut self, diagnostic: Diagnostic) -> io::Result<usize> {
        self.entry.refuse(diagnostic, self.diagnostics);
        self.state = State::Stopped;
        Err(stopped())
    }

    /// The end of the data: checks it came to the declared size and CRC-32.
    fn finish(&mut self) -> io::Result<usize> {
        if self.produced != self.entry.size {
            let reason = InflateReason::EndedEarly;
            return self.stop(Diagnostic::ZipInflateFailed { reason });
        }
        let actual = std::mem::take(&mut self.hasher).finalize();
        if actual != self.entry.crc32 {
            return self.stop(Diagnostic::ZipCrcMismatch {
                expected: self.entry.crc32.into(),
                actual: actual.into(),
            });
        }
        self.state = State::Done;
        Ok(0)
    }
}

impl<R: Read> EntryReader<'_, R> {
    /// Copies stored data into `out`; 0 at its end.
    fn copy(&mut self, out: &mut [u8]) -> io::Result<Result<usize, InflateReason>> {
        let len = out
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        let (out, _) = out.split_at_mut(len);
        self.reader.read_exact(out)?;
        self.remaining -= len as u64;
        Ok(Ok(len))
    }

    /// Inflates deflated data into `out` with `state`; 0 at the end of the
    /// stream.
    fn inflate(
        &mut self,
        state: &mut InflateState,
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
                let (fill, _) = self.input.split_at_mut(len);
                self.reader.read_exact(fill)?;
                (self.start, self.end) = (0, len);
                self.remaining -= len as u64;
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

impl<R: Read> Read for EntryReader<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        match self.state {
            State::Done => return Ok(0),
            State::Stopped => return Err(stopped()),
            State::Failed => {
                return Err(io::Error::other("an earlier read of the zip entry failed"));
            }
            State::Reading if out.is_empty() => return Ok(0),
            State::Reading => {}
        }
        // One byte past the declared size, to catch data that runs longer.
        let left = self.entry.size.saturating_sub(self.produced);
        let room = usize::try_from(left)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        let (out, _) = out.split_at_mut(out.len().min(room));
        let written = match self.inflater.take() {
            Some(mut state) => {
                let written = self.inflate(&mut state, out);
                self.inflater = Some(state);
                written
            }
            None => self.copy(out),
        };
        if written.is_err() {
            self.state = State::Failed;
        }
        let written = written?;
        let written = match written {
            Ok(0) => return self.finish(),
            Ok(written) => written,
            Err(reason) => return self.stop(Diagnostic::ZipInflateFailed { reason }),
        };
        if written as u64 > left {
            let reason = InflateReason::LongerThanDeclared;
            return self.stop(Diagnostic::ZipInflateFailed { reason });
        }
        self.hasher.update(out.get(..written).unwrap_or_default());
        self.produced = self.produced.saturating_add(written as u64);
        Ok(written)
    }
}

/// The error every read returns once the reader has stopped.
fn stopped() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "the zip entry cannot be read; a diagnostic says why",
    )
}
