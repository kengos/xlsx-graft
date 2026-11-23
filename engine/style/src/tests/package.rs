//! A [`StylesEdit`] given to `PackageEdit::replace` as the rewriter of the
//! styles part of a workbook of `fixtures/`: the cell format it adds is read
//! back, a part it adds nothing to is the same bytes, and a styles part that
//! cannot be read makes `finish` report it and write nothing.

use super::read_back::workbook;
use crate::{Font, Style, StyleLimits, StylesEdit};
use std::fs::File;
use std::io::Cursor;
use std::path::Path;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package, PartName, Presence};
use xlsx_graft_read::{STYLES, Styles, Workbook};
use xlsx_graft_spool::TempFiles;
use xlsx_graft_write::{PackageEdit, WriteLimits};

const STYLES_PART: &str = "/xl/styles.xml";

/// The name of the styles part.
fn name() -> PartName {
    PartName::parse(STYLES_PART).unwrap()
}

/// `table.xlsx` of `fixtures/`, opened.
fn table() -> Package<File> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/table.xlsx");
    let file = File::open(path).unwrap();
    let package = Package::open(file, &Limits::default(), &mut Diagnostics::new());
    package.unwrap().unwrap()
}

/// `package` with its styles part written by `edit`, finished into bytes, and
/// the diagnostics `finish` pushed.
fn finished<R: std::io::Read + std::io::Seek>(
    package: Package<R>,
    edit: StylesEdit,
) -> (Option<Vec<u8>>, Diagnostics) {
    let mut package_edit = PackageEdit::new(package);
    package_edit.replace(&name(), Box::new(edit)).unwrap();
    let mut diagnostics = Diagnostics::new();
    let written = package_edit.finish(
        Vec::new(),
        &mut TempFiles::new(),
        &WriteLimits::default(),
        &mut diagnostics,
    );
    (written.unwrap(), diagnostics)
}

/// The bytes of the styles part and the number of its cell formats in the zip
/// `bytes`.
fn read_back(bytes: Vec<u8>) -> (Vec<u8>, usize) {
    let mut diagnostics = Diagnostics::new();
    let reader = Cursor::new(bytes);
    let mut package = Package::open(reader, &Limits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let book = Workbook::find(&package, &mut diagnostics).unwrap();
    let Presence::Part(at) = book.related(&package, &STYLES, &mut diagnostics) else {
        panic!("no styles part");
    };
    let limits = xlsx_graft_read::ReadLimits::default();
    let part = package
        .read_xml_part(at, limits.xml.max_bytes, &mut diagnostics)
        .unwrap()
        .unwrap();
    let styles = Styles::read(&mut package, &book, &limits, &mut diagnostics).unwrap();
    let Styles::Formats(kinds) = styles else {
        panic!("{styles:?}");
    };
    assert!(diagnostics.records().is_empty());
    (part, kinds.len())
}

/// The bytes of the styles part of `table.xlsx`.
fn original() -> Vec<u8> {
    let mut package = table();
    let at = (package.parts().iter())
        .position(|part| part.name().as_str() == STYLES_PART)
        .unwrap();
    let limit = StyleLimits::default().xml.max_bytes;
    let mut diagnostics = Diagnostics::new();
    let bytes = package.read_xml_part(at, limit, &mut diagnostics);
    bytes.unwrap().unwrap()
}

#[test]
fn a_cell_format_added_is_one_more_cell_format_in_the_written_workbook() {
    let bytes = original();
    let limits = StyleLimits::default();
    let untouched = finished(table(), StylesEdit::read(&bytes, &limits).unwrap());
    let (_, before) = read_back(untouched.0.unwrap());
    let mut edit = StylesEdit::read(&bytes, &limits).unwrap();
    let bold = Style {
        font: Some(Font {
            bold: Some(true),
            ..Font::default()
        }),
        ..Style::default()
    };
    let index = edit.cell_format(0, &bold).unwrap();
    let (written, diagnostics) = finished(table(), edit);
    assert!(diagnostics.records().is_empty());
    let (part, after) = read_back(written.unwrap());
    assert_eq!(after, before + 1);
    assert_eq!(usize::try_from(index).unwrap(), before);
    assert_ne!(part, bytes);
}

#[test]
fn a_part_nothing_was_added_to_is_the_same_bytes_in_the_written_workbook() {
    let bytes = original();
    let edit = StylesEdit::read(&bytes, &StyleLimits::default()).unwrap();
    let (written, diagnostics) = finished(table(), edit);
    assert!(diagnostics.records().is_empty());
    assert_eq!(read_back(written.unwrap()).0, bytes);
}

#[test]
fn a_styles_part_that_cannot_be_read_makes_finish_report_it_and_write_nothing() {
    let valid =
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#;
    let edit = StylesEdit::read(valid.as_bytes(), &StyleLimits::default()).unwrap();
    let broken = workbook("<styleSheet><fonts>");
    let mut diagnostics = Diagnostics::new();
    let reader = Cursor::new(broken);
    let package = Package::open(reader, &Limits::default(), &mut diagnostics);
    let (written, diagnostics) = finished(package.unwrap().unwrap(), edit);
    assert!(written.is_none());
    let codes: Vec<String> = (diagnostics.records().iter())
        .map(|record| record.diagnostic.code().to_string())
        .collect();
    assert_eq!(codes.len(), 1, "{codes:?}");
    assert!(codes[0].starts_with("xml_"), "{codes:?}");
}
