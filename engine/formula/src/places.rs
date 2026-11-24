//! Where a formula is in a part: the elements whose text is one, and the
//! attributes whose value is one.
//!
//! A formula is the text of an element, found by local name in any
//! namespace, or the value of an unprefixed attribute of one, whatever the
//! part:
//!
//! - every place SpreadsheetML types a formula (`ST_Formula`,
//!   ECMA-376-1:2016, `sml.xsd`): the text of `f` (also a chart's `c:f` and
//!   an extension's `xm:f`), `definedName`, `calculatedColumnFormula` and
//!   `totalsRowFormula` (a table column's), `formula1` and `formula2` (a data
//!   validation's), `formula` and `oldFormula` (a conditional format's, a
//!   revision's), and the `macro` of an `objectPr` and the `macro`,
//!   `linkedCell` and `listFillRange` of a `controlPr` (an embedded object's
//!   and a control's);
//! - the places that hold a formula as a string: the `val` of a `cfvo` (a
//!   conditional format's value), the `formula` of a `cacheField` and of a
//!   `calculatedItem` (a pivot cache's), the `link` of an `oleObject` (a
//!   linked object's source), the `refersTo` of a `definedName` (an
//!   external workbook's), the `textlink` of an `sp` (a drawing's text box
//!   linked to cells), the `macro` of a drawing's `sp`, `cxnSp`, `pic` and
//!   `graphicFrame` (`dml-spreadsheetDrawing.xsd`), and the `cellRange` of
//!   a `cameraTool` (a linked picture's source).

/// The elements whose text is a formula (`ST_Formula`, ECMA-376-1:2016,
/// `sml.xsd`).
pub const FORMULA_ELEMENTS: [&str; 8] = [
    "f",
    "oldFormula",
    "definedName",
    "formula1",
    "formula2",
    "formula",
    "calculatedColumnFormula",
    "totalsRowFormula",
];

/// An attribute that holds a formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormulaAttribute {
    /// The local name of the element that has it.
    pub element: &'static str,
    /// The attribute's name.
    pub attribute: &'static str,
    /// Where the formula is, as a reader writes it (`element@attribute`).
    pub place: &'static str,
}

/// The attributes that hold a formula, in the order a reader reads those of
/// one element.
pub const FORMULA_ATTRIBUTES: [FormulaAttribute; 15] = [
    FormulaAttribute {
        element: "sp",
        attribute: "textlink",
        place: "sp@textlink",
    },
    FormulaAttribute {
        element: "sp",
        attribute: "macro",
        place: "sp@macro",
    },
    FormulaAttribute {
        element: "cxnSp",
        attribute: "macro",
        place: "cxnSp@macro",
    },
    FormulaAttribute {
        element: "pic",
        attribute: "macro",
        place: "pic@macro",
    },
    FormulaAttribute {
        element: "graphicFrame",
        attribute: "macro",
        place: "graphicFrame@macro",
    },
    FormulaAttribute {
        element: "objectPr",
        attribute: "macro",
        place: "objectPr@macro",
    },
    FormulaAttribute {
        element: "controlPr",
        attribute: "macro",
        place: "controlPr@macro",
    },
    FormulaAttribute {
        element: "controlPr",
        attribute: "linkedCell",
        place: "controlPr@linkedCell",
    },
    FormulaAttribute {
        element: "controlPr",
        attribute: "listFillRange",
        place: "controlPr@listFillRange",
    },
    FormulaAttribute {
        element: "cameraTool",
        attribute: "cellRange",
        place: "cameraTool@cellRange",
    },
    FormulaAttribute {
        element: "cfvo",
        attribute: "val",
        place: "cfvo@val",
    },
    FormulaAttribute {
        element: "cacheField",
        attribute: "formula",
        place: "cacheField@formula",
    },
    FormulaAttribute {
        element: "calculatedItem",
        attribute: "formula",
        place: "calculatedItem@formula",
    },
    FormulaAttribute {
        element: "oleObject",
        attribute: "link",
        place: "oleObject@link",
    },
    FormulaAttribute {
        element: "definedName",
        attribute: "refersTo",
        place: "definedName@refersTo",
    },
];
