//! What the references of a report share: the name of the part, of the sheet
//! changed and of the table removed, and the `at` of the formulas inside one
//! cell or range, are one allocation each, not a copy for each reference.

use super::parts::{CHART, WORKSHEET, beta_with, book};
use super::{NS, R, TABLE, applied_with_report};
use crate::structure::{KeptReference, StructureReport};

/// The workbook `Alpha`, `Beta` whose first sheet has the table `table`
/// (a name) and whose second sheet is `sheet`, with `extra` parts, after
/// `Alpha` is deleted.
fn deleted_with_table(
    table: &str,
    sheet: &str,
    extra: &[(&str, &str, &str)],
    names: &str,
) -> StructureReport {
    let part = format!(r#"<table xmlns="{NS}" name="{table}" displayName="{table}" ref="A1:B3"/>"#);
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{R}/table" Target="../tables/table1.xml"/></Relationships>"#
    );
    let mut parts = vec![
        ("xl/worksheets/sheet2.xml", WORKSHEET, sheet),
        ("xl/worksheets/_rels/sheet1.xml.rels", "", &rels[..]),
        ("xl/tables/table1.xml", TABLE, &part[..]),
    ];
    parts.extend_from_slice(extra);
    let (_, report) = applied_with_report(book(names, &parts), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    report
}

/// The address of the text of `value`, to tell one allocation from a copy.
fn address(value: Option<&str>) -> usize {
    value.map_or(0, |text| text.as_ptr() as usize)
}

#[test]
fn the_references_to_one_table_share_its_name_as_its_part_writes_it() {
    let sheet = beta_with(
        r#"<c r="A1"><f>SUM(Sales[Amount])</f></c><c r="A2"><f>SUM(SALES[c])+sales[x]</f></c>"#,
    );
    let names = super::defined(&[("Tot", "", "Sales[Amount]")]);
    let report = deleted_with_table("Sales", &sheet, &[], &names);
    let tables: Vec<Option<&str>> = report.kept().iter().map(KeptReference::table).collect();
    assert_eq!(tables, [Some("Sales"); 3]);
    let addresses: Vec<usize> = tables.iter().map(|table| address(*table)).collect();
    assert!(addresses.iter().all(|address| *address == addresses[0]));
}

#[test]
fn a_table_name_of_60000_bytes_is_one_allocation_for_all_the_formulas_that_name_it() {
    let name = "T".repeat(60_000);
    let cell = |n: usize| format!(r#"<c r="A{n}"><f>{name}[c]</f></c>"#);
    let sheet = beta_with(&format!("{}{}{}", cell(1), cell(2), cell(3)));
    let report = deleted_with_table(&name, &sheet, &[], "");
    assert_eq!(report.kept().len(), 3);
    let tables: Vec<Option<&str>> = report.kept().iter().map(KeptReference::table).collect();
    assert_eq!(tables[0].map(str::len), Some(60_000));
    let addresses: Vec<usize> = tables.iter().map(|table| address(*table)).collect();
    assert!(addresses.iter().all(|address| *address == addresses[0]));
}

#[test]
fn the_references_of_a_part_share_its_name_and_two_parts_do_not() {
    let sheet = beta_with(r#"<c r="A1"><f>Alpha!"x</f></c><c r="A2"><f>Alpha!"y</f></c>"#);
    let graph = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:f>Alpha!"x</c:f><c:f>Alpha!"y</c:f></c:chartSpace>"#;
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..]),
        ("xl/charts/chart1.xml", CHART, graph),
    ];
    let (_, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let parts: Vec<usize> = (report.kept().iter())
        .map(|k| address(Some(k.part())))
        .collect();
    assert_eq!(parts.len(), 4);
    assert_eq!((parts[0], parts[2]), (parts[1], parts[3]));
    assert_ne!(parts[0], parts[2]);
}

#[test]
fn the_references_to_one_sheet_changed_share_its_name_in_every_part() {
    let sheet = beta_with(r#"<c r="A1"><f>Alpha!"x</f></c><c r="A2"><f>ALPHA!"y</f></c>"#);
    let graph = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:f>alpha!"x</c:f></c:chartSpace>"#;
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..]),
        ("xl/charts/chart1.xml", CHART, graph),
    ];
    let names = super::defined(&[("N", "", "Alpha!\"x")]);
    let (_, report) = applied_with_report(book(&names, &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let sheets: Vec<Option<&str>> = report.kept().iter().map(KeptReference::sheet).collect();
    assert_eq!(sheets, [Some("Alpha"); 4]);
    let addresses: Vec<usize> = sheets.iter().map(|sheet| address(*sheet)).collect();
    assert!(addresses.iter().all(|address| *address == addresses[0]));
}

#[test]
fn the_references_inside_one_cell_or_range_share_its_place() {
    let sheet = beta_with(concat!(
        r#"<c r="B2"><f>Alpha!"x</f><f>Alpha!"y</f></c>"#,
        r#"<conditionalFormatting sqref="D1:D9"><cfRule><dataBar><cfvo val="Alpha!$A$1"/><cfvo val="Alpha!$A$2"/></dataBar></cfRule></conditionalFormatting>"#,
    ));
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (_, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let ats: Vec<Option<&str>> = report.kept().iter().map(KeptReference::at).collect();
    assert_eq!(ats, [Some("B2"), Some("B2"), Some("D1:D9"), Some("D1:D9")]);
    let addresses: Vec<usize> = ats.iter().map(|at| address(*at)).collect();
    assert_eq!((addresses[0], addresses[2]), (addresses[1], addresses[3]));
    assert_ne!(addresses[0], addresses[2]);
}
