//! `SheetEdit::open`: what is read of the part, and the problems that end
//! in `Ok(None)` with a diagnostic or in an `Err`.

use super::{NS, R, SHEET, STRICT_NS, TABLE, part, r, run, sheet, zip};
use crate::sheet::{SheetEdit, SheetError};
use crate::{WriteError, WriteLimits};
use std::io::Cursor;
use xlsx_graft_diagnostics::{Diagnostics, Limit};
use xlsx_graft_package::{Limits, Package};

/// The codes of the diagnostics, each with the part it points at.
fn found(diagnostics: &Diagnostics) -> Vec<String> {
    (diagnostics.records().iter())
        .map(|record| {
            let at = record
                .location
                .part
                .as_ref()
                .map_or("", |echo| echo.as_str());
            format!("{} {at}", record.diagnostic.code())
        })
        .collect()
}

/// `SheetEdit::open` on the sheet `xml`, with `limits`, and the diagnostics.
fn opened(
    xml: &str,
    extra: &[(&str, &str, &str)],
    limits: &WriteLimits,
) -> (Option<SheetEdit>, Vec<String>) {
    let mut package = super::package(zip(&[xml], extra));
    let mut diagnostics = Diagnostics::new();
    let edit = SheetEdit::open(&mut package, &part(SHEET), limits, &mut diagnostics);
    (edit.unwrap(), found(&diagnostics))
}

#[test]
fn a_part_the_package_lacks_or_a_relationships_part_is_an_error() {
    let mut package = super::package(zip(&[&sheet("")], &[]));
    let mut diagnostics = Diagnostics::new();
    let limits = WriteLimits::default();
    let missing = part("/xl/worksheets/sheet2.xml");
    let error = SheetEdit::open(&mut package, &missing, &limits, &mut diagnostics).unwrap_err();
    assert!(
        matches!(&error, WriteError::PartMissing { name } if name == "/xl/worksheets/sheet2.xml"),
        "{error:?}"
    );
    let rels = part("/xl/_rels/workbook.xml.rels");
    let error = SheetEdit::open(&mut package, &rels, &limits, &mut diagnostics).unwrap_err();
    assert!(
        matches!(error, WriteError::PartReserved { .. }),
        "{error:?}"
    );
    assert!(diagnostics.records().is_empty());
}

#[test]
fn a_root_that_is_not_a_worksheet_of_the_main_namespace_is_refused() {
    for xml in [
        "<other/>".to_owned(),
        r#"<worksheet xmlns="urn:other"/>"#.to_owned(),
        "<worksheet/>".to_owned(),
    ] {
        let (edit, codes) = opened(&xml, &[], &WriteLimits::default());
        assert!(edit.is_none(), "{xml}");
        assert_eq!(codes, [format!("xml_schema_invalid {SHEET}")], "{xml}");
    }
}

#[test]
fn a_worksheet_of_the_transitional_or_the_strict_namespace_or_with_a_prefix_is_opened() {
    for xml in [
        sheet(""),
        format!(r#"<worksheet xmlns="{STRICT_NS}"/>"#),
        format!(r#"<x:worksheet xmlns:x="{NS}"/>"#),
    ] {
        let (edit, codes) = opened(&xml, &[], &WriteLimits::default());
        assert!(edit.is_some(), "{xml}");
        assert!(codes.is_empty(), "{xml}");
    }
}

#[test]
fn xml_that_is_not_well_formed_or_past_the_limits_is_a_diagnostic() {
    let (edit, codes) = opened(
        &format!("<worksheet xmlns=\"{NS}\"><a></b></worksheet>"),
        &[],
        &WriteLimits::default(),
    );
    assert!(edit.is_none());
    assert_eq!(codes, [format!("xml_malformed {SHEET}")]);
    let mut limits = WriteLimits::default();
    limits.xml.max_depth = Limit::default_of(2);
    let (edit, codes) = opened(&sheet("<a><b/></a>"), &[], &limits);
    assert!(edit.is_none());
    assert_eq!(codes, [format!("xml_too_deep {SHEET}")]);
}

#[test]
fn a_part_whose_xml_is_damaged_in_the_zip_is_a_diagnostic_of_the_xml_reader() {
    let xml = sheet("<sheetData/>");
    let mut bytes = zip(&[&xml], &[]);
    // The stored data of the sheet: flip a byte, so its CRC-32 is wrong.
    let at = bytes
        .windows(xml.len())
        .position(|w| w == xml.as_bytes())
        .unwrap();
    bytes[at + xml.len() - 3] ^= 1;
    let mut package = Package::open(
        Cursor::new(bytes),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    let mut diagnostics = Diagnostics::new();
    let edit = SheetEdit::open(
        &mut package,
        &part(SHEET),
        &WriteLimits::default(),
        &mut diagnostics,
    );
    assert!(edit.unwrap().is_none());
    assert_eq!(found(&diagnostics).len(), 1, "{:?}", found(&diagnostics));
}

#[test]
fn a_part_whose_crc_is_wrong_is_a_diagnostic_from_the_zip_reader() {
    let mut bytes = zip(&[&sheet("<sheetData/>")], &[]);
    // The CRC-32 of the sheet, in its local header and its central directory
    // entry.
    let name = b"xl/worksheets/sheet1.xml";
    let at: Vec<usize> = (46..bytes.len() - name.len())
        .filter(|&at| &bytes[at..at + name.len()] == name)
        .collect();
    // The name follows the 30 bytes of the local header and the 46 bytes of
    // the central directory entry; the names in the content types stream and
    // the relationships do not.
    let signed = |signature: &[u8], size: usize| {
        at.iter()
            .map(|at| at - size)
            .find(|&start| bytes[start..start + 4] == *signature)
            .unwrap()
    };
    let (local, central) = (signed(b"PK\x03\x04", 30), signed(b"PK\x01\x02", 46));
    bytes[local + 14] ^= 1;
    bytes[central + 16] ^= 1;
    let mut package = Package::open(
        Cursor::new(bytes),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    let mut diagnostics = Diagnostics::new();
    let limits = WriteLimits::default();
    let edit = SheetEdit::open(&mut package, &part(SHEET), &limits, &mut diagnostics);
    assert!(edit.unwrap().is_none());
    assert_eq!(found(&diagnostics).len(), 1, "{:?}", found(&diagnostics));
}

#[test]
fn a_part_the_package_refused_when_it_was_opened_is_none_with_no_diagnostic_of_its_own() {
    let mut bytes = zip(&[&sheet("<sheetData/>")], &[]);
    // An unsupported compression method (99) in the local header and the
    // central directory entry of the sheet.
    let name = b"xl/worksheets/sheet1.xml";
    let at: Vec<usize> = (0..bytes.len() - name.len())
        .filter(|&at| &bytes[at..at + name.len()] == name)
        .collect();
    for at in at {
        let (local, central) = (at.saturating_sub(30), at.saturating_sub(46));
        if bytes[local..local + 4] == *b"PK\x03\x04" {
            bytes[local + 8] = 99;
        }
        if bytes[central..central + 4] == *b"PK\x01\x02" {
            bytes[central + 10] = 99;
        }
    }
    let mut package = Package::open(
        Cursor::new(bytes),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    let mut diagnostics = Diagnostics::new();
    let limits = WriteLimits::default();
    let edit = SheetEdit::open(&mut package, &part(SHEET), &limits, &mut diagnostics);
    assert!(edit.unwrap().is_none());
    assert!(diagnostics.records().is_empty());
}

#[test]
fn the_tables_of_the_sheet_are_read_from_its_relationships() {
    // With a declaration, a comment and white space around the root.
    let table = |ref_| {
        format!(
            "<?xml version=\"1.0\"?>\n<!-- table -->\n<table xmlns=\"{NS}\" id=\"1\" name=\"T\" displayName=\"T\" ref=\"{ref_}\"/>\n"
        )
    };
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{R}/table" Target="../tables/table1.xml"/><Relationship Id="rId2" Type="{R}/hyperlink" Target="http://example.com/" TargetMode="External"/><Relationship Id="rId3" Type="{R}/table" Target="http://example.com/t.xml" TargetMode="External"/><Relationship Id="rId4" Type="{R}/table" Target="../tables/table2.xml"/><Relationship Id="rId5" Type="{R}/drawing" Target="../tables/table3.xml"/></Relationships>"#
    );
    let (t1, t2, t3) = (table("B2:C5"), table("not a range"), table("A1:A9"));
    let extra = [
        ("xl/worksheets/_rels/sheet1.xml.rels", "", rels.as_str()),
        ("xl/tables/table1.xml", TABLE, &t1),
        ("xl/tables/table2.xml", TABLE, &t2),
        ("xl/tables/table3.xml", TABLE, &t3),
    ];
    let xml = sheet("<sheetData/>");
    let (edit, codes) = opened(&xml, &extra, &WriteLimits::default());
    assert!(codes.is_empty(), "{codes:?}");
    let mut edit = edit.unwrap();
    // The `ref` of table1 is a table; table2's is not a range and table3 is
    // not a table of the sheet (a drawing relationship targets it).
    assert_eq!(
        edit.merge(r("A1:B2")),
        Err(SheetError::MergeTable {
            range: r("A1:B2"),
            table: r("B2:C5")
        })
    );
    assert_eq!(edit.merge(r("A1:A2")), Ok(()));
    assert_eq!(edit.merge(r("A8:A9")), Ok(()));
}

#[test]
fn a_table_part_that_cannot_be_read_ends_the_open() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{R}/table" Target="../tables/table1.xml"/></Relationships>"#
    );
    let xml = sheet("<sheetData/>");
    let broken = ("xl/tables/table1.xml", TABLE, "<table><a></b></table>");
    let extra = [
        ("xl/worksheets/_rels/sheet1.xml.rels", "", rels.as_str()),
        broken,
    ];
    let (edit, codes) = opened(&xml, &extra, &WriteLimits::default());
    assert!(edit.is_none());
    assert_eq!(codes, ["xml_malformed /xl/tables/table1.xml"]);
    // Past the limit on how much XML is read into memory.
    let mut limits = WriteLimits::default();
    limits.xml.max_bytes = Limit::default_of(5);
    let ok = ("xl/tables/table1.xml", TABLE, "<table ref=\"A1:B2\" />");
    let extra = [
        ("xl/worksheets/_rels/sheet1.xml.rels", "", rels.as_str()),
        ok,
    ];
    let (edit, codes) = opened(&xml, &extra, &limits);
    assert!(edit.is_none());
    assert_eq!(codes, ["xml_too_large /xl/tables/table1.xml"]);
}

#[test]
fn a_call_that_changes_nothing_does_not_replace_the_part() {
    let xml = sheet(r#"<sheetData/><mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells>"#);
    let got = run(&xml, |_| {});
    assert!(got.holds(&xml));
    // A call that returns an error changes nothing either.
    let got = run(&xml, |edit| {
        assert!(edit.merge(r("A1:B2")).is_err());
        assert!(edit.unmerge(r("D1:E2")).is_err());
        assert!(edit.set_column_width(0, 1, Some(1.0)).is_err());
        assert!(edit.set_row_height(0, Some(1.0)).is_err());
        assert!(edit.freeze_panes(1_048_576, 0).is_err());
    });
    assert!(got.holds(&xml));
}

#[test]
fn a_part_the_package_lost_is_missing_when_the_edit_is_applied() {
    let bytes = zip(&[&sheet("<sheetData/>")], &[]);
    let mut package = super::package(bytes);
    let mut sheet_edit = SheetEdit::open(
        &mut package,
        &part(SHEET),
        &WriteLimits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    sheet_edit.merge(r("A1:B2")).unwrap();
    let mut edit = crate::PackageEdit::new(package);
    edit.remove(&part(SHEET)).unwrap();
    let error = sheet_edit.apply(&mut edit).unwrap_err();
    assert!(matches!(error, WriteError::PartMissing { .. }), "{error:?}");
}
