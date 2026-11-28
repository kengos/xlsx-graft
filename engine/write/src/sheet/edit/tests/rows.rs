//! `set_row_height`: the `row`s written, the heights set or removed in the
//! rows that are there, the rows added, and what is refused.

use super::{SHEET, package, part, run, sheet, text, zip};
use crate::sheet::{SheetEdit, SheetError};
use crate::{PackageEdit, WriteLimits};
use xlsx_graft_diagnostics::Diagnostics;

const DATA: &str = r#"<sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row><row r="3" ht="10" customHeight="1" spans="1:2"><c r="A3"/></row></sheetData>"#;

#[test]
fn a_height_is_set_on_a_row_and_replaces_the_one_it_had() {
    let got = text(&sheet(DATA), |edit| {
        edit.set_row_height(1, Some(30.0)).unwrap();
        edit.set_row_height(3, Some(15.5)).unwrap();
    });
    let expected = sheet(
        r#"<sheetData><row r="1" ht="30" customHeight="1"><c r="A1" t="s"><v>0</v></c></row><row r="3" ht="15.5" customHeight="1" spans="1:2"><c r="A3"/></row></sheetData>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_height_is_removed_with_its_customheight_and_the_other_attributes_stay() {
    let got = text(&sheet(DATA), |edit| edit.set_row_height(3, None).unwrap());
    let expected = sheet(
        r#"<sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row><row r="3" spans="1:2"><c r="A3"/></row></sheetData>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_row_the_part_lacks_is_written_in_order_with_its_height() {
    let got = text(&sheet(DATA), |edit| {
        edit.set_row_height(5, Some(2.0)).unwrap();
        edit.set_row_height(2, Some(1.0)).unwrap();
        edit.set_row_height(4, None).unwrap();
    });
    let expected = sheet(
        r#"<sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row><row r="2" ht="1" customHeight="1"/><row r="3" ht="10" customHeight="1" spans="1:2"><c r="A3"/></row><row r="5" ht="2" customHeight="1"/></sheetData>"#,
    );
    assert_eq!(got, expected);
    let late = sheet(r#"<sheetData><row r="3"/></sheetData>"#);
    let got = text(&late, |edit| edit.set_row_height(1, Some(1.0)).unwrap());
    let expected =
        sheet(r#"<sheetData><row r="1" ht="1" customHeight="1"/><row r="3"/></sheetData>"#);
    assert_eq!(got, expected);
}

#[test]
fn a_row_with_no_r_is_the_one_after_the_row_before() {
    let xml = sheet(r#"<sheetData><row><c/></row><row/><row r="5"/><row/></sheetData>"#);
    let got = text(&xml, |edit| {
        edit.set_row_height(1, Some(1.0)).unwrap();
        edit.set_row_height(2, None).unwrap();
        edit.set_row_height(4, Some(4.0)).unwrap();
        edit.set_row_height(6, Some(6.0)).unwrap();
    });
    let expected = sheet(
        r#"<sheetData><row ht="1" customHeight="1"><c/></row><row/><row r="4" ht="4" customHeight="1"/><row r="5"/><row ht="6" customHeight="1"/></sheetData>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn an_empty_sheetdata_is_filled_when_a_height_is_set_and_left_when_none_is() {
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.set_row_height(5, Some(2.0)).unwrap()
    });
    assert_eq!(
        got,
        sheet(r#"<sheetData><row r="5" ht="2" customHeight="1"/></sheetData>"#)
    );
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.set_row_height(5, None).unwrap()
    });
    assert_eq!(got, sheet("<sheetData/>"));
}

#[test]
fn a_part_with_no_sheetdata_gets_one_with_the_rows() {
    let xml = sheet("<pageMargins/>");
    let got = text(&xml, |edit| {
        edit.set_row_height(2, Some(1.0)).unwrap();
        edit.set_row_height(1, Some(3.0)).unwrap();
    });
    let expected = sheet(
        r#"<sheetData><row r="1" ht="3" customHeight="1"/><row r="2" ht="1" customHeight="1"/></sheetData><pageMargins/>"#,
    );
    assert_eq!(got, expected);
    // No height to write: no sheetData.
    let got = text(&xml, |edit| edit.set_row_height(2, None).unwrap());
    assert_eq!(got, xml);
}

#[test]
fn a_row_number_that_comes_again_is_written_as_it_was() {
    let xml = sheet(r#"<sheetData><row r="2"/><row r="2"/><row r="5"/><row r="3"/></sheetData>"#);
    let got = text(&xml, |edit| {
        edit.set_row_height(2, Some(1.0)).unwrap();
        edit.set_row_height(3, Some(3.0)).unwrap();
    });
    // Row 3 was written before row 5; the row 3 that comes later is as it was.
    let expected = sheet(
        r#"<sheetData><row r="2" ht="1" customHeight="1"/><row r="2"/><row r="3" ht="3" customHeight="1"/><row r="5"/><row r="3"/></sheetData>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn what_is_in_the_sheetdata_but_a_row_and_the_cells_are_written_as_they_were() {
    let xml = sheet(
        "<sheetData>\n <row r=\"1\"  spans='1:1' >\n  <c r=\"A1\"><f>1+1</f><v>2</v></c><!--c-->\n </row>\n <extra><a/></extra><extra/> text\n</sheetData>",
    );
    let got = text(&xml, |edit| edit.set_row_height(1, Some(8.0)).unwrap());
    let expected = sheet(
        "<sheetData>\n <row r=\"1\" spans=\"1:1\" ht=\"8\" customHeight=\"1\">\n  <c r=\"A1\"><f>1+1</f><v>2</v></c><!--c-->\n </row>\n <extra><a/></extra><extra/> text\n</sheetData>",
    );
    assert_eq!(got, expected);
}

#[test]
fn the_last_row_is_a_row_and_a_row_past_it_is_not() {
    let xml = sheet(r#"<sheetData><row r="1048576"/></sheetData>"#);
    let got = text(&xml, |edit| {
        edit.set_row_height(1_048_576, Some(1.0)).unwrap()
    });
    assert_eq!(
        got,
        sheet(r#"<sheetData><row r="1048576" ht="1" customHeight="1"/></sheetData>"#)
    );
}

/// What `finish` makes of the sheet `xml` with a height set: no zip, and
/// the diagnostics' codes with the part and the offset.
fn refused(xml: &str) -> Vec<String> {
    let mut package = package(zip(&[xml], &[]));
    let mut diagnostics = Diagnostics::new();
    let limits = WriteLimits::default();
    let mut edit = SheetEdit::open(&mut package, &part(SHEET), &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    edit.set_row_height(2, Some(1.0)).unwrap();
    let mut package_edit = PackageEdit::new(package);
    edit.apply(&mut package_edit).unwrap();
    let written = package_edit
        .finish(
            crate::tests::Sink::default(),
            &mut xlsx_graft_spool::MemorySpools,
            &limits,
            &mut diagnostics,
        )
        .unwrap();
    assert!(written.is_none());
    (diagnostics.records().iter())
        .map(|record| {
            let at = record
                .location
                .part
                .as_ref()
                .map_or("", |echo| echo.as_str());
            format!(
                "{} {} {}",
                record.diagnostic.code(),
                record.diagnostic.message(),
                at
            )
        })
        .collect()
}

#[test]
fn a_row_whose_r_is_not_a_row_refuses_the_write() {
    for bad in ["x", "0", "1048577", "-1", "", "1.5", "99999999999"] {
        let xml = sheet(&format!(r#"<sheetData><row r="{bad}"/></sheetData>"#));
        let got = refused(&xml);
        assert_eq!(got.len(), 1, "{bad}: {got:?}");
        assert!(got[0].starts_with("xml_schema_invalid "), "{bad}: {got:?}");
        assert!(got[0].contains("invalid_value: r"), "{bad}: {got:?}");
        assert!(got[0].ends_with(SHEET), "{bad}: {got:?}");
    }
    // A row with no `r` after the last row.
    let xml = sheet(r#"<sheetData><row r="1048576"/><row/></sheetData>"#);
    assert_eq!(refused(&xml).len(), 1);
}

#[test]
fn a_call_that_is_refused_changes_nothing() {
    let xml = sheet(DATA);
    let got = run(&xml, |edit| {
        assert_eq!(
            edit.set_row_height(0, Some(1.0)),
            Err(SheetError::RowRange { row: 0 })
        );
        assert_eq!(
            edit.set_row_height(1_048_577, None),
            Err(SheetError::RowRange { row: 1_048_577 })
        );
        for height in [-1.0, 409.5, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                edit.set_row_height(1, Some(height)),
                Err(SheetError::RowHeight { .. })
            ));
        }
        assert_eq!(edit.set_row_height(1, Some(409.0)), Ok(()));
        assert_eq!(edit.set_row_height(1, Some(0.0)), Ok(()));
    });
    assert!(
        got.sheet()
            .contains(r#"<row r="1" ht="0" customHeight="1">"#)
    );
}
