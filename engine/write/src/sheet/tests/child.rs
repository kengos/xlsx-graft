//! `WorksheetChild`: the order of ECMA-376-1:2016 §18.3.1.99, the names,
//! and which children repeat.

use crate::sheet::WorksheetChild;

const NAMES: [&str; 39] = [
    "sheetPr",
    "dimension",
    "sheetViews",
    "sheetFormatPr",
    "cols",
    "sheetData",
    "sheetCalcPr",
    "sheetProtection",
    "protectedRanges",
    "scenarios",
    "autoFilter",
    "sortState",
    "dataConsolidate",
    "customSheetViews",
    "mergeCells",
    "phoneticPr",
    "conditionalFormatting",
    "dataValidations",
    "hyperlinks",
    "printOptions",
    "pageMargins",
    "pageSetup",
    "headerFooter",
    "rowBreaks",
    "colBreaks",
    "customProperties",
    "cellWatches",
    "ignoredErrors",
    "smartTags",
    "drawing",
    "legacyDrawing",
    "legacyDrawingHF",
    "drawingHF",
    "picture",
    "oleObjects",
    "controls",
    "webPublishItems",
    "tableParts",
    "extLst",
];

#[test]
fn all_lists_the_children_in_the_order_of_the_schema_and_of_ord() {
    let names: Vec<_> = WorksheetChild::ALL.iter().map(|c| c.local_name()).collect();
    assert_eq!(names, NAMES);
    assert!(WorksheetChild::ALL.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn a_name_reads_back_as_its_child_and_an_unknown_name_is_none() {
    for child in WorksheetChild::ALL {
        let name = child.local_name().as_bytes();
        assert_eq!(WorksheetChild::from_local_name(name), Some(child));
    }
    for unknown in [&b""[..], b"worksheet", b"SheetPr", b"sheetpr", b"cols "] {
        assert_eq!(WorksheetChild::from_local_name(unknown), None);
    }
}

#[test]
fn only_cols_and_conditional_formatting_repeat() {
    let repeating: Vec<_> = WorksheetChild::ALL
        .into_iter()
        .filter(|c| c.repeats())
        .collect();
    assert_eq!(
        repeating,
        [WorksheetChild::Cols, WorksheetChild::ConditionalFormatting]
    );
}
