//! Tests for `blank`: the bytes of the built-in workbook, that it opens
//! without a diagnostic, the worksheet each kind of `Defaults` gives, the
//! error for bytes that are not a package, and the create procedure the
//! module documents, run on it for first names that do and do not collide
//! with the built-in sheet's name.

use super::{DefaultFont, Defaults, SHEET, bytes, open_blank, package, styles};
use crate::structure::model::Workbook;
use crate::structure::{StructureEdit, StructureError, check_sheet_name};
use crate::tests::{open, read};
use crate::{PackageEdit, WriteError, WriteLimits};
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::MemorySpools;

/// The create procedure of the module's documentation: the workbook whose
/// sheets are `first` and then each of `more`, as the bytes of its zip.
fn create(first: &str, more: &[&str]) -> Vec<u8> {
    let defaults = Defaults::default();
    let limits = WriteLimits::default();
    let mut diagnostics = Diagnostics::new();
    let mut package = package(&defaults).unwrap();
    let mut structure = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    let mut built_in = SHEET;
    if let Err(StructureError::SheetNameTaken { .. }) = check_sheet_name(first, [SHEET]) {
        structure.rename_sheet(SHEET, "Sheet2").unwrap();
        built_in = "Sheet2";
    }
    structure.add_sheet(first, None, None).unwrap();
    structure.delete_sheet(built_in).unwrap();
    for name in more {
        structure.add_sheet(name, None, None).unwrap();
    }
    let mut edit = PackageEdit::new(package);
    structure.apply(&mut edit).unwrap();
    let finished = edit.finish(Vec::new(), &mut MemorySpools, &limits, &mut diagnostics);
    assert!(diagnostics.records().is_empty());
    finished.unwrap().unwrap()
}

/// The names of the sheets of the workbook in the zip `bytes`.
fn sheet_names(bytes: Vec<u8>) -> Vec<String> {
    let mut diagnostics = Diagnostics::new();
    let mut package = Package::open(Cursor::new(bytes), &Limits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let book = Workbook::read(&mut package, &WriteLimits::default(), &mut diagnostics);
    assert!(diagnostics.records().is_empty());
    let book = book.unwrap().unwrap();
    book.sheets().iter().map(|s| s.name().to_owned()).collect()
}

/// The names of the entries of the zip `bytes`, in order.
fn entry_names(bytes: Vec<u8>) -> Vec<String> {
    let archive = open(bytes);
    let names = archive.entries().iter().map(|e| e.name());
    names
        .map(|name| String::from_utf8(name.to_vec()).unwrap())
        .collect()
}

#[test]
fn the_zip_of_the_built_in_workbook_is_the_same_on_every_call() {
    assert_eq!(
        bytes(&Defaults::default()).unwrap(),
        bytes(&Defaults::default()).unwrap()
    );
}

#[test]
fn the_built_in_workbook_opens_without_a_diagnostic_and_holds_its_parts_in_order() {
    let zip = bytes(&Defaults::default()).unwrap();
    let mut diagnostics = Diagnostics::new();
    let opened = Package::open(
        Cursor::new(zip.clone()),
        &Limits::default(),
        &mut diagnostics,
    );
    assert!(opened.unwrap().is_some());
    assert!(diagnostics.records().is_empty());
    assert_eq!(
        entry_names(zip),
        [
            "[Content_Types].xml",
            "_rels/.rels",
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels",
            "xl/worksheets/sheet1.xml",
            "xl/styles.xml",
        ]
    );
}

/// The text of the entry at `index` of the zip `bytes`.
fn entry_text(bytes: Vec<u8>, index: usize) -> String {
    String::from_utf8(read(&mut open(bytes), index)).unwrap()
}

#[test]
fn the_built_in_workbook_has_one_sheet_named_sheet1_and_the_styles_and_sheet_format_of_the_defaults()
 {
    let defaults = Defaults::default();
    let zip = bytes(&defaults).unwrap();
    assert_eq!(sheet_names(zip.clone()), [SHEET]);
    assert_eq!(SHEET, "Sheet1");
    assert_eq!(entry_text(zip.clone(), 5), styles(defaults.font()));
    assert_eq!(
        entry_text(zip, 4),
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" ",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
            "<sheetFormatPr defaultRowHeight=\"15\"/><sheetData/></worksheet>",
        )
    );
}

#[test]
fn the_worksheet_always_has_the_sheet_format_pr_of_the_defaults_and_custom_height_only_when_set() {
    let yu_gothic = || DefaultFont::new("Yu Gothic", 11.0).unwrap();
    let cases = [
        (
            Defaults::default(),
            r#"<sheetFormatPr defaultRowHeight="15"/>"#,
        ),
        (
            Defaults::new(Some(yu_gothic()), None).unwrap(),
            r#"<sheetFormatPr defaultRowHeight="15" customHeight="1"/>"#,
        ),
        (
            Defaults::new(Some(yu_gothic()), Some(20.5)).unwrap(),
            r#"<sheetFormatPr defaultRowHeight="20.5" customHeight="1"/>"#,
        ),
        (
            Defaults::new(None, Some(0.0)).unwrap(),
            r#"<sheetFormatPr defaultRowHeight="0" customHeight="1"/>"#,
        ),
    ];
    for (defaults, element) in cases {
        let sheet = entry_text(bytes(&defaults).unwrap(), 4);
        assert!(
            sheet.ends_with(&format!("{element}<sheetData/></worksheet>")),
            "{sheet}"
        );
        assert_eq!(sheet.matches("sheetFormatPr").count(), 1, "{sheet}");
    }
}

#[test]
fn the_styles_part_has_the_font_of_the_defaults_with_its_name_escaped_as_an_attribute_value_only() {
    let zip = bytes(&Defaults::default()).unwrap();
    let text = entry_text(zip, 5);
    assert!(text.contains(r#"<font><sz val="11"/><name val="Calibri"/></font>"#));
    let odd = DefaultFont::new("A&B \"<x>\"\t_x0041_", 10.5).unwrap();
    let defaults = Defaults::new(Some(odd.clone()), None).unwrap();
    let text = entry_text(bytes(&defaults).unwrap(), 5);
    assert_eq!(text, styles(&odd));
    assert!(
        text.contains(
            r#"<font><sz val="10.5"/><name val="A&amp;B &quot;&lt;x&gt;&quot;&#9;_x0041_"/></font>"#
        ),
        "{text}"
    );
    assert!(text.contains(r#"<cellXfs count="1">"#));
}

#[test]
fn the_same_defaults_give_the_same_bytes_and_other_defaults_other_bytes() {
    let font = DefaultFont::new("Arial", 10.0).unwrap();
    let a = Defaults::new(Some(font), Some(18.0)).unwrap();
    assert_eq!(bytes(&a).unwrap(), bytes(&a).unwrap());
    assert_ne!(bytes(&a).unwrap(), bytes(&Defaults::default()).unwrap());
}

#[test]
fn package_is_the_package_open_blank_makes_of_the_zip() {
    let package = package(&Defaults::default()).unwrap();
    let names: Vec<&str> = package.parts().iter().map(|p| p.name().as_str()).collect();
    assert_eq!(
        names,
        [
            "/_rels/.rels",
            "/xl/workbook.xml",
            "/xl/_rels/workbook.xml.rels",
            "/xl/worksheets/sheet1.xml",
            "/xl/styles.xml",
        ]
    );
}

#[test]
fn open_blank_returns_blank_unreadable_for_bytes_that_are_not_a_package() {
    let got = open_blank(b"not a zip".to_vec());
    assert!(matches!(got, Err(WriteError::BlankUnreadable)));
    assert_eq!(
        WriteError::BlankUnreadable.to_string(),
        "the built-in blank workbook could not be opened"
    );
}

#[test]
fn the_create_procedure_gives_a_workbook_of_the_first_name_alone_for_names_that_collide_or_not() {
    for first in ["Data", "Sheet1", "sheet1", "SHEET1", "Sheet2"] {
        let zip = create(first, &[]);
        assert_eq!(sheet_names(zip.clone()), [first], "{first}");
        let names = entry_names(zip);
        assert_eq!(names[0], "[Content_Types].xml", "{first}");
    }
}

#[test]
fn the_create_procedure_adds_the_further_sheets_after_the_first_in_order() {
    let zip = create("sheet1", &["Second", "Third"]);
    assert_eq!(sheet_names(zip), ["sheet1", "Second", "Third"]);
}

#[test]
fn the_create_procedure_gives_the_same_bytes_for_the_same_input() {
    for first in ["Data", "Sheet1"] {
        assert_eq!(
            create(first, &["More"]),
            create(first, &["More"]),
            "{first}"
        );
    }
}
