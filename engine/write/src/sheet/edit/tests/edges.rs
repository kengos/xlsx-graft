//! Cases that the mutation run found with no test: the limits the rewriter reads
//! under, what `open` takes for a merged range, a `pane` or a table `ref`
//! (only where the schema puts it), elements named `col` or `row` that are
//! not those of the main namespace, and a width past the last of several
//! `cols`.

use super::{NS, TABLE, finish, open, package, part, r, run, sheet, text, zip};
use crate::sheet::SheetEdit;
use crate::{PackageEdit, WriteLimits};
use xlsx_graft_diagnostics::{Diagnostics, Limit};

#[test]
fn the_rewriter_reads_under_the_limits_the_edit_was_opened_with() {
    // 300 levels deep: past the default depth limit, inside the one given.
    let deep = format!("<foo>{}{}</foo>", "<x>".repeat(300), "</x>".repeat(300));
    let xml = sheet(&format!("<sheetData/>{deep}"));
    let mut package = package(zip(&[&xml], &[]));
    let mut limits = WriteLimits::default();
    limits.xml.max_depth = Limit::default_of(1000);
    let mut sheet_edit = SheetEdit::open(
        &mut package,
        &part(super::SHEET),
        &limits,
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    sheet_edit.merge(r("A1:B2")).unwrap();
    let mut edit = PackageEdit::new(package);
    sheet_edit.apply(&mut edit).unwrap();
    let expected = sheet(&format!(
        r#"<sheetData/>{deep}<mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells>"#
    ));
    // Not `assert_eq!`: the text is long.
    assert!(finish(edit).sheet() == expected);
}

#[test]
fn each_mergecells_is_counted_by_its_position_and_a_mergecell_outside_one_is_not_a_range() {
    let xml = sheet(
        r#"<sheetData/><mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells><foo><mergeCell ref="D1:E2"/></foo>"#,
    );
    let mut edit = open(&xml);
    assert_eq!(edit.merge(r("D1:E2")), Ok(()));
}

/// Whether `freeze_panes(0, 0)` replaces the part of `body`: it does when
/// the first `sheetView` of `workbookViewId` 0 of `sheetViews` has a `pane`.
fn unfreezing_replaces(body: &str) -> bool {
    let xml = sheet(body);
    !run(&xml, |edit| edit.freeze_panes(0, 0).unwrap()).holds(&xml)
}

#[test]
fn only_a_pane_in_the_first_sheetview_of_view_0_in_sheetviews_counts() {
    let view = r#"<sheetView workbookViewId="0"><pane ySplit="1"/></sheetView>"#;
    assert!(unfreezing_replaces(&format!(
        "<sheetViews>{view}</sheetViews>"
    )));
    for body in [
        // Elsewhere than in `sheetViews`.
        r#"<sheetData><row workbookViewId="0"><pane/></row></sheetData>"#.to_owned(),
        format!("<foo>{view}</foo>"),
        // After the `sheetViews` ended.
        format!(r#"<sheetViews><sheetView workbookViewId="1"/></sheetViews><foo>{view}</foo>"#),
        // A second `sheetView` of view 0, and a `pane` after the first one
        // ended, inside another child of `sheetViews`.
        format!(
            r#"<sheetViews><sheetView workbookViewId="0"/>{view}</sheetViews>"#
        ),
        r#"<sheetViews><sheetView workbookViewId="0"></sheetView><extLst><pane/></extLst></sheetViews>"#
            .to_owned(),
        // A child of `sheetViews` that is not a `sheetView`.
        r#"<sheetViews><extLst workbookViewId="0"><pane/></extLst></sheetViews>"#.to_owned(),
    ] {
        assert!(!unfreezing_replaces(&body), "{body}");
    }
}

#[test]
fn only_the_ref_of_the_root_table_element_is_read() {
    let table = |body: &str| format!(r#"<table xmlns="{NS}" ref="B2:C5">{body}</table>"#);
    let other_root = format!(r#"<other xmlns="{NS}" ref="B2:C5"/>"#);
    let nested = table(r#"<table ref="J10:K12"/>"#);
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{}/table" Target="../tables/table1.xml"/></Relationships>"#,
        super::R
    );
    let xml = sheet("<sheetData/>");
    for (content, merges) in [(nested.as_str(), "J10:K12"), (other_root.as_str(), "B2:C5")] {
        let extra = [
            ("xl/worksheets/_rels/sheet1.xml.rels", "", rels.as_str()),
            ("xl/tables/table1.xml", TABLE, content),
        ];
        let mut package = package(zip(&[&xml], &extra));
        let mut diagnostics = Diagnostics::new();
        let mut edit = SheetEdit::open(
            &mut package,
            &part(super::SHEET),
            &WriteLimits::default(),
            &mut diagnostics,
        )
        .unwrap()
        .unwrap();
        assert_eq!(edit.merge(r(merges)), Ok(()), "{content}");
    }
}

#[test]
fn an_element_that_is_not_a_col_in_cols_is_written_as_it_was() {
    let others = [
        r#"<other min="1" max="3" width="9"/>"#,
        r#"<other min="1" max="3" width="9"></other>"#,
        r#"<x:col xmlns:x="urn:x" min="1" max="3" width="9"/>"#,
        r#"<x:col xmlns:x="urn:x" min="1" max="3" width="9"></x:col>"#,
    ];
    for other in others {
        let xml = sheet(&format!("<cols>{other}</cols><sheetData/>"));
        let got = text(&xml, |edit| edit.set_column_width(1, 3, Some(1.0)).unwrap());
        let expected = sheet(&format!(
            r#"<cols>{other}<col min="1" max="3" width="1" customWidth="1"/></cols><sheetData/>"#
        ));
        assert_eq!(got, expected, "{other}");
    }
}

#[test]
fn an_element_that_is_not_a_row_in_sheetdata_is_written_as_it_was() {
    let others = [
        r#"<other r="1"/>"#,
        r#"<other r="1"></other>"#,
        r#"<x:row xmlns:x="urn:x" r="1"/>"#,
        r#"<x:row xmlns:x="urn:x" r="1"></x:row>"#,
    ];
    for other in others {
        let xml = sheet(&format!("<sheetData>{other}</sheetData>"));
        let got = text(&xml, |edit| edit.set_row_height(1, Some(2.0)).unwrap());
        let expected = sheet(&format!(
            r#"<sheetData>{other}<row r="1" ht="2" customHeight="1"/></sheetData>"#
        ));
        assert_eq!(got, expected, "{other}");
    }
}

#[test]
fn a_width_past_the_last_col_goes_at_the_end_of_the_last_of_several_cols() {
    let xml = sheet(
        r#"<cols><col min="1" max="1" width="9"/></cols><cols><col min="2" max="2" width="9"/></cols><sheetData/>"#,
    );
    let got = text(&xml, |edit| {
        edit.set_column_width(20, 20, Some(1.0)).unwrap()
    });
    let expected = sheet(
        r#"<cols><col min="1" max="1" width="9"/></cols><cols><col min="2" max="2" width="9"/><col min="20" max="20" width="1" customWidth="1"/></cols><sheetData/>"#,
    );
    assert_eq!(got, expected);
}
