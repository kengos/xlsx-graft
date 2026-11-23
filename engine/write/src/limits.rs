//! [`WriteLimits`]: how much of a part being written is held in memory,
//! and what the XML the edit reads is held to.

use xlsx_graft_xml::XmlLimits;

/// The bytes of a part deflated that stay in memory before the rest goes to
/// scratch storage: 8 MiB.
const PART_SPILL_BYTES: u64 = 8 * 1024 * 1024;

/// The limits [`PackageEdit::finish`](crate::PackageEdit::finish) holds
/// itself to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteLimits {
    /// How many bytes of a part's deflated content are held in memory
    /// before they are moved to a spool. Only the one part being written is
    /// held, so the memory a write takes does not grow with the number or
    /// the size of the parts copied as they are. The default is 8 MiB.
    pub part_spill_bytes: u64,
    /// The limits on the XML of the parts the edit reads itself, the
    /// content types stream and the Relationships parts it changes. A
    /// [`PartRewriter`](crate::PartRewriter) builds its own reader with the
    /// limits it is given.
    pub xml: XmlLimits,
}

impl Default for WriteLimits {
    fn default() -> Self {
        Self {
            part_spill_bytes: PART_SPILL_BYTES,
            xml: XmlLimits::default(),
        }
    }
}
