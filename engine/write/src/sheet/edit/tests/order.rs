//! The order of the root's children, the prefix of the root, and what is
//! written as it was: the declaration, the byte order mark, the white space
//! and the elements `SheetEdit` does not know.

use super::{NS, run, sheet, text};
use crate::sheet::SheetProtection;

const BAR: &str = r#"<foo:bar xmlns:foo="urn:foo"><baz/></foo:bar>"#;

/// Makes one call of each kind on the edit.
fn all(edit: &mut crate::sheet::SheetEdit) {
    edit.merge(super::r("A1:B2")).unwrap();
    edit.set_column_width(2, 3, Some(9.0)).unwrap();
    edit.set_row_height(1, Some(9.0)).unwrap();
    edit.freeze_panes(1, 0).unwrap();
    edit.protect(SheetProtection::default());
}

const VIEWS: &str = r#"<sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/><selection pane="bottomLeft" activeCell="A2" sqref="A2"/></sheetView></sheetViews>"#;
const COLS: &str = r#"<cols><col min="2" max="3" width="9" customWidth="1"/></cols>"#;
const PROTECTION: &str = r#"<sheetProtection sheet="1" objects="1" scenarios="1"/>"#;
const MERGES: &str = r#"<mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells>"#;

#[test]
fn each_kind_is_written_where_the_schema_puts_it() {
    let xml = sheet(
        r#"<sheetPr/><dimension ref="A1"/><sheetFormatPr/><sheetData><row r="3"/></sheetData><pageMargins/>"#,
    );
    let got = text(&xml, all);
    let expected = sheet(&format!(
        r#"<sheetPr/><dimension ref="A1"/>{VIEWS}<sheetFormatPr/>{COLS}<sheetData><row r="1" ht="9" customHeight="1"/><row r="3"/></sheetData>{PROTECTION}{MERGES}<pageMargins/>"#
    ));
    assert_eq!(got, expected);
}

#[test]
fn the_elements_written_new_take_the_prefix_of_the_root() {
    let xml = format!(
        r#"<x:worksheet xmlns:x="{NS}"><x:sheetData><x:row r="2"/></x:sheetData><x:pageMargins/></x:worksheet>"#
    );
    let got = text(&xml, all);
    let expected = format!(
        r#"<x:worksheet xmlns:x="{NS}"><x:sheetViews><x:sheetView workbookViewId="0"><x:pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/><x:selection pane="bottomLeft" activeCell="A2" sqref="A2"/></x:sheetView></x:sheetViews><x:cols><x:col min="2" max="3" width="9" customWidth="1"/></x:cols><x:sheetData><x:row r="1" ht="9" customHeight="1"/><x:row r="2"/></x:sheetData><x:sheetProtection sheet="1" objects="1" scenarios="1"/><x:mergeCells count="1"><x:mergeCell ref="A1:B2"/></x:mergeCells><x:pageMargins/></x:worksheet>"#
    );
    assert_eq!(got, expected);
}

#[test]
fn the_elements_written_inside_one_the_part_has_take_the_prefix_too() {
    let xml = format!(
        r#"<x:worksheet xmlns:x="{NS}"><x:sheetViews><x:sheetView workbookViewId="0"/></x:sheetViews><x:cols><x:col min="1" max="1" width="1"/></x:cols><x:sheetData/><x:mergeCells count="1"><x:mergeCell ref="D1:E2"/></x:mergeCells></x:worksheet>"#
    );
    let got = text(&xml, all);
    let expected = format!(
        r#"<x:worksheet xmlns:x="{NS}"><x:sheetViews><x:sheetView workbookViewId="0"><x:pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/><x:selection pane="bottomLeft" activeCell="A2" sqref="A2"/></x:sheetView></x:sheetViews><x:cols><x:col min="1" max="1" width="1"/><x:col min="2" max="3" width="9" customWidth="1"/></x:cols><x:sheetData><x:row r="1" ht="9" customHeight="1"/></x:sheetData><x:sheetProtection sheet="1" objects="1" scenarios="1"/><x:mergeCells count="2"><x:mergeCell ref="D1:E2"/><x:mergeCell ref="A1:B2"/></x:mergeCells></x:worksheet>"#
    );
    assert_eq!(got, expected);
}

#[test]
fn an_empty_root_gets_every_element() {
    let got = text(&format!(r#"<worksheet xmlns="{NS}"/>"#), all);
    let expected = sheet(&format!(
        r#"{VIEWS}{COLS}<sheetData><row r="1" ht="9" customHeight="1"/></sheetData>{PROTECTION}{MERGES}"#
    ));
    assert_eq!(got, expected);
    let got = text(&format!(r#"<x:worksheet xmlns:x="{NS}"/>"#), |edit| {
        edit.merge(super::r("A1:B2")).unwrap();
    });
    let expected = format!(
        r#"<x:worksheet xmlns:x="{NS}"><x:mergeCells count="1"><x:mergeCell ref="A1:B2"/></x:mergeCells></x:worksheet>"#
    );
    assert_eq!(got, expected);
}

#[test]
fn what_the_edit_does_not_change_is_written_as_it_was() {
    let xml = format!(
        "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n<worksheet xmlns=\"{NS}\" xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:x14ac=\"urn:x14ac\" mc:Ignorable=\"x14ac\">\r\n  <!-- c -->\r\n  <?pi data?>\r\n  <sheetData/>\r\n  {BAR}\r\n  <extLst><ext/></extLst>\r\n</worksheet>\r\n"
    );
    let got = text(&xml, |edit| edit.merge(super::r("A1:B2")).unwrap());
    let expected = xml.replace("<extLst>", &format!("{MERGES}<extLst>"));
    assert_eq!(got, expected);
}

#[test]
fn a_kind_the_part_lacks_goes_after_an_element_that_is_not_in_the_table() {
    let xml = sheet(&format!("<sheetData/>{BAR}"));
    let got = text(&xml, |edit| edit.merge(super::r("A1:B2")).unwrap());
    assert_eq!(got, sheet(&format!("<sheetData/>{BAR}{MERGES}")));
}

#[test]
fn no_call_leaves_the_part_in_the_package_as_it_was() {
    let xml = sheet("<sheetData/>");
    assert!(run(&xml, |_| {}).holds(&xml));
}
