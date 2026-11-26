//! [`StructureError`]: why a name, a position or a formula a structure edit
//! was given cannot be written.

use thiserror::Error;

/// A name, a position or a formula the caller gave that the workbook cannot
/// hold: the caller's mistake, never a problem in the workbook.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StructureError {
    /// A sheet name Excel does not take.
    #[error("the sheet name {name:?} cannot be written: {reason:?}")]
    SheetNameInvalid {
        /// The name as the caller gave it.
        name: String,
        /// The first rule it breaks.
        reason: SheetNameFault,
    },
    /// A sheet name another sheet has, letter case aside.
    #[error("the sheet name {name:?} is the sheet {other:?}'s, letter case aside")]
    SheetNameTaken {
        /// The name as the caller gave it.
        name: String,
        /// The other sheet's name.
        other: String,
    },
    /// A defined name a formula cannot name, or that Excel does not take.
    #[error("the defined name {name:?} cannot be written: {reason:?}")]
    DefinedNameInvalid {
        /// The name as the caller gave it.
        name: String,
        /// The first rule it breaks.
        reason: DefinedNameFault,
    },
    /// A defined name a table has, letter case aside.
    #[error("the defined name {name:?} is the table {table:?}'s, letter case aside")]
    DefinedNameTable {
        /// The name as the caller gave it.
        name: String,
        /// The table's name.
        table: String,
    },
    /// No sheet has the name, letter case aside.
    #[error("the workbook has no sheet {name:?}")]
    SheetMissing {
        /// The name as the caller gave it.
        name: String,
    },
    /// A position past the end of the sheets: after the last one when a
    /// sheet is added, the last one's when a sheet is moved.
    #[error("the position {at} is past the end of the {sheets} sheets")]
    PositionOutOfRange {
        /// The position the caller gave.
        at: usize,
        /// How many sheets the workbook has.
        sheets: usize,
    },
    /// The only sheet of the workbook cannot be deleted.
    #[error("the sheet {name:?} is the only sheet of the workbook")]
    LastSheet {
        /// The sheet's name.
        name: String,
    },
    /// The workbook would have no visible sheet left.
    #[error("the sheet {name:?} is the only visible sheet of the workbook")]
    LastVisibleSheet {
        /// The sheet's name.
        name: String,
    },
    /// The sheet is hidden or very hidden, so it cannot be the active sheet.
    #[error("the sheet {name:?} is hidden, so it cannot be the active sheet")]
    SheetHidden {
        /// The sheet's name.
        name: String,
    },
    /// The largest `sheetId` of the workbook is `u32::MAX`, so a new sheet
    /// has no `sheetId` that is one past it.
    #[error("no sheetId is left for a new sheet")]
    SheetIdExhausted,
    /// No defined name of the scope has the name, letter case aside.
    #[error("the workbook has no defined name {name:?} in that scope")]
    DefinedNameMissing {
        /// The name as the caller gave it.
        name: String,
    },
    /// The workbook already holds as many defined names as it can.
    #[error("the workbook cannot hold more than 65,536 defined names")]
    DefinedNamesTooMany,
    /// A formula the formula reader cannot read whole.
    #[error("the formula {formula:?} cannot be read")]
    FormulaMalformed {
        /// The formula as the caller gave it.
        formula: String,
    },
    /// A formula has a sheet prefix of this workbook (no external book)
    /// whose sheet the workbook does not have.
    #[error("the formula {formula:?} names the sheet {sheet:?}, which the workbook does not have")]
    FormulaSheetMissing {
        /// The formula as the caller gave it.
        formula: String,
        /// The sheet's name, as the formula writes it.
        sheet: String,
    },
}

/// The rule a sheet name breaks, the first of these in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetNameFault {
    /// It is empty.
    Empty,
    /// It is longer than 31 UTF-16 code units.
    TooLong,
    /// It holds one of `[ ] : * ? / \`.
    Character,
    /// It starts or ends with `'`.
    Apostrophe,
    /// It holds a C0 control character (U+0000 to U+001F, tab, LF and CR
    /// included), or a character XML 1.0 cannot hold.
    Control,
    /// It is `History`, letter case aside.
    Reserved,
}

/// The rule a defined name breaks, the first of these in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinedNameFault {
    /// It is not a name ECMA-376-1:2016 §18.17.2.5 and MS-OI29500 allow, or
    /// it holds `?`, or it is `\` with at most one character after it.
    Character,
    /// It holds a character XML 1.0 cannot hold.
    Xml,
    /// It is longer than 255 UTF-16 code units.
    TooLong,
    /// It is of the form of a cell reference: `A1`, `R1C1`, `R`, `C`.
    Reference,
    /// It is `TRUE` or `FALSE`, letter case aside.
    Boolean,
    /// It starts with `_xlnm.` and is not one of the seven built-in names the
    /// writer writes (`_xlnm.Database` is not), or starts with a prefix the
    /// reader takes off a word before reading it as a function or a LAMBDA
    /// parameter (`_xlfn.`, `_xlws.`, `_xludf.`, `_xleta.`, `_xlpm.`); ASCII
    /// letter case aside.
    ReservedPrefix,
    /// A built-in name scoped to the workbook; a built-in name is scoped to a sheet.
    BuiltinScope,
}
