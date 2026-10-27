//! The checks on the entries that need no more reading: what an entry's own
//! central directory header says, and what it declares it inflates to.

use super::{Entry, Method};
use crate::ZipLimits;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};

/// Bit 0 of the general purpose flags: the entry is encrypted.
const ENCRYPTED: u16 = 0x0001;
/// Bit 6: the entry is encrypted with strong encryption, which sets bit 0
/// too when the writer follows the APPNOTE.
const STRONG_ENCRYPTION: u16 = 0x0040;
/// Bit 13: the central directory is encrypted and the local header's values
/// are masked (APPNOTE 6.3.10 §4.4.4).
const MASKED_HEADER: u16 = 0x2000;

/// Refuses each entry whose name is too long, that is encrypted, or that is
/// compressed with a method a package may not use.
pub(super) fn fields(entries: &mut [Entry], limits: &ZipLimits, diagnostics: &mut Diagnostics) {
    let limit = limits.max_entry_name_bytes;
    for entry in entries {
        let length = entry.name.len() as u64;
        if length > limit.value {
            entry.refuse(Diagnostic::EntryNameTooLong { length, limit }, diagnostics);
        } else if entry.flags & (ENCRYPTED | STRONG_ENCRYPTION | MASKED_HEADER) != 0 {
            entry.refuse(Diagnostic::ZipEncryptedEntry {}, diagnostics);
        } else if let Method::Other(method) = entry.method {
            let method = u64::from(method);
            entry.refuse(
                Diagnostic::ZipUnsupportedCompression { method },
                diagnostics,
            );
        }
    }
}

/// Refuses each readable entry that declares more than one entry may
/// inflate to, or more than its compressed size allows. Then, of the rest
/// in directory order, refuses the one whose declared size takes the total
/// past its limit, and leaves every later one unread without a diagnostic of
/// its own: the one reported is their cause.
pub(super) fn sizes(entries: &mut [Entry], limits: &ZipLimits, diagnostics: &mut Diagnostics) {
    let limit = limits.max_entry_bytes;
    let ratio = limits.max_compression_ratio;
    for entry in entries.iter_mut().filter(|entry| entry.readable) {
        let size = entry.size;
        let compressed_size = entry.compressed_size;
        if size > limit.value {
            entry.refuse(Diagnostic::ZipEntryTooLarge { size, limit }, diagnostics);
        } else if size >= limits.min_ratio_checked_bytes.value
            && size > compressed_size.saturating_mul(ratio.value)
        {
            let diagnostic = Diagnostic::ZipRatioExceeded {
                size,
                compressed_size,
                limit: ratio,
            };
            entry.refuse(diagnostic, diagnostics);
        }
    }
    let limit = limits.max_total_bytes;
    let readable = || entries.iter().filter(|entry| entry.readable);
    let size = readable().fold(0_u64, |total, entry| total.saturating_add(entry.size));
    if size <= limit.value {
        return;
    }
    let mut total = 0_u64;
    let mut crossed = false;
    for entry in entries.iter_mut().filter(|entry| entry.readable) {
        total = total.saturating_add(entry.size);
        if crossed {
            entry.readable = false;
        } else if total > limit.value {
            crossed = true;
            entry.refuse(Diagnostic::ZipTotalTooLarge { size, limit }, diagnostics);
        }
    }
}
