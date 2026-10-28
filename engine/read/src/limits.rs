//! [`ReadLimits`]: how much of a workbook's parts the reader admits, and
//! when it moves them out of memory.

use xlsx_graft_diagnostics::Limit;
use xlsx_graft_xml::XmlLimits;

/// The limits the parts of a workbook are read under.
///
/// The defaults are provisional, to be revisited against large real
/// workbooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimits {
    /// The limits on each XML part read. The shared strings table is read
    /// as a stream, so `max_bytes` does not apply to it.
    pub xml: XmlLimits,
    /// The most strings the shared strings table may hold. Policy name
    /// `maxSharedStrings`.
    pub max_shared_strings: Limit,
    /// The most bytes of text, as UTF-8, the shared strings table may hold
    /// in all. The offsets into the text are 32-bit, so a value past
    /// `u32::MAX` is held to it. Policy name `maxSharedStringsBytes`.
    pub max_shared_strings_bytes: Limit,
    /// The bytes of text and offsets the shared strings table holds in
    /// memory before it moves both to a spool. Once moved, each keeps a
    /// write buffer of half this in memory. Policy name
    /// `sharedStringsSpillBytes`.
    pub shared_strings_spill_bytes: u64,
}

impl ReadLimits {
    /// The default `max_shared_strings`: an offset table of 64 MiB.
    pub const DEFAULT_MAX_SHARED_STRINGS: u64 = 16 * 1024 * 1024;
    /// The default `max_shared_strings_bytes`: as far as 32-bit offsets
    /// reach, which a part within the default `maxEntryBytes` of 2 GiB does
    /// not pass.
    pub const DEFAULT_MAX_SHARED_STRINGS_BYTES: u64 = u32::MAX as u64;
    /// The default `shared_strings_spill_bytes`, 32 MiB.
    pub const DEFAULT_SHARED_STRINGS_SPILL_BYTES: u64 = 32 * 1024 * 1024;
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            xml: XmlLimits::default(),
            max_shared_strings: Limit::default_of(Self::DEFAULT_MAX_SHARED_STRINGS),
            max_shared_strings_bytes: Limit::default_of(Self::DEFAULT_MAX_SHARED_STRINGS_BYTES),
            shared_strings_spill_bytes: Self::DEFAULT_SHARED_STRINGS_SPILL_BYTES,
        }
    }
}
