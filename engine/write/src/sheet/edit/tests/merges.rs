//! `merge` and `unmerge`: what each refuses (and that the edit is as it was),
//! and the `mergeCells` written.

use super::{NS, TABLE, open, r, sheet, text};
use crate::sheet::SheetError;

const ONE: &str =
    r#"<mergeCells count="2"><mergeCell ref="A1:B2"/><mergeCell ref="D1:E2"/></mergeCells>"#;

#[test]
fn a_range_is_added_before_the_end_of_the_mergecells_and_counted() {
    let xml = sheet(&format!("<sheetData/>{ONE}<pageMargins/>"));
    let got = text(&xml, |edit| edit.merge(r("G1:H2")).unwrap());
    let expected = sheet(
        r#"<sheetData/><mergeCells count="3"><mergeCell ref="A1:B2"/><mergeCell ref="D1:E2"/><mergeCell ref="G1:H2"/></mergeCells><pageMargins/>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_part_with_no_mergecells_gets_one_where_the_schema_puts_it() {
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.merge(r("B2:A1")).unwrap();
        edit.merge(r("C3:D4")).unwrap();
    });
    let expected = sheet(
        r#"<sheetData/><mergeCells count="2"><mergeCell ref="A1:B2"/><mergeCell ref="C3:D4"/></mergeCells>"#,
    );
    assert_eq!(got, expected);
    let got = text(&sheet("<sheetData/><pageMargins/>"), |edit| {
        edit.merge(r("A1:B2")).unwrap();
    });
    let expected = sheet(
        r#"<sheetData/><mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells><pageMargins/>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn an_empty_mergecells_is_filled() {
    for empty in [
        "<mergeCells/>",
        r#"<mergeCells count="0"/>"#,
        "<mergeCells></mergeCells>",
    ] {
        let got = text(&sheet(&format!("<sheetData/>{empty}")), |edit| {
            edit.merge(r("A1:B2")).unwrap();
        });
        let expected =
            sheet(r#"<sheetData/><mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells>"#);
        assert_eq!(got, expected, "{empty}");
    }
}

#[test]
fn unmerge_leaves_out_the_range_and_the_rest_is_written_as_it_was() {
    let xml = sheet(
        "<sheetData/><mergeCells count='3'>\n  <mergeCell ref=\"A1:B2\" />\n  <mergeCell ref=\"D1:E2\"></mergeCell>\n  <mergeCell ref=\"G1:H2\"/>\n</mergeCells>",
    );
    let got = text(&xml, |edit| edit.unmerge(r("E2:D1")).unwrap());
    let expected = sheet(
        "<sheetData/><mergeCells count=\"2\">\n  <mergeCell ref=\"A1:B2\" />\n  \n  <mergeCell ref=\"G1:H2\"/>\n</mergeCells>",
    );
    assert_eq!(got, expected);
}

#[test]
fn a_mergecells_left_with_no_range_is_not_written() {
    let xml = sheet(&format!("<sheetData/>{ONE}<pageMargins/>"));
    let got = text(&xml, |edit| {
        edit.unmerge(r("A1:B2")).unwrap();
        edit.unmerge(r("D1:E2")).unwrap();
    });
    assert_eq!(got, sheet("<sheetData/><pageMargins/>"));
}

#[test]
fn unmerge_removes_a_range_added_and_every_equal_range_of_the_part() {
    let xml = sheet(
        r#"<sheetData/><mergeCells count="3"><mergeCell ref="A1:B2"/><mergeCell ref="A1:B2"/><mergeCell ref="D1:E2"/></mergeCells>"#,
    );
    let got = text(&xml, |edit| {
        edit.unmerge(r("A1:B2")).unwrap();
        edit.merge(r("G1:H2")).unwrap();
        edit.unmerge(r("G1:H2")).unwrap();
        edit.merge(r("A1:B2")).unwrap();
    });
    let expected = sheet(
        r#"<sheetData/><mergeCells count="2"><mergeCell ref="D1:E2"/><mergeCell ref="A1:B2"/></mergeCells>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_range_that_cannot_be_read_stays_and_is_counted_as_written() {
    let xml = sheet(r#"<sheetData/><mergeCells count="1"><mergeCell ref="bogus"/></mergeCells>"#);
    let got = text(&xml, |edit| edit.merge(r("A1:B2")).unwrap());
    let expected = sheet(
        r#"<sheetData/><mergeCells count="2"><mergeCell ref="bogus"/><mergeCell ref="A1:B2"/></mergeCells>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_child_of_mergecells_that_is_not_a_mergecell_is_kept() {
    let xml = sheet(
        r#"<sheetData/><mergeCells count="1"><mergeCell ref="A1:B2"/><other><a/></other><!--c--></mergeCells>"#,
    );
    let got = text(&xml, |edit| edit.merge(r("D1:E2")).unwrap());
    let expected = sheet(
        r#"<sheetData/><mergeCells count="2"><mergeCell ref="A1:B2"/><other><a/></other><!--c--><mergeCell ref="D1:E2"/></mergeCells>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn merge_refuses_what_excel_cannot_hold_and_leaves_the_edit_as_it_was() {
    let table = format!(r#"<table xmlns="{NS}" ref="J10:K12"/>"#);
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{}/table" Target="../tables/table1.xml"/></Relationships>"#,
        super::R
    );
    let extra = [
        ("xl/worksheets/_rels/sheet1.xml.rels", "", rels.as_str()),
        ("xl/tables/table1.xml", TABLE, table.as_str()),
    ];
    let xml = sheet(&format!("<sheetData/>{ONE}"));
    let mut package = super::package(super::zip(&[&xml], &extra));
    let mut diagnostics = xlsx_graft_diagnostics::Diagnostics::new();
    let mut edit = crate::sheet::SheetEdit::open(
        &mut package,
        &super::part(super::SHEET),
        &crate::WriteLimits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    let single = edit.merge(r("C3"));
    assert_eq!(single, Err(SheetError::MergeSingleCell { range: r("C3") }));
    // The part's range, a range added, a table.
    let over = edit.merge(r("B2:C3"));
    assert_eq!(
        over,
        Err(SheetError::MergeOverlap {
            range: r("B2:C3"),
            other: r("A1:B2")
        })
    );
    assert_eq!(edit.merge(r("G1:H2")), Ok(()));
    let again = edit.merge(r("H2:I3"));
    assert_eq!(
        again,
        Err(SheetError::MergeOverlap {
            range: r("H2:I3"),
            other: r("G1:H2")
        })
    );
    let table = edit.merge(r("K12:L13"));
    assert_eq!(
        table,
        Err(SheetError::MergeTable {
            range: r("K12:L13"),
            table: r("J10:K12")
        })
    );
    // Ranges that share an edge or a corner point only do not overlap.
    assert_eq!(edit.merge(r("C1:C2")), Ok(()));
    assert_eq!(edit.merge(r("C3:D4")), Ok(()));
    let missing = edit.unmerge(r("A1:A2"));
    assert_eq!(missing, Err(SheetError::MergeMissing { range: r("A1:A2") }));
    let mut pe = crate::PackageEdit::new(package);
    edit.apply(&mut pe).unwrap();
    let got = super::finish(pe).sheet();
    let expected = sheet(
        r#"<sheetData/><mergeCells count="5"><mergeCell ref="A1:B2"/><mergeCell ref="D1:E2"/><mergeCell ref="G1:H2"/><mergeCell ref="C1:C2"/><mergeCell ref="C3:D4"/></mergeCells>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn the_most_ranges_a_sheet_holds_are_262144() {
    let ranges: String = (1..=262_144)
        .map(|row| format!(r#"<mergeCell ref="A{row}:B{row}"/>"#))
        .collect();
    let xml = sheet(&format!(
        r#"<mergeCells count="262144">{ranges}</mergeCells>"#
    ));
    let mut edit = open(&xml);
    let error = SheetError::TooManyMerges {
        count: 262_145,
        limit: 262_144,
    };
    assert_eq!(edit.merge(r("D1:E1")), Err(error));
    edit.unmerge(r("A5:B5")).unwrap();
    assert_eq!(edit.merge(r("D1:E1")), Ok(()));
    assert_eq!(
        edit.merge(r("G1:H1")),
        Err(SheetError::TooManyMerges {
            count: 262_145,
            limit: 262_144
        })
    );
}
