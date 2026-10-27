//! [`MemorySpool`]: a spool in memory, and [`MemorySpools`], which makes
//! them.

use crate::spool::{Spool, SpoolFactory, past_end};
use std::io;

/// A [`Spool`] in a `Vec<u8>`, for a host with no storage of its own and
/// for tests. Growing it reports an allocation that fails as
/// `std::io::ErrorKind::OutOfMemory` instead of aborting.
#[derive(Debug, Default)]
pub struct MemorySpool {
    bytes: Vec<u8>,
}

impl MemorySpool {
    /// An empty spool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Spool for MemorySpool {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.bytes.try_reserve(bytes.len())?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let src = usize::try_from(offset)
            .ok()
            .and_then(|start| self.bytes.get(start..start.checked_add(buf.len())?))
            .ok_or_else(past_end)?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

/// Makes [`MemorySpool`]s: scratch storage that never leaves memory.
#[derive(Debug, Default)]
pub struct MemorySpools;

impl SpoolFactory for MemorySpools {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Ok(Box::new(MemorySpool::new()))
    }
}
