//! [`Limits`]: the limits [`verify`](crate::verify) holds a workbook to,
//! each with the policy name a host sets it by.

use xlsx_graft_diagnostics::Limit;
use xlsx_graft_read::ReadLimits;

/// The limits [`verify`](crate::verify) holds a workbook to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The package reader's, whose XML limits also hold each XML part
    /// read.
    pub package: xlsx_graft_package::Limits,
    /// The most bytes of one formula kept, as UTF-8: a longer one is
    /// counted, not kept. Policy name `maxFormulaBytes`.
    pub max_formula_bytes: Limit,
    /// The most defined names of the workbook part read: past it, the
    /// workbook is refused, and the names past it are no names a formula
    /// is judged by. Policy name `maxDefinedNames`.
    pub max_defined_names: Limit,
    /// The most merged ranges of one worksheet kept to check: past it,
    /// the sheet is refused, and the ranges past it are not checked.
    /// Policy name `maxMergedCells`.
    pub max_merged_cells: Limit,
    /// The most cell formats (`xf` in `cellXfs`) the styles may hold: past
    /// it, the styles are refused, and no cell's format is checked against
    /// them. Policy name `maxCellFormats`.
    pub max_cell_formats: Limit,
    /// The most number formats (`numFmt` in `numFmts`) the styles may
    /// hold: past it, as past `max_cell_formats`. Policy name
    /// `maxNumberFormats`.
    pub max_number_formats: Limit,
}

impl Limits {
    /// The default `max_formula_bytes`, 64 KiB: Excel keeps a formula to
    /// 8,192 characters, at most 24 KiB as UTF-8, and under 56 KiB with
    /// every character written as an `_xHHHH_` escape.
    pub const DEFAULT_MAX_FORMULA_BYTES: u64 = 64 * 1024;

    /// The default `max_defined_names`, 65,536: each name kept costs a few
    /// hundred bytes beside its text.
    pub const DEFAULT_MAX_DEFINED_NAMES: u64 = 65_536;

    /// The default `max_merged_cells`, 262,144: each range kept costs some
    /// 100 bytes as overlaps are looked for, under 30 MiB at the most, and
    /// the columns they reach 1 MiB at the most.
    pub const DEFAULT_MAX_MERGED_CELLS: u64 = 262_144;
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            package: xlsx_graft_package::Limits::default(),
            max_formula_bytes: Limit::default_of(Self::DEFAULT_MAX_FORMULA_BYTES),
            max_defined_names: Limit::default_of(Self::DEFAULT_MAX_DEFINED_NAMES),
            max_merged_cells: Limit::default_of(Self::DEFAULT_MAX_MERGED_CELLS),
            max_cell_formats: Limit::default_of(ReadLimits::DEFAULT_MAX_CELL_FORMATS),
            max_number_formats: Limit::default_of(ReadLimits::DEFAULT_MAX_NUMBER_FORMATS),
        }
    }
}
