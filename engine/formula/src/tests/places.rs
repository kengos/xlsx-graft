//! The tables of the places that hold a formula.

use crate::places::{FORMULA_ATTRIBUTES, FORMULA_ELEMENTS, OnSheetChange, formula_element};

#[test]
fn the_elements_are_the_ones_sml_types_as_a_formula_and_say_what_a_sheet_change_does() {
    use OnSheetChange::{Report, Rewrite};
    let rows: Vec<(&str, OnSheetChange)> = FORMULA_ELEMENTS
        .iter()
        .map(|row| (row.local, row.on_sheet_change))
        .collect();
    assert_eq!(
        rows,
        [
            ("f", Rewrite),
            ("oldFormula", Report),
            ("definedName", Rewrite),
            ("formula1", Rewrite),
            ("formula2", Rewrite),
            ("formula", Rewrite),
            ("calculatedColumnFormula", Rewrite),
            ("totalsRowFormula", Rewrite),
        ]
    );
}

#[test]
fn a_formula_element_is_found_by_its_local_name() {
    assert_eq!(
        formula_element("oldFormula").map(|row| row.on_sheet_change),
        Some(OnSheetChange::Report)
    );
    assert_eq!(formula_element("F"), None);
    assert_eq!(formula_element("c"), None);
}

#[test]
fn the_attributes_are_in_the_order_verifys_collector_reads_them() {
    let rows: Vec<(&str, &str, &str)> = FORMULA_ATTRIBUTES
        .iter()
        .map(|row| (row.element, row.attribute, row.place))
        .collect();
    assert_eq!(
        rows,
        [
            ("sp", "textlink", "sp@textlink"),
            ("sp", "macro", "sp@macro"),
            ("cxnSp", "macro", "cxnSp@macro"),
            ("pic", "macro", "pic@macro"),
            ("graphicFrame", "macro", "graphicFrame@macro"),
            ("objectPr", "macro", "objectPr@macro"),
            ("controlPr", "macro", "controlPr@macro"),
            ("controlPr", "linkedCell", "controlPr@linkedCell"),
            ("controlPr", "listFillRange", "controlPr@listFillRange"),
            ("cameraTool", "cellRange", "cameraTool@cellRange"),
            ("cfvo", "val", "cfvo@val"),
            ("cacheField", "formula", "cacheField@formula"),
            ("calculatedItem", "formula", "calculatedItem@formula"),
            ("oleObject", "link", "oleObject@link"),
            ("definedName", "refersTo", "definedName@refersTo"),
        ]
    );
}

#[test]
fn only_the_external_workbooks_names_are_ignored_by_a_sheet_change() {
    let ignored: Vec<&str> = (FORMULA_ATTRIBUTES.iter())
        .filter(|row| row.on_sheet_change == OnSheetChange::Ignore)
        .map(|row| row.place)
        .collect();
    let reported = (FORMULA_ATTRIBUTES.iter())
        .filter(|row| row.on_sheet_change == OnSheetChange::Report)
        .count();
    assert_eq!(ignored, ["definedName@refersTo"]);
    assert_eq!(reported, 14);
}
