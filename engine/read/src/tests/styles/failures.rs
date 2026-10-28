//! A damaged, refused or malformed styles part, and the failures of the
//! package's reader.

use super::super::zip::Item;
use super::super::{NS, open_poisoned};
use super::{PART, part, read, styled, styled_with};
use crate::{ReadError, ReadLimits, Styles, Workbook};
use std::cell::Cell;
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};
use xlsx_graft_package::PackageError;

#[test]
fn a_damaged_entry_reports_its_crc_alone_even_where_its_xml_is_wrong() {
    let xml = format!(r#"<styleSheet xmlns="{NS}"><cellXfs><xf/></cellXfs></styleSheet>"#);
    // XML left malformed, then an element the schema does not have.
    for (from, to) in [(b"<xf/", b"<xf "), (b"<xf/", b"<xx/")] {
        let damaged = part(&xml).damaged(|data| {
            let at = data.windows(4).position(|w| w == from).unwrap();
            data[at..at + 4].copy_from_slice(to);
        });
        let (styles, seen) = read(styled_with(damaged));
        assert_eq!(styles, Styles::Unknown);
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            matches!(seen[0].0, Diagnostic::ZipCrcMismatch { .. }),
            "{seen:?}"
        );
        assert_eq!(seen[0].1, "xl/styles.xml");
    }
}

#[test]
fn an_entry_the_zip_refused_reports_nothing_more() {
    let refused = Item::stored("xl/styles.xml", b"<styleSheet/>").with_method(99);
    let (styles, seen) = read(styled_with(refused));
    assert_eq!(styles, Styles::Unknown);
    assert_eq!(seen.len(), 1);
    assert!(
        matches!(
            seen[0].0,
            Diagnostic::ZipUnsupportedCompression { method: 99 }
        ),
        "{seen:?}"
    );
}

#[test]
fn xml_the_reader_refuses_is_its_diagnostic_alone() {
    let (styles, seen) = read(styled("<cellXfs><xf></cellXfs>"));
    assert_eq!(styles, Styles::Unknown);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(
        matches!(seen[0].0, Diagnostic::XmlMalformed { .. }),
        "{seen:?}"
    );
    assert_eq!(seen[0].1, PART);
}

#[test]
fn a_part_in_utf_16_reads() {
    let xml =
        format!(r#"<styleSheet xmlns="{NS}"><cellXfs><xf numFmtId="14"/></cellXfs></styleSheet>"#);
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let (styles, seen) = read(styled_with(Item::stored("xl/styles.xml", &bytes)));
    assert_eq!(seen, vec![]);
    assert_eq!(styles.kind(0), crate::FormatKind::Date);
}

/// Reads the styles of `items` with the reader failing from `from` bytes
/// into the styles entry's data to its end, `from` 0 failing the seek to
/// it too.
fn read_failing(items: Vec<Item>, from: u64) -> Result<Styles, ReadError> {
    let mut diagnostics = Diagnostics::new();
    let poisoned = Rc::new(Cell::new((0, 0)));
    let mut package = open_poisoned(items, &mut diagnostics, Rc::clone(&poisoned));
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let entry = package.part(PART).unwrap().entry();
    let range = package.archive().entries()[entry].data_range();
    poisoned.set((range.start + from, range.end + 1));
    Styles::read(
        &mut package,
        &workbook,
        &ReadLimits::default(),
        &mut diagnostics,
    )
}

#[test]
fn a_host_failure_opening_or_reading_the_part_is_an_error() {
    for from in [0, 10] {
        let read = read_failing(styled(&"<fonts/>".repeat(10)), from);
        assert!(
            matches!(read, Err(ReadError::Package(PackageError::Io(_)))),
            "{from}: {read:?}"
        );
    }
}
