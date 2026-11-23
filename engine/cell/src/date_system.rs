//! [`DateSystem`]: how a serial date-time counts days.

/// How a serial date-time counts days (§18.17.4.1), as Excel takes it
/// (MS-OI29500 on §18.2.28, note j): the `date1904` attribute of
/// `workbookPr` decides between 1900 and 1904, and a workbook whose
/// `conformance` attribute of `workbook` is `strict` counts 1900 as the
/// standard has it. `dateCompatibility`, which Excel ignores, is not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateSystem {
    /// Day 1 is 1900-01-01, and day 60 the 1900-02-29 that never was, as
    /// in Lotus 1-2-3: the default.
    Compatibility1900,
    /// Day 0 is 1899-12-30: `conformance="strict"`.
    Base1900,
    /// Day 0 is 1904-01-01: `date1904`.
    Base1904,
}
