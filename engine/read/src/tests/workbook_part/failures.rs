//! A damaged, refused, malformed or too large workbook part: its one
//! diagnostic, and the part not read.

use super::super::zip::Item;
use super::super::{NS, content_types, rel, rels};
use super::{PART, R, book_with, read, read_with};
use crate::ReadLimits;
use xlsx_graft_diagnostics::{Diagnostic, Limit};

/// A workbook part of one sheet, linked to nothing.
fn xml() -> String {
    format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}"><sheets><sheet name="A" sheetId="1" r:id="rId1"/></sheets></workbook>"#
    )
}

#[test]
fn a_damaged_entry_reports_its_crc_alone_even_where_its_xml_is_wrong() {
    // XML left malformed, then an element the schema does not have.
    for (from, to) in [(b"<sheets", b"<sheets "), (b"<sheets", b"<sheetx ")] {
        let mut items = book_with(&xml(), "", vec![]);
        items[2] = Item::stored("xl/workbook.xml", xml().as_bytes()).damaged(|data| {
            let at = data.windows(7).position(|w| w == from).unwrap();
            data[at..at + 8].copy_from_slice(to);
        });
        let (part, seen) = read(items);
        assert_eq!(part, None);
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            matches!(seen[0].0, Diagnostic::ZipCrcMismatch { .. }),
            "{seen:?}"
        );
    }
}

#[test]
fn an_entry_the_zip_refused_reports_nothing_more() {
    let main = rel(
        "rId1",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
        "xl/workbook.xml",
    );
    let items = vec![
        content_types(),
        rels("_rels/.rels", &main),
        Item::stored("xl/workbook.xml", xml().as_bytes()).with_method(99),
    ];
    let (part, seen) = read(items);
    assert_eq!(part, None);
    assert_eq!(seen.len(), 1, "{seen:?}");
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
    let xml = format!(r#"<workbook xmlns="{NS}"><sheets></workbook>"#);
    let (part, seen) = read(book_with(&xml, "", vec![]));
    assert_eq!(part, None);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(
        matches!(seen[0].0, Diagnostic::XmlMalformed { .. }),
        "{seen:?}"
    );
    assert_eq!(seen[0].1, PART);
}

#[test]
fn a_part_past_max_xml_bytes_is_too_large() {
    let mut limits = ReadLimits::default();
    let size = xml().len() as u64;
    limits.xml.max_bytes = Limit::default_of(size - 1);
    let (part, seen, _) = read_with(book_with(&xml(), "", vec![]), &limits);
    assert_eq!(part, None);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(
        matches!(seen[0].0, Diagnostic::XmlTooLarge { .. }),
        "{seen:?}"
    );
    assert_eq!(seen[0].1, PART);
}
