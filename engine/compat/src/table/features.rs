//! The feature rows: parts and kinds of content a workbook may hold that
//! not every application has, each found in the package by a media type,
//! a relationship type or an element name.
//!
//! A cell no cited source settles is `unk()`, never a guess; a fallback is
//! written only where a source says what an older version shows. The
//! columns are excel, excel-mac, libreoffice, google-sheets, numbers,
//! onlyoffice.

use super::sources::{
    CELL_IMAGE, LO_TDF_159606, MS_DA_OLD, MS_DATA_MODEL, MS_NEW_2021, MS_QA_DATA_MODEL, MS_SLICER,
    MS_TC, MS_TC2, MS_TIMELINE, NUM_14_4, OO_CL, X14_SPARKLINE,
};
use super::{YES, between, le, no, on, s, unk};
use crate::app::{Excel as X, ExcelMac as M, LibreOffice as L, Numbers as N, OnlyOffice as O};
use crate::entry::Fallback::{AsNote, Degraded, LegacyArray, ValueError};
use crate::row::Row;

features! {
    /// A formula whose result spills over the cells next to it: `cm` on a
    /// cell, pointing at the dynamic array property in `xl/metadata.xml`.
    DynamicArray "dynamic_array" => s(
        on(X::V2021).fallback(LegacyArray), on(M::V2021).fallback(LegacyArray),
        unk().note("24.8 added the array functions; whether a result spills by itself is not known"),
        YES.note("arrays spill by a mechanism of Sheets' own; how it reads `cm` from an xlsx is not known"),
        on(N::V14_4), on(O::V9_3)),
        requires: [], sources: &[MS_DA_OLD, MS_NEW_2021, NUM_14_4, OO_CL];

    /// A threaded comment: `xl/threadedComments/` and `xl/persons/`, written
    /// beside a legacy note that older versions show.
    ThreadedComment "threaded_comment" => s(
        between(X::V2021, X::V365).fallback(AsNote)
            .note("retail Excel 2021 has it and LTSC 2021 does not; Excel 2024 is not known"),
        le(M::V365).fallback(AsNote), unk(), unk(), unk(), unk()),
        requires: [], sources: &[MS_NEW_2021, MS_TC, MS_TC2];

    /// A slicer on a PivotTable: `xl/slicers/` and `xl/slicerCaches/`.
    Slicer "slicer" => s(
        le(X::V2016), le(M::V2021), unk(), unk(), unk(), on(O::V8_0).note("since 6.2")),
        requires: [], sources: &[MS_SLICER, OO_CL];

    /// A slicer on a table: a slicer cache that holds a
    /// `x15:tableSlicerCache`.
    TableSlicer "table_slicer" => s(
        le(X::V2016), le(M::V2021), unk(), unk(), unk(), on(O::V8_0).note("since 6.0")),
        requires: [], sources: &[MS_SLICER, OO_CL];

    /// A timeline: `xl/timelines/` and `xl/timelineCaches/`.
    Timeline "timeline" => s(
        le(X::V2016), le(M::V2021),
        unk(), unk(), unk(), unk()),
        requires: [], sources: &[MS_TIMELINE];

    /// The Data Model (Power Pivot): `xl/model/item.data`.
    DataModel "data_model" => s(
        le(X::V2021),
        no().fallback(Degraded).note("a PivotTable on the Data Model cannot be changed"),
        unk(), unk(), unk(), unk()),
        requires: [], sources: &[MS_DATA_MODEL, MS_QA_DATA_MODEL];

    /// A sparkline: `x14:sparklineGroups` in a worksheet's `extLst`.
    Sparkline "sparkline" => s(
        on(X::V2010), unk(),
        on(L::V24_2).note("since 7.4; writing sparklines back to xlsx has a reported bug (tdf#159606)"),
        unk(), unk(), on(O::V8_0).note("shown and edited since 4.2, added since 6.4")),
        requires: [], sources: &[X14_SPARKLINE, LO_TDF_159606, OO_CL];

    /// A picture placed in a cell: `xl/richData/` and a cell's `vm`.
    CellImage "cell_image" => s(
        le(X::V365).fallback(ValueError), le(M::V365),
        unk(), unk(), unk(), unk()),
        requires: [], sources: &[CELL_IMAGE];
}
