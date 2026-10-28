//! [`StringTable`]: the strings of a shared strings table as one run of
//! UTF-8 bytes and a table of where each ends, spilled together once they
//! pass a threshold; and [`Builder`], which fills one under the limits.

use super::{Halt, Lookup};
use crate::ReadLimits;
use std::io;
use xlsx_graft_diagnostics::{Diagnostic, Limit};
use xlsx_graft_spool::{Fill, SpillBuffer, SpoolError, SpoolFactory};

/// The bytes one offset takes in the table of ends.
const END: u64 = 4;

/// The strings of a shared strings table that was read whole.
#[derive(Debug)]
pub struct StringTable {
    /// Every string's text, one after another.
    text: SpillBuffer,
    /// Where each string's text ends in `text`, a little-endian `u32` each.
    ends: SpillBuffer,
    /// How many strings there are.
    count: u64,
    /// The bytes of the string looked up last.
    scratch: Vec<u8>,
}

impl StringTable {
    /// How many strings the table holds.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.count
    }

    /// Whether the table holds no string.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Whether the strings were moved to a spool.
    #[must_use]
    pub fn is_spilled(&self) -> bool {
        self.text.is_spilled()
    }

    /// The bytes of memory the text and the table of ends hold allocated,
    /// as [`SpillBuffer::memory_bytes`] counts them. Once spilled, no more
    /// than `sharedStringsSpillBytes`. Before, up to twice it, since a
    /// buffer in memory grows by doubling. Not counted: the room kept for
    /// the longest string looked up so far.
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        (self.text.memory_bytes()).saturating_add(self.ends.memory_bytes())
    }

    /// Puts the string at `index` in `out`, in place of what it held.
    /// `out` is left empty unless it is found.
    ///
    /// # Errors
    ///
    /// When the spool fails, or memory for the string cannot be allocated.
    pub fn get(&mut self, index: u64, out: &mut String) -> Result<Lookup, SpoolError> {
        out.clear();
        let found = match self.bounds(index)? {
            Some((start, len)) => {
                self.scratch.clear();
                self.scratch.try_reserve(len).map_err(io::Error::from)?;
                self.scratch.resize(len, 0);
                self.text.read_exact_at(start, &mut self.scratch)? == Fill::Filled
            }
            None => false,
        };
        if found {
            out.try_reserve(self.scratch.len())
                .map_err(io::Error::from)?;
            out.push_str(&String::from_utf8_lossy(&self.scratch));
        }
        Ok(if found {
            Lookup::Found
        } else {
            Lookup::OutOfRange
        })
    }

    /// Where the string at `index` starts in the text and how long it is;
    /// `None` when there is no such string.
    fn bounds(&mut self, index: u64) -> Result<Option<(u64, usize)>, SpoolError> {
        // The end of the string before, then this one's: the first starts
        // at 0.
        let mut ends = [0; 2 * END as usize];
        let filled = match index.checked_sub(1) {
            Some(before) => (self.ends).read_exact_at(before.saturating_mul(END), &mut ends)?,
            None => (self.ends).read_exact_at(0, &mut ends[END as usize..])?,
        };
        let [a, b, c, d, e, f, g, h] = ends;
        let (start, end) = (
            u32::from_le_bytes([a, b, c, d]),
            u32::from_le_bytes([e, f, g, h]),
        );
        let len = usize::try_from(end.saturating_sub(start)).unwrap_or(usize::MAX);
        Ok((filled == Fill::Filled).then_some((u64::from(start), len)))
    }
}

/// Fills a [`StringTable`], one string at a time, under the limits.
pub(super) struct Builder<'a> {
    table: StringTable,
    limits: &'a ReadLimits,
    spools: &'a mut dyn SpoolFactory,
}

impl<'a> Builder<'a> {
    /// An empty table, filled under `limits`, spilling to a spool from
    /// `spools`.
    pub(super) fn new(limits: &'a ReadLimits, spools: &'a mut dyn SpoolFactory) -> Self {
        let table = StringTable {
            text: SpillBuffer::new(),
            ends: SpillBuffer::new(),
            count: 0,
            scratch: Vec::new(),
        };
        Self {
            table,
            limits,
            spools,
        }
    }

    /// Starts a string, unless the table holds as many as it may.
    pub(super) fn begin(&self) -> Result<(), Halt> {
        let limit = self.limits.max_shared_strings;
        if self.table.count >= limit.value {
            let count = self.table.count.saturating_add(1);
            return Err(Halt::Refused(Diagnostic::SharedStringsTooMany {
                count,
                limit,
            }));
        }
        Ok(())
    }

    /// Adds `text` to the string started last, unless the text would pass
    /// its limit.
    pub(super) fn text(&mut self, text: &str) -> Result<(), Halt> {
        let limit = self.limits.max_shared_strings_bytes;
        let limit = Limit {
            value: limit.value.min(u64::from(u32::MAX)),
            ..limit
        };
        let size = self.table.text.len().saturating_add(text.len() as u64);
        if size > limit.value {
            return Err(Halt::Refused(Diagnostic::SharedStringsTooLarge {
                size,
                limit,
            }));
        }
        Ok(self.append(false, text.as_bytes())?)
    }

    /// Ends the string started last.
    pub(super) fn end(&mut self) -> Result<(), Halt> {
        // `text` holds the text to `u32::MAX` bytes.
        let end = u32::try_from(self.table.text.len()).unwrap_or(u32::MAX);
        self.append(true, &end.to_le_bytes())?;
        self.table.count = self.table.count.saturating_add(1);
        Ok(())
    }

    /// The table, filled.
    pub(super) fn finish(self) -> StringTable {
        self.table
    }

    /// Appends `bytes` to the table of ends when `ends`, else to the text,
    /// first moving both to a spool when together they would pass the
    /// threshold. Judged on what was appended, not on what memory holds,
    /// which grows by doubling; once spilled, each keeps a write buffer of
    /// half the threshold.
    fn append(&mut self, ends: bool, bytes: &[u8]) -> Result<(), SpoolError> {
        let threshold = self.limits.shared_strings_spill_bytes;
        let held = (self.table.text.len())
            .saturating_add(self.table.ends.len())
            .saturating_add(bytes.len() as u64);
        if held > threshold {
            let buffer = usize::try_from(threshold / 2).unwrap_or(usize::MAX);
            self.table.text.spill(self.spools, buffer)?;
            self.table.ends.spill(self.spools, buffer)?;
        }
        let buffer = if ends {
            &mut self.table.ends
        } else {
            &mut self.table.text
        };
        buffer.append(bytes)
    }
}
