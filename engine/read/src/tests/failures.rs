//! A damaged or refused entry, and the failures of the host: the package's
//! reader and the spool.

use super::zip::Item;
use super::{NS, Seen, book, book_with, open_poisoned, read, read_with};
use crate::{ReadError, ReadLimits, SharedStrings, Workbook};
use std::cell::Cell;
use std::io;
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit};
use xlsx_graft_package::PackageError;
use xlsx_graft_spool::{MemorySpools, Spool, SpoolError, SpoolFactory};

/// The shared strings part of `xml`, its stored bytes changed by `damage`
/// after the CRC-32 was taken.
fn damaged(xml: &str, damage: impl FnOnce(&mut Vec<u8>)) -> Vec<Item> {
    book_with(Item::stored("xl/sharedStrings.xml", xml.as_bytes()).damaged(damage))
}

/// Asserts `seen` is `zip_crc_mismatch` against the shared strings entry
/// alone.
fn assert_crc_mismatch(seen: &Seen) {
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(
        matches!(seen[0].0, Diagnostic::ZipCrcMismatch { .. }),
        "{seen:?}"
    );
    assert_eq!(seen[0].1, "xl/sharedStrings.xml");
}

#[test]
fn a_damaged_entry_read_whole_reports_its_crc_alone() {
    let xml = format!(r#"<sst xmlns="{NS}"><si><t>a</t></si></sst>"#);
    let (strings, seen) = read(damaged(&xml, |data| {
        let at = data.len() - 16;
        data[at] = b'b';
    }));
    assert!(matches!(strings, SharedStrings::Unknown));
    assert_crc_mismatch(&seen);
}

#[test]
fn a_problem_found_in_a_damaged_entry_gives_way_to_its_crc() {
    let xml = format!(r#"<sst xmlns="{NS}"><si><t>a</t></si></sst>"#);
    // An element the schema does not have, then XML that is malformed.
    for (from, to) in [(b"<si>", b"<sx>"), (b"</t>", b"</x>")] {
        let (strings, seen) = read(damaged(&xml, |data| {
            let at = data.windows(4).position(|w| w == from).unwrap();
            data[at..at + 4].copy_from_slice(to);
        }));
        assert!(matches!(strings, SharedStrings::Unknown));
        assert_crc_mismatch(&seen);
    }
    // And a limit.
    let limits = ReadLimits {
        max_shared_strings: Limit::default_of(0),
        ..ReadLimits::default()
    };
    let items = damaged(&xml, |data| data.push(b' '));
    let (strings, seen) = read_with(items, &limits, &mut MemorySpools);
    assert!(matches!(strings, SharedStrings::Unknown));
    assert_crc_mismatch(&seen);
}

#[test]
fn an_entry_the_zip_refused_reports_nothing_more() {
    let part = Item::stored("xl/sharedStrings.xml", b"<sst/>").with_method(99);
    let (strings, seen) = read(book_with(part));
    assert!(matches!(strings, SharedStrings::Unknown));
    assert_eq!(seen.len(), 1);
    assert!(
        matches!(
            seen[0].0,
            Diagnostic::ZipUnsupportedCompression { method: 99 }
        ),
        "{seen:?}"
    );
}

/// Reads the shared strings of `items` with the reader failing from
/// `from` bytes into the shared strings entry's data to its end, `from` 0
/// failing the seek to it too.
fn read_failing(items: Vec<Item>, from: u64) -> Result<SharedStrings, ReadError> {
    let mut diagnostics = Diagnostics::new();
    let poisoned = Rc::new(Cell::new((0, 0)));
    let mut package = open_poisoned(items, &mut diagnostics, Rc::clone(&poisoned));
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let part = package.part("/xl/sharedStrings.xml").unwrap().entry();
    let range = package.archive().entries()[part].data_range();
    poisoned.set((range.start + from, range.end + 1));
    let limits = ReadLimits::default();
    let mut spools = MemorySpools;
    SharedStrings::read(
        &mut package,
        &workbook,
        &limits,
        &mut spools,
        &mut diagnostics,
    )
}

/// Whether `result` is a failure of the package's reader.
fn is_host_failure(result: &Result<SharedStrings, ReadError>) -> bool {
    matches!(result, Err(ReadError::Package(PackageError::Io(_))))
}

/// A table of strings that runs to about 200 KB.
fn large(first: &str) -> Vec<Item> {
    book(&format!(
        "{first}{}",
        "<si><t>0123456789</t></si>".repeat(8_000)
    ))
}

#[test]
fn a_host_failure_opening_the_part_is_an_error() {
    assert!(is_host_failure(&read_failing(book("<si/>"), 0)));
}

#[test]
fn a_host_failure_reading_the_part_is_an_error() {
    assert!(is_host_failure(&read_failing(large(""), 1_000)));
}

#[test]
fn a_host_failure_reading_past_a_problem_is_an_error() {
    assert!(is_host_failure(&read_failing(large("<x/>"), 100_000)));
}

/// Spools that cannot be made.
struct Broken;

impl SpoolFactory for Broken {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Err(io::Error::other("no room"))
    }
}

#[test]
fn a_spool_that_fails_is_an_error() {
    let limits = ReadLimits {
        shared_strings_spill_bytes: 0,
        ..ReadLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let mut package = super::open(book("<si><t>a</t></si>"), &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let read = SharedStrings::read(
        &mut package,
        &workbook,
        &limits,
        &mut Broken,
        &mut diagnostics,
    );
    assert!(matches!(read, Err(ReadError::Spool(SpoolError::Io(_)))));
}
