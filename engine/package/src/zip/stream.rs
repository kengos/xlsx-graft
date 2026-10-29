//! [`EntryStream`]: an entry's inflated data, read from the archive it is
//! handed at each read, cut off at the size its header declares and checked
//! against its CRC-32 once it ends. Taking its stored bytes from the file,
//! and inflating them, is `input.rs`.

mod input;

use super::{Entry, Method, ZipArchive};
use input::INPUT_BUFFER;
use miniz_oxide::DataFormat;
use miniz_oxide::inflate::stream::InflateState;
use std::io::{self, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, InflateReason};

/// Where a stream is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Reading,
    /// The data ended at the declared size with the declared CRC-32.
    Done,
    /// The entry is refused: a diagnostic was pushed, by this stream or by
    /// another reader of the entry; every later read fails.
    Stopped,
    /// Reading the file failed partway, or the archive has no such entry;
    /// every later read fails, since what was read into the inflater's
    /// input and the caller's buffer is no longer known.
    Failed,
}

/// The inflated data of one readable entry, from
/// [`ZipArchive::stream_entry`], read through [`EntryStream::read`] from
/// the archive it came from.
///
/// It is what an [`EntryReader`](crate::EntryReader) reads, without a
/// borrow of the archive: the archive and the diagnostics are handed in at
/// each read, so a caller can keep the stream beside the archive it reads
/// from, and the archive stays free between reads. Each read seeks to where
/// the stream is in the data before it reads the file, so other reads of
/// the archive in between do not move it.
///
/// It never yields more than the entry declares, so the limits checked on
/// the declared sizes hold for what it yields. When the data is not what the
/// header declares (it inflates to more or less, is not valid deflate, or
/// has another CRC-32), it pushes one diagnostic against the entry, which is
/// not readable from then on, and every read from then on fails with
/// [`io::ErrorKind::InvalidData`]; the bytes it yielded before cannot be
/// trusted. An entry another reader refused meanwhile, before this stream
/// read to its end, fails the same way, without a second diagnostic; a
/// stream that read to its end keeps returning 0. Only the read that returns `Ok(0)` has
/// checked the CRC-32. Compressed bytes after the end of the deflate stream,
/// within the entry's compressed size, are not read.
///
/// A failure of the host fails the read with its own error, and every read
/// after it too.
pub struct EntryStream {
    /// The entry's index in [`ZipArchive::entries`].
    index: usize,
    /// Where the entry's data starts, its compressed size and its CRC-32,
    /// to tell the entry from another archive's at the same index.
    identity: (u64, u64, u32),
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

impl EntryStream {
    /// A stream of `entry`, at `index`, from its start.
    pub(super) fn new(index: usize, entry: &Entry) -> Self {
        let deflated = entry.method == Method::Deflate;
        let remaining = entry.compressed_size;
        let input = if deflated {
            let len = usize::try_from(remaining).unwrap_or(INPUT_BUFFER);
            vec![0; len.min(INPUT_BUFFER)]
        } else {
            Vec::new()
        };
        Self {
            index,
            identity: (entry.data_offset, entry.compressed_size, entry.crc32),
            inflater: deflated.then(|| InflateState::new_boxed(DataFormat::Raw)),
            input,
            start: 0,
            end: 0,
            remaining,
            stream_ended: false,
            produced: 0,
            hasher: crc32fast::Hasher::new(),
            state: State::Reading,
        }
    }

    /// Whether the stream stopped because the entry is refused, so an error
    /// it returned stands for a problem in the entry, which a diagnostic
    /// says, rather than a failure of the host.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.state == State::Stopped
    }

    /// Reads the next inflated bytes into `out` from `archive`, the one the
    /// stream came from, pushing the diagnostic that refuses the entry into
    /// `diagnostics`; 0 at the end of the data, once its size and CRC-32
    /// are checked.
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::InvalidData`] once the entry is refused
    /// ([`is_stopped`](Self::is_stopped)); the host's own error when
    /// seeking or reading `archive` fails, and an error of its own after
    /// that or when `archive` is not the one the stream came from: it has
    /// no entry at the stream's index, or one with other data.
    pub fn read<R: Read + Seek>(
        &mut self,
        archive: &mut ZipArchive<R>,
        out: &mut [u8],
        diagnostics: &mut Diagnostics,
    ) -> io::Result<usize> {
        match self.state {
            State::Done => return Ok(0),
            State::Stopped => return Err(stopped()),
            State::Failed => {
                return Err(io::Error::other("an earlier read of the zip entry failed"));
            }
            State::Reading => {}
        }
        let reader = &mut archive.source.reader;
        let entry = (archive.entries.get_mut(self.index)).filter(|entry| {
            (entry.data_offset, entry.compressed_size, entry.crc32) == self.identity
        });
        let Some(entry) = entry else {
            self.state = State::Failed;
            return Err(io::Error::other("the archive has no such zip entry"));
        };
        if !entry.readable {
            self.state = State::Stopped;
            return Err(stopped());
        }
        if out.is_empty() {
            return Ok(0);
        }
        // One byte past the declared size, to catch data that runs longer.
        let left = entry.size.saturating_sub(self.produced);
        let room = usize::try_from(left)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        let (out, _) = out.split_at_mut(out.len().min(room));
        let written = match self.inflater.take() {
            Some(mut state) => {
                let written = self.inflate(&mut state, reader, entry, out);
                self.inflater = Some(state);
                written
            }
            None => self.copy(reader, entry, out),
        };
        if written.is_err() {
            self.state = State::Failed;
        }
        let written = written?;
        let written = match written {
            Ok(0) => return self.finish(entry, diagnostics),
            Ok(written) => written,
            Err(reason) => {
                let diagnostic = Diagnostic::ZipInflateFailed { reason };
                return self.stop(entry, diagnostic, diagnostics);
            }
        };
        if written as u64 > left {
            let reason = InflateReason::LongerThanDeclared;
            let diagnostic = Diagnostic::ZipInflateFailed { reason };
            return self.stop(entry, diagnostic, diagnostics);
        }
        self.hasher.update(out.get(..written).unwrap_or_default());
        self.produced = self.produced.saturating_add(written as u64);
        Ok(written)
    }

    /// Pushes `diagnostic` against `entry`, which is not readable from then
    /// on, and fails this and every later read.
    #[track_caller]
    fn stop(
        &mut self,
        entry: &mut Entry,
        diagnostic: Diagnostic,
        diagnostics: &mut Diagnostics,
    ) -> io::Result<usize> {
        entry.refuse(diagnostic, diagnostics);
        self.state = State::Stopped;
        Err(stopped())
    }

    /// The end of the data: checks it came to `entry`'s declared size and
    /// CRC-32.
    fn finish(&mut self, entry: &mut Entry, diagnostics: &mut Diagnostics) -> io::Result<usize> {
        if self.produced != entry.size {
            let reason = InflateReason::EndedEarly;
            let diagnostic = Diagnostic::ZipInflateFailed { reason };
            return self.stop(entry, diagnostic, diagnostics);
        }
        let actual = std::mem::take(&mut self.hasher).finalize();
        if actual != entry.crc32 {
            let diagnostic = Diagnostic::ZipCrcMismatch {
                expected: entry.crc32.into(),
                actual: actual.into(),
            };
            return self.stop(entry, diagnostic, diagnostics);
        }
        self.state = State::Done;
        Ok(0)
    }
}

/// The error every read returns once the stream has stopped.
fn stopped() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "the zip entry cannot be read; a diagnostic says why",
    )
}
