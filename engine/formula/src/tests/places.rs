//! The tables of the places that hold a formula.

use crate::places::{FORMULA_ATTRIBUTES, FORMULA_ELEMENTS};

#[test]
fn the_elements_are_the_ones_sml_types_as_a_formula() {
    assert_eq!(
        FORMULA_ELEMENTS,
        [
            "f",
            "oldFormula",
            "definedName",
            "formula1",
            "formula2",
            "formula",
            "calculatedColumnFormula",
            "totalsRowFormula",
        ]
    );
}

#[test]
fn the_attributes_are_in_the_order_a_reader_reads_them() {
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
