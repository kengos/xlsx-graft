//! `SheetEdit::apply` after `StructureEdit::apply` on the same
//! `PackageEdit`: what `StructureEdit` writes in the sheet passes through,
//! and a sheet it deleted is missing.

use super::{SHEET, finish, package, part, r, sheet, zip_with};
use crate::sheet::{SheetEdit, SheetProtection};
use crate::structure::StructureEdit;
use crate::{PackageEdit, WriteError, WriteLimits};
use xlsx_graft_diagnostics::Diagnostics;

const VIEWS: &str = r#"<bookViews><workbookView activeTab="0"/></bookViews>"#;

fn alpha() -> String {
    sheet(
        r#"<sheetViews><sheetView tabSelected="1" workbookViewId="0"/></sheetViews><sheetData><row r="1"><c r="A1"><f>Alpha!B1</f></c></row></sheetData>"#,
    )
}

#[test]
fn what_the_structure_edit_wrote_passes_through_and_the_sheet_edit_adds_its_own() {
    let bytes = zip_with(VIEWS, &[&alpha(), &sheet("<sheetData/>")], &[]);
    let mut package = package(bytes);
    let limits = WriteLimits::default();
    let mut diagnostics = Diagnostics::new();
    let mut structure = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    structure.rename_sheet("Alpha", "Renamed").unwrap();
    structure.set_active_sheet("Beta").unwrap();
    let mut sheet_edit = SheetEdit::open(&mut package, &part(SHEET), &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    sheet_edit.merge(r("A1:B2")).unwrap();
    sheet_edit.set_column_width(2, 2, Some(9.0)).unwrap();
    sheet_edit.protect(SheetProtection::default());
    let mut edit = PackageEdit::new(package);
    structure.apply(&mut edit).unwrap();
    sheet_edit.apply(&mut edit).unwrap();
    let got = finish(edit).sheet();
    let expected = sheet(
        r#"<sheetViews><sheetView workbookViewId="0"/></sheetViews><cols><col min="2" max="2" width="9" customWidth="1"/></cols><sheetData><row r="1"><c r="A1"><f>Renamed!B1</f></c></row></sheetData><sheetProtection sheet="1" objects="1" scenarios="1"/><mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells>"#,
    );
    assert_eq!(got, expected);
    assert!(diagnostics.records().is_empty());
}

#[test]
fn a_sheet_the_structure_edit_deleted_is_missing_and_one_it_added_has_no_part_to_open() {
    let bytes = zip_with(VIEWS, &[&alpha(), &sheet("<sheetData/>")], &[]);
    let mut package = package(bytes);
    let limits = WriteLimits::default();
    let mut diagnostics = Diagnostics::new();
    let mut structure = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    structure.delete_sheet("Alpha").unwrap();
    structure.add_sheet("New", None, None).unwrap();
    let mut sheet_edit = SheetEdit::open(&mut package, &part(SHEET), &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    sheet_edit.merge(r("A1:B2")).unwrap();
    let added = part("/xl/worksheets/sheet3.xml");
    let error = SheetEdit::open(&mut package, &added, &limits, &mut diagnostics).unwrap_err();
    assert!(matches!(error, WriteError::PartMissing { .. }), "{error:?}");
    let mut edit = PackageEdit::new(package);
    structure.apply(&mut edit).unwrap();
    let error = sheet_edit.apply(&mut edit).unwrap_err();
    assert!(
        matches!(&error, WriteError::PartMissing { name } if name == SHEET),
        "{error:?}"
    );
}
