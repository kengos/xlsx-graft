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
//!
//! Each row also says what a rename or a delete of a sheet does to the
//! formula there ([`OnSheetChange`], #23): the places a structure edit
//! rewrites, the ones it reports and leaves as they are, and the one that
//! names another workbook's sheets.

/// What a rename or a delete of a sheet (#23) does to a formula at a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnSheetChange {
    /// The sheet prefixes in the formula are rewritten, in the parts that
    /// `StructureEdit::apply` rewrites (its documentation lists them); in
    /// any other part the formula is treated as [`Report`](Self::Report).
    Rewrite,
    /// The formula is left as it is, and reported when a prefix in it names
    /// the renamed or deleted sheet: it is not in the closed list of places
    /// the edit rewrites.
    Report,
    /// The formula is neither rewritten nor reported: it names the sheets of
    /// another workbook.
    Ignore,
}

/// An element whose text is a formula (`ST_Formula`, ECMA-376-1:2016,
/// `sml.xsd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormulaElement {
    /// The element's local name.
    pub local: &'static str,
    /// What a rename or a delete of a sheet does to its text.
    pub on_sheet_change: OnSheetChange,
}

/// The elements whose text is a formula, each with what a sheet change does
/// to it. `oldFormula` is a revision's record of a formula that was.
pub const FORMULA_ELEMENTS: [FormulaElement; 8] = [
    FormulaElement {
        local: "f",
        on_sheet_change: OnSheetChange::Rewrite,
    },
    FormulaElement {
        local: "oldFormula",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaElement {
        local: "definedName",
        on_sheet_change: OnSheetChange::Rewrite,
    },
    FormulaElement {
        local: "formula1",
        on_sheet_change: OnSheetChange::Rewrite,
    },
    FormulaElement {
        local: "formula2",
        on_sheet_change: OnSheetChange::Rewrite,
    },
    FormulaElement {
        local: "formula",
        on_sheet_change: OnSheetChange::Rewrite,
    },
    FormulaElement {
        local: "calculatedColumnFormula",
        on_sheet_change: OnSheetChange::Rewrite,
    },
    FormulaElement {
        local: "totalsRowFormula",
        on_sheet_change: OnSheetChange::Rewrite,
    },
];

/// The row of [`FORMULA_ELEMENTS`] for the local name `local`, if it has one.
#[must_use]
pub fn formula_element(local: &str) -> Option<&'static FormulaElement> {
    FORMULA_ELEMENTS.iter().find(|row| row.local == local)
}

/// An attribute that holds a formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormulaAttribute {
    /// The local name of the element that has it.
    pub element: &'static str,
    /// The attribute's name.
    pub attribute: &'static str,
    /// Where the formula is, as verify's collector writes it in `at` and
    /// `KeptReference::place` does (`element@attribute`).
    pub place: &'static str,
    /// What a rename or a delete of a sheet does to its value.
    pub on_sheet_change: OnSheetChange,
}

/// The attributes that hold a formula, in the order verify's collector reads
/// those of one element.
pub const FORMULA_ATTRIBUTES: [FormulaAttribute; 15] = [
    FormulaAttribute {
        element: "sp",
        attribute: "textlink",
        place: "sp@textlink",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "sp",
        attribute: "macro",
        place: "sp@macro",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "cxnSp",
        attribute: "macro",
        place: "cxnSp@macro",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "pic",
        attribute: "macro",
        place: "pic@macro",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "graphicFrame",
        attribute: "macro",
        place: "graphicFrame@macro",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "objectPr",
        attribute: "macro",
        place: "objectPr@macro",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "controlPr",
        attribute: "macro",
        place: "controlPr@macro",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "controlPr",
        attribute: "linkedCell",
        place: "controlPr@linkedCell",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "controlPr",
        attribute: "listFillRange",
        place: "controlPr@listFillRange",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "cameraTool",
        attribute: "cellRange",
        place: "cameraTool@cellRange",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "cfvo",
        attribute: "val",
        place: "cfvo@val",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "cacheField",
        attribute: "formula",
        place: "cacheField@formula",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "calculatedItem",
        attribute: "formula",
        place: "calculatedItem@formula",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "oleObject",
        attribute: "link",
        place: "oleObject@link",
        on_sheet_change: OnSheetChange::Report,
    },
    FormulaAttribute {
        element: "definedName",
        attribute: "refersTo",
        place: "definedName@refersTo",
        on_sheet_change: OnSheetChange::Ignore,
    },
];
