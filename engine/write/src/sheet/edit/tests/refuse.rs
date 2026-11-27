//! A part with an element of an edited kind out of the schema's order, or
//! wrapped in `mc:AlternateContent`, is refused by `finish` (no zip, an
//! `xml_schema_invalid` at the part); a kind no call edits passes through.

use super::wrapped::MC;
use super::{SHEET, r, refusal, sheet, text};
use crate::sheet::{SheetEdit, SheetProtection};

/// What `refusal` says for a refusal at the first `at` of `xml` naming `name`.
fn expected(xml: &str, at: &str, name: &str) -> String {
    let offset = xml.find(at).unwrap();
    format!(
        "xml_schema_invalid the XML breaks its schema at byte {offset} (unexpected_element: {name}) {SHEET}"
    )
}

/// The body of a sheet, the start of the element, its name and the call.
type Late = (&'static str, &'static str, &'static str, fn(&mut SheetEdit));

/// The kind's name, the element and the call.
type Wrapped = (&'static str, &'static str, fn(&mut SheetEdit));

/// As [`Late`], with the body as text.
type Repeated = (String, &'static str, &'static str, fn(&mut SheetEdit));

fn merge(edit: &mut SheetEdit) {
    edit.merge(r("A1:B2")).unwrap();
}

fn protect(edit: &mut SheetEdit) {
    edit.protect(SheetProtection::default());
}

fn width(edit: &mut SheetEdit) {
    edit.set_column_width(1, 1, Some(2.0)).unwrap();
}

fn height(edit: &mut SheetEdit) {
    edit.set_row_height(1, Some(2.0)).unwrap();
}

fn freeze(edit: &mut SheetEdit) {
    edit.freeze_panes(1, 0).unwrap();
}

#[test]
fn an_edited_element_after_its_place_is_refused() {
    let cases: [Late; 5] = [
        (
            r#"<sheetData/><pageMargins/><mergeCells count="1"><mergeCell ref="D1:E2"/></mergeCells>"#,
            "<mergeCells",
            "mergeCells",
            merge,
        ),
        (
            r#"<sheetData/><pageMargins/><sheetProtection sheet="1"/>"#,
            "<sheetProtection",
            "sheetProtection",
            protect,
        ),
        (
            r#"<sheetData/><cols><col min="1" max="1" width="2"/></cols>"#,
            "<cols",
            "cols",
            width,
        ),
        (
            r#"<pageMargins/><sheetData/>"#,
            "<sheetData",
            "sheetData",
            height,
        ),
        (
            r#"<sheetData/><sheetViews><sheetView workbookViewId="0"/></sheetViews>"#,
            "<sheetViews",
            "sheetViews",
            freeze,
        ),
    ];
    for (body, at, name, call) in cases {
        let xml = sheet(body);
        assert_eq!(refusal(&xml, call), expected(&xml, at, name), "{name}");
    }
}

#[test]
fn an_edited_element_wrapped_in_alternate_content_is_refused() {
    let cases: [Wrapped; 3] = [
        (
            "mergeCells",
            r#"<mergeCells count="1"><mergeCell ref="D1:E2"/></mergeCells>"#,
            merge,
        ),
        (
            "sheetProtection",
            r#"<sheetProtection sheet="1"/>"#,
            protect,
        ),
        (
            "cols",
            r#"<cols><col min="1" max="1" width="2"/></cols>"#,
            width,
        ),
    ];
    for (name, wrapped, call) in cases {
        let wrapper = format!(
            r#"<mc:AlternateContent {MC}><mc:Choice Requires="x">{wrapped}</mc:Choice></mc:AlternateContent>"#
        );
        let xml = sheet(&format!("<sheetData/>{wrapper}"));
        assert_eq!(
            refusal(&xml, call),
            expected(&xml, "<mc:AlternateContent", name),
            "{name}"
        );
    }
}

#[test]
fn a_kind_no_call_edits_passes_through_out_of_order_or_wrapped() {
    let late =
        r#"<sheetData/><pageMargins/><mergeCells count="1"><mergeCell ref="D1:E2"/></mergeCells>"#;
    let wrapper = format!(
        r#"<mc:AlternateContent {MC}><mc:Choice Requires="x"><mergeCells count="1"><mergeCell ref="D1:E2"/></mergeCells></mc:Choice></mc:AlternateContent>"#
    );
    for body in [late.to_owned(), format!("<sheetData/>{wrapper}")] {
        let got = text(&sheet(&body), width);
        let expected = sheet(&format!(
            r#"<cols><col min="1" max="1" width="2" customWidth="1"/></cols>{body}"#
        ));
        assert_eq!(got, expected);
    }
}

#[test]
fn finish_writes_no_zip_and_reports_the_diagnostic_at_the_sheet_part() {
    let xml = sheet(
        r#"<sheetData/><pageMargins/><mergeCells count="1"><mergeCell ref="D1:E2"/></mergeCells>"#,
    );
    let got = refusal(&xml, merge);
    assert!(got.starts_with("xml_schema_invalid "), "{got}");
    assert!(got.ends_with(&format!(" {SHEET}")), "{got}");
}

#[test]
fn a_second_element_of_an_edited_kind_that_does_not_repeat_is_refused() {
    let views = r#"<sheetViews><sheetView workbookViewId="0"/></sheetViews>"#;
    let merges = r#"<mergeCells count="1"><mergeCell ref="D1:E2"/></mergeCells>"#;
    let cases: [Repeated; 4] = [
        (
            format!("{views}{views}<sheetData/>"),
            "<sheetViews",
            "sheetViews",
            freeze,
        ),
        (
            "<sheetData/><sheetData/>".to_owned(),
            "<sheetData",
            "sheetData",
            height,
        ),
        (
            format!("<sheetData/>{merges}{merges}"),
            "<mergeCells",
            "mergeCells",
            merge,
        ),
        (
            r#"<sheetData/><sheetProtection sheet="1"/><sheetProtection sheet="1"/>"#.to_owned(),
            "<sheetProtection",
            "sheetProtection",
            protect,
        ),
    ];
    for (body, at, name, call) in cases {
        let xml = sheet(&body);
        // The second element of the kind, not the first.
        let offset = xml.rfind(at).unwrap();
        let expected = format!(
            "xml_schema_invalid the XML breaks its schema at byte {offset} (unexpected_element: {name}) {SHEET}"
        );
        assert_eq!(refusal(&xml, call), expected, "{name}");
    }
}

#[test]
fn a_second_element_of_a_kind_no_call_edits_or_one_that_repeats_passes_through() {
    let merges = r#"<mergeCells count="1"><mergeCell ref="D1:E2"/></mergeCells>"#;
    let xml = sheet(&format!("<sheetData/>{merges}{merges}"));
    let got = text(&xml, width);
    let expected = sheet(&format!(
        r#"<cols><col min="1" max="1" width="2" customWidth="1"/></cols><sheetData/>{merges}{merges}"#
    ));
    assert_eq!(got, expected);
    let cols = r#"<cols><col min="1" max="1" width="9"/></cols>"#;
    let got = text(&sheet(&format!("{cols}{cols}<sheetData/>")), width);
    assert!(
        got.contains(r#"<col min="1" max="1" width="2" customWidth="1"/>"#),
        "{got}"
    );
}
