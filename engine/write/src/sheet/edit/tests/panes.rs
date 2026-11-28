//! `freeze_panes`: the `sheetViews` written, the `pane` and `selection`s of
//! the first `sheetView` of `workbookViewId` 0 replaced or removed, and what
//! is refused.

use super::{STRICT_NS, run, sheet, text};
use crate::sheet::SheetError;

const NEW: &str = r#"<pane xSplit="1" ySplit="1" topLeftCell="B2" activePane="bottomRight" state="frozen"/><selection pane="bottomRight" activeCell="B2" sqref="B2"/>"#;

/// The sheet whose `sheetViews` hold `views`.
fn views(views: &str) -> String {
    sheet(&format!("<sheetViews>{views}</sheetViews><sheetData/>"))
}

#[test]
fn a_part_with_no_sheetviews_gets_one_before_the_sheetdata() {
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.freeze_panes(1, 1).unwrap()
    });
    let expected = views(&format!(
        r#"<sheetView workbookViewId="0">{NEW}</sheetView>"#
    ));
    assert_eq!(got, expected);
    let got = text(&sheet("<sheetFormatPr/><sheetData/>"), |edit| {
        edit.freeze_panes(2, 0).unwrap()
    });
    let expected = sheet(
        r#"<sheetViews><sheetView workbookViewId="0"><pane ySplit="2" topLeftCell="A3" activePane="bottomLeft" state="frozen"/><selection pane="bottomLeft" activeCell="A3" sqref="A3"/></sheetView></sheetViews><sheetFormatPr/><sheetData/>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn an_empty_sheetview_is_expanded_and_its_attributes_stay() {
    let xml = views(r#"<sheetView showGridLines="0" workbookViewId="0"/>"#);
    let got = text(&xml, |edit| edit.freeze_panes(1, 1).unwrap());
    let expected = views(&format!(
        r#"<sheetView showGridLines="0" workbookViewId="0">{NEW}</sheetView>"#
    ));
    assert_eq!(got, expected);
}

#[test]
fn the_pane_and_the_selections_the_sheetview_had_are_replaced() {
    let old = r#"<sheetView tabSelected="1" workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/><selection pane="bottomLeft" activeCell="A2" sqref="A2"/><selection activeCell="A1" sqref="A1"/><pivotSelection/><extLst><ext/></extLst></sheetView>"#;
    let got = text(&views(old), |edit| edit.freeze_panes(0, 2).unwrap());
    let expected = views(
        r#"<sheetView tabSelected="1" workbookViewId="0"><pane xSplit="2" topLeftCell="C1" activePane="topRight" state="frozen"/><selection pane="topRight" activeCell="C1" sqref="C1"/><pivotSelection/><extLst><ext/></extLst></sheetView>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn nothing_frozen_removes_the_pane_and_the_selections_that_name_a_pane() {
    let old = r#"<sheetView tabSelected="1" workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/><selection pane="bottomLeft" activeCell="A2" sqref="A2"/><selection activeCell="A1" sqref="A1"/><selection pane="topRight"/></sheetView>"#;
    let got = text(&views(old), |edit| edit.freeze_panes(0, 0).unwrap());
    let expected = views(
        r#"<sheetView tabSelected="1" workbookViewId="0"><selection activeCell="A1" sqref="A1"/></sheetView>"#,
    );
    assert_eq!(got, expected);
    let old = r#"<sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"></pane></sheetView>"#;
    let got = text(&views(old), |edit| edit.freeze_panes(0, 0).unwrap());
    assert_eq!(got, views(r#"<sheetView workbookViewId="0"></sheetView>"#));
}

#[test]
fn nothing_frozen_in_a_sheet_with_no_pane_changes_nothing() {
    for xml in [
        sheet("<sheetData/>"),
        views(r#"<sheetView workbookViewId="0"><selection pane="topRight"/></sheetView>"#),
        views(r#"<sheetView workbookViewId="1"><pane ySplit="1" state="frozen"/></sheetView>"#),
        views(r#"<sheetView workbookViewId="0"/>"#),
    ] {
        let got = run(&xml, |edit| edit.freeze_panes(0, 0).unwrap());
        assert!(got.holds(&xml), "{xml}");
    }
}

#[test]
fn a_sheetview_of_another_workbook_view_is_kept_and_one_of_view_0_is_added() {
    let xml = views(r#"<sheetView workbookViewId="1"><pane ySplit="1"/></sheetView>"#);
    let got = text(&xml, |edit| edit.freeze_panes(1, 1).unwrap());
    let expected = views(&format!(
        r#"<sheetView workbookViewId="1"><pane ySplit="1"/></sheetView><sheetView workbookViewId="0">{NEW}</sheetView>"#
    ));
    assert_eq!(got, expected);
    // Before the `extLst` of the `sheetViews`, in either form.
    for ext in ["<extLst><ext/></extLst>", "<extLst/>"] {
        let xml = views(&format!(r#"<sheetView workbookViewId="3"/>{ext}"#));
        let got = text(&xml, |edit| edit.freeze_panes(1, 1).unwrap());
        let expected = views(&format!(
            r#"<sheetView workbookViewId="3"/><sheetView workbookViewId="0">{NEW}</sheetView>{ext}"#
        ));
        assert_eq!(got, expected, "{ext}");
    }
}

#[test]
fn only_the_first_sheetview_of_view_0_is_changed() {
    let xml = views(
        r#"<sheetView workbookViewId=' 0 '><a><b/></a></sheetView><sheetView workbookViewId="0"><pane ySplit="1"/></sheetView>"#,
    );
    let got = text(&xml, |edit| edit.freeze_panes(1, 1).unwrap());
    let expected = views(&format!(
        r#"<sheetView workbookViewId=' 0 '>{NEW}<a><b/></a></sheetView><sheetView workbookViewId="0"><pane ySplit="1"/></sheetView>"#
    ));
    assert_eq!(got, expected);
}

#[test]
fn an_empty_sheetviews_is_filled_when_something_is_frozen() {
    let xml = sheet("<sheetViews/><sheetData/>");
    let got = text(&xml, |edit| edit.freeze_panes(1, 1).unwrap());
    let expected = views(&format!(
        r#"<sheetView workbookViewId="0">{NEW}</sheetView>"#
    ));
    assert_eq!(got, expected);
    let got = run(&xml, |edit| edit.freeze_panes(0, 0).unwrap());
    assert!(got.holds(&xml));
}

#[test]
fn the_other_children_of_sheetviews_and_the_text_between_are_kept() {
    let xml =
        views("\n <!--c-->\n <x><y/></x><sheetView workbookViewId=\"0\">\n text<q/></sheetView>\n");
    let got = text(&xml, |edit| edit.freeze_panes(1, 1).unwrap());
    let expected = views(&format!(
        "\n <!--c-->\n <x><y/></x><sheetView workbookViewId=\"0\">{NEW}\n text<q/></sheetView>\n"
    ));
    assert_eq!(got, expected);
}

#[test]
fn a_strict_worksheet_is_edited_too() {
    let xml = format!(r#"<worksheet xmlns="{STRICT_NS}"><sheetData/></worksheet>"#);
    let got = text(&xml, |edit| edit.freeze_panes(1, 0).unwrap());
    assert!(
        got.contains(r#"<sheetViews><sheetView workbookViewId="0"><pane ySplit="1""#),
        "{got}"
    );
}

#[test]
fn a_call_that_is_refused_changes_nothing() {
    let xml = sheet("<sheetData/>");
    let got = run(&xml, |edit| {
        let error = SheetError::FreezeRange {
            rows: 1_048_576,
            columns: 0,
        };
        assert_eq!(edit.freeze_panes(1_048_576, 0), Err(error));
        let error = SheetError::FreezeRange {
            rows: 0,
            columns: 16_384,
        };
        assert_eq!(edit.freeze_panes(0, 16_384), Err(error));
    });
    assert!(got.holds(&xml));
    let got = text(&xml, |edit| edit.freeze_panes(1_048_575, 16_383).unwrap());
    assert!(
        got.contains(r#"xSplit="16383" ySplit="1048575" topLeftCell="XFD1048576""#),
        "{got}"
    );
}
