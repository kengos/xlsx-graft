//! The layout of the file before the central directory: the local records
//! the entries point to, in the order they start, must tile it exactly,
//! from the start of the file to the start of the directory, with no
//! overlap and no gap (APPNOTE 6.3.10 §4.3.6). A reader that walks the local
//! headers from the start then sees the same entries as one that reads the
//! central directory.
//!
//! Every entry is laid out, a refused one too, so the bytes it takes are not
//! taken for a gap; only a readable one gets a diagnostic for its record.
//! An entry is checked for overlap BEFORE its local header is read, and a
//! record's bytes are read for that record alone, so what the local headers
//! make the reader read is bounded by the file's size, however many entries
//! point at the same bytes.

use super::Entry;
use super::local::{self, Located};
use super::source::Source;
use std::io::{self, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};

/// Lays out every entry in the bytes before the central directory, which
/// starts at `directory`: refuses each readable entry whose record starts
/// before an earlier one ends (the earlier one keeps the bytes), runs into
/// the directory, or does not agree with the central directory; and reports
/// the bytes no record accounts for.
pub(super) fn check<R: Read + Seek>(
    source: &mut Source<R>,
    entries: &mut [Entry],
    directory: u64,
    diagnostics: &mut Diagnostics,
) -> io::Result<()> {
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&i| entries[i].local_offset);
    let starts: Vec<u64> = order.iter().map(|&i| entries[i].local_offset).collect();
    // Where the next record should start; `None` once a record's end is not
    // known, since a gap after it could not be told from the rest of it.
    let mut cursor = Some(0);
    // The record laid out last, which ends furthest.
    let mut furthest: Option<usize> = None;
    for i in order {
        let start = entries[i].local_offset;
        if start >= directory {
            refuse(
                &mut entries[i],
                Diagnostic::ZipEntryOverlapsDirectory {},
                diagnostics,
            );
            continue;
        }
        if let Some(other) = furthest.filter(|&other| start < entries[other].end_offset) {
            let other = Echo::from_utf8_lossy(&entries[other].name);
            refuse(
                &mut entries[i],
                Diagnostic::ZipEntriesOverlap { other },
                diagnostics,
            );
            continue;
        }
        if let Some(from) = cursor.filter(|&from| from < start) {
            unaccounted(from, start, entries[i].location(), diagnostics);
        }
        let Located { complete, problem } =
            local::locate(source, &mut entries[i], &starts, directory)?;
        if let Some(problem) = problem {
            refuse(&mut entries[i], problem, diagnostics);
        }
        cursor = complete.then_some(entries[i].end_offset);
        furthest = Some(i);
    }
    if let Some(from) = cursor.filter(|&from| from < directory) {
        unaccounted(from, directory, Location::default(), diagnostics);
    }
    Ok(())
}

/// Refuses `entry` with `diagnostic` if it is still readable; a refused
/// entry already has its diagnostic.
#[track_caller]
fn refuse(entry: &mut Entry, diagnostic: Diagnostic, diagnostics: &mut Diagnostics) {
    if entry.readable {
        entry.refuse(diagnostic, diagnostics);
    }
}

/// Reports the bytes from `from` to `to`, which no record accounts for, at
/// `location`: data in front of the zip when they start the file.
#[track_caller]
fn unaccounted(from: u64, to: u64, location: Location, diagnostics: &mut Diagnostics) {
    let size = to.saturating_sub(from);
    if from == 0 {
        diagnostics.push(Diagnostic::ZipPrependedData { size }, Location::default());
    } else {
        diagnostics.push(Diagnostic::ZipUnlistedData { size }, location);
    }
}
