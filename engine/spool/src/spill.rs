//! [`SpillBuffer`]: bytes held in memory until the caller spills them to a
//! [`Spool`].

use crate::SpoolError;
use crate::spool::{Spool, SpoolFactory};
use std::{fmt, io};

/// An append-only run of bytes, in memory until [`spill`](Self::spill)
/// moves it to a spool, and in the spool from then on, behind a write
/// buffer. The crate root says why the caller decides when.
///
/// Every byte appended can be read back with
/// [`read_exact_at`](Self::read_exact_at), before or after the spill, and
/// whether it reached the spool yet or not.
pub struct SpillBuffer {
    /// The bytes not in the spool: all of them before the spill, the
    /// write buffer after it.
    memory: Vec<u8>,
    /// Once spilled, the spool and the size of the write buffer.
    spool: Option<(Box<dyn Spool>, usize)>,
    /// How many of the bytes are in the spool.
    spooled: u64,
}

/// Whether [`SpillBuffer::read_exact_at`] filled the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Fill {
    /// The buffer holds the bytes asked for.
    Filled,
    /// The range runs past the bytes appended, and the buffer is as it was.
    OutOfRange,
}

impl SpillBuffer {
    /// An empty buffer, in memory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: Vec::new(),
            spool: None,
            spooled: 0,
        }
    }

    /// How many bytes were appended in all.
    #[must_use]
    pub fn len(&self) -> u64 {
        // A Vec never holds 2^64 bytes, nor a spool: no saturation.
        self.spooled.saturating_add(self.memory.len() as u64)
    }

    /// Whether nothing was appended.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether [`spill`](Self::spill) moved the bytes to a spool.
    #[must_use]
    pub fn is_spilled(&self) -> bool {
        self.spool.is_some()
    }

    /// The bytes of memory the buffer holds allocated, not counting a
    /// spool's own (a [`MemorySpool`](crate::MemorySpool) is all memory,
    /// and while spilling to one both are held). Before the spill that is
    /// the bytes appended and the room grown for more, which doubles as it
    /// grows: up to twice [`len`](Self::len), so a caller holding to a
    /// limit checks `len()` and the bytes it is about to append, before it
    /// appends them. After the spill it is the write buffer: nothing until
    /// the first append, then the size [`spill`](Self::spill) was given,
    /// allocated once and never grown.
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.memory.capacity()
    }

    /// Writes `bytes` after the bytes appended so far: into memory, or once
    /// spilled into the write buffer, which goes to the spool when the
    /// bytes would not fit in it. Bytes larger than the write buffer go to
    /// the spool directly.
    ///
    /// # Errors
    ///
    /// When memory cannot be allocated, or the spool fails. Nothing of
    /// `bytes` is counted then, though the write buffer may have reached
    /// the spool.
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), SpoolError> {
        let Some((spool, buffer)) = &mut self.spool else {
            self.memory
                .try_reserve(bytes.len())
                .map_err(io::Error::from)?;
            self.memory.extend_from_slice(bytes);
            return Ok(());
        };
        let full = self.memory.len().saturating_add(bytes.len()) > *buffer;
        if full && !self.memory.is_empty() {
            spool.append(&self.memory)?;
            self.spooled = self.spooled.saturating_add(self.memory.len() as u64);
            self.memory.clear();
        }
        if bytes.len() > *buffer {
            spool.append(bytes)?;
            self.spooled = self.spooled.saturating_add(bytes.len() as u64);
        } else {
            // Once, the first time: the whole write buffer, so it never
            // grows by doubling past its size.
            self.memory
                .try_reserve_exact(buffer.saturating_sub(self.memory.len()))
                .map_err(io::Error::from)?;
            self.memory.extend_from_slice(bytes);
        }
        Ok(())
    }

    /// Moves the bytes appended to a spool `spools` makes, frees the memory
    /// they were in, and from then on holds no more than `buffer` bytes in
    /// memory. A buffer spilled already stays as it is.
    ///
    /// # Errors
    ///
    /// When the spool cannot be made or written. The bytes are still in
    /// memory then, and the buffer is as it was.
    pub fn spill(
        &mut self,
        spools: &mut dyn SpoolFactory,
        buffer: usize,
    ) -> Result<(), SpoolError> {
        if self.spool.is_some() {
            return Ok(());
        }
        let mut spool = spools.create()?;
        spool.append(&self.memory)?;
        self.spooled = self.memory.len() as u64;
        // Freed before the write buffer is allocated, at the first append:
        // the two are never held at once.
        self.memory = Vec::new();
        self.spool = Some((spool, buffer));
        Ok(())
    }

    /// Fills `buf` with the bytes appended from `offset` on: from the
    /// spool, the memory, or both.
    ///
    /// # Errors
    ///
    /// When the spool fails. `buf` may hold some of the bytes then.
    pub fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<Fill, SpoolError> {
        let stop = offset.checked_add(buf.len() as u64);
        if stop.is_none_or(|stop| stop > self.len()) {
            return Ok(Fill::OutOfRange);
        }
        // The part of the range in the spool, then the part in memory.
        let in_spool = usize::try_from(self.spooled.saturating_sub(offset))
            .map_or(buf.len(), |n| n.min(buf.len()));
        let (head, tail) = buf.split_at_mut(in_spool);
        if let (false, Some((spool, _))) = (head.is_empty(), &mut self.spool) {
            spool.read_exact_at(offset, head)?;
        }
        let start = offset
            .saturating_add(in_spool as u64)
            .saturating_sub(self.spooled);
        let src = usize::try_from(start)
            .ok()
            .and_then(|start| self.memory.get(start..start.checked_add(tail.len())?));
        if let Some(src) = src {
            tail.copy_from_slice(src);
        }
        Ok(Fill::Filled)
    }
}

impl fmt::Debug for SpillBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A host's spool need not be Debug: say what the buffer holds.
        f.debug_struct("SpillBuffer")
            .field("len", &self.len())
            .field("spooled", &self.spooled)
            .field("memory_bytes", &self.memory_bytes())
            .field("buffer", &self.spool.as_ref().map(|(_, buffer)| buffer))
            .finish_non_exhaustive()
    }
}

impl Default for SpillBuffer {
    fn default() -> Self {
        Self::new()
    }
}
