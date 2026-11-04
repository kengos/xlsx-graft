//! The sources the table's rows cite, one constant per page.

/// Excel's functions by category: the version each function came in.
pub(super) const MS_FN: &str = "https://support.microsoft.com/en-us/office/excel-functions-by-category-5f91f4e9-7b42-46d2-9bd1-63f26a86c0eb";
/// [MS-XLSX] Functions: the functions Excel saves with an `_xlfn.` prefix.
pub(super) const MS_XLSX_FN: &str = "https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/5d1b6d44-6fc1-4ecd-8fef-0b27406cc2bf";
/// An `_xlfn.` prefix is displayed in front of a formula: what an Excel
/// that lacks a function shows.
pub(super) const MS_XLFN: &str = "https://support.microsoft.com/office/issue-an-xlfn-prefix-is-displayed-in-front-of-a-formula-882f1ef7-68fb-4fcd-8d54-9fbb77fd5025";
/// Dynamic array formulas in non-dynamic aware Excel.
pub(super) const MS_DA_OLD: &str = "https://support.microsoft.com/en-us/office/dynamic-array-formulas-in-non-dynamic-aware-excel-696e164e-306b-4282-ae9d-aa88f5502fa2";
/// What's new in Excel 2021 for Windows.
pub(super) const MS_NEW_2021: &str =
    "https://support.microsoft.com/en-us/Excel/what-s-new-in-excel-2021-for-windows";
/// What's new in Excel 2024 for Windows and Mac.
pub(super) const MS_NEW_2024: &str =
    "https://support.microsoft.com/topic/faee26b6-ad74-40a8-9304-aa6db716553f";
/// A threaded comment opened in a version that cannot edit it.
pub(super) const MS_TC: &str = "https://support.microsoft.com/en-au/office/this-threaded-comment-can-be-viewed-but-not-edited-17b3eb02-efa7-4934-9e8e-b8916482bf41";
/// The difference between threaded comments and notes.
pub(super) const MS_TC2: &str = "https://support.microsoft.com/office/the-difference-between-threaded-comments-and-notes-75a51eec-4092-42ab-abf8-7669077b7be3";
/// Create a PivotTable timeline: its Applies To starts at Excel 2016 and
/// Excel 2021 for Mac.
pub(super) const MS_TIMELINE: &str =
    "https://support.microsoft.com/en-US/excel/create-a-pivottable-timeline-to-filter-dates";
/// Use slicers to filter data: its Applies To starts at Excel 2021, and it
/// names the ribbon of Excel 2016 and older.
pub(super) const MS_SLICER: &str = "https://support.microsoft.com/en-us/office/use-slicers-to-filter-data-249f966b-a9d5-4b0f-b31a-12651785d29d";
/// Create a Data Model in Excel: its Applies To starts at Excel 2021.
pub(super) const MS_DATA_MODEL: &str = "https://support.microsoft.com/en-us/office/create-a-data-model-in-excel-87e7a54c-87dc-488e-9410-5c75dbcb0f7b";
/// The SWITCH function: Office 2019 or Microsoft 365.
pub(super) const MS_SWITCH: &str = "https://support.microsoft.com/en-us/office/switch-function-47ab33c0-28ce-4530-8a45-d532ec4aa25e";
/// The GROUPBY function: Excel for Microsoft 365.
pub(super) const MS_GROUPBY: &str = "https://support.microsoft.com/en-us/office/groupby-function-5e08ae8c-6800-4b72-b623-c41773611505";
/// The REGEXEXTRACT function: Microsoft 365 and Microsoft 365 for Mac.
pub(super) const MS_REGEXEXTRACT: &str = "https://support.microsoft.com/en-us/office/regexextract-function-4b96c140-9205-4b6e-9fbe-6aa9e783ff57";
/// The AGGREGATE function: its Applies To starts at Excel 2016.
pub(super) const MS_AGGREGATE: &str = "https://support.microsoft.com/en-us/office/aggregate-function-43b9278e-6aa7-4f17-92b6-e19993fa26df";
/// The ENCODEURL function: not in Excel for Mac.
pub(super) const MS_ENCODEURL: &str = "https://support.microsoft.com/en-us/office/encodeurl-function-07c7fb90-7c60-4bff-8687-fac50fe33d0e";
/// The FILTERXML function: not in Excel for Mac.
pub(super) const MS_FILTERXML: &str = "https://support.microsoft.com/en-us/office/filterxml-function-4df72efc-11ec-4951-86f5-c1374812f5b7";
/// The WEBSERVICE function: no results on a Mac.
pub(super) const MS_WEBSERVICE: &str = "https://support.microsoft.com/en-us/office/webservice-function-0546a35a-ecc6-4739-aed7-c0b7ce1562c4";
/// The IMAGE function: Excel 2024, 2024 for Mac and Microsoft 365.
pub(super) const MS_IMAGE: &str = "https://support.microsoft.com/en-us/office/image-function-7e112975-5e52-4f2a-b9da-1d913d51f5d5";
/// Data Model features are not supported in Excel for Mac (Microsoft Q&A).
pub(super) const MS_QA_DATA_MODEL: &str = "https://learn.microsoft.com/en-us/answers/questions/5445256/excel-on-mac-data-model-features-are-not-supported";
/// LibreOffice Calc's function help: each page says the release a function
/// came in.
pub(super) const LO_HELP: &str =
    "https://help.libreoffice.org/latest/en-US/text/scalc/01/04060100.html";
/// What's new in LibreOffice 25.8: the array and text functions it added.
pub(super) const LO_25_8: &str = "https://whatsnew.libreoffice.org/25.8/";
/// tdf#138749: LAMBDA, still a request.
pub(super) const LO_TDF_138749: &str =
    "https://www.libreoffice.org/bugzilla/show_bug.cgi?id=138749";
/// tdf#159606: sparklines exported to xlsx.
pub(super) const LO_TDF_159606: &str =
    "https://www.libreoffice.org/bugzilla/show_bug.cgi?id=159606";
/// Google Sheets' function list.
pub(super) const GS_LIST: &str = "https://support.google.com/docs/table/25273";
/// Google Sheets' XMATCH, which has a page but is missing from the list.
pub(super) const GS_XMATCH: &str = "https://support.google.com/docs/answer/12406049";
/// What's new in Numbers for iCloud: XLOOKUP, XMATCH and REGEX (2020).
pub(super) const NUM_ICLOUD: &str = "https://support.apple.com/HT202998";
/// What's new in Numbers 14.4 for Mac: LET, LAMBDA, FILTER, SORT, UNIQUE
/// and spilling arrays.
pub(super) const NUM_14_4: &str =
    "https://support.apple.com/en-us/guide/numbers/tan700f60676/14.4/mac/1.0";
/// Numbers' help for XLOOKUP.
pub(super) const NUM_XLOOKUP: &str =
    "https://support.apple.com/guide/functions/xlookup-ffa6df8dc1ac/web";
/// Numbers' help for XMATCH.
pub(super) const NUM_XMATCH: &str = "https://support.apple.com/guide/functions/ffa5e40ce687/web";
/// Numbers' features page, which lists TEXTJOIN, CONCAT and SWITCH.
pub(super) const NUM_FEATURES: &str = "https://www.apple.com/numbers/features/";
/// ONLYOFFICE Docs' changelog: the functions and features each release
/// added.
pub(super) const OO_CL: &str =
    "https://github.com/ONLYOFFICE/DocumentServer/blob/master/CHANGELOG.md";
/// ONLYOFFICE's formula engine: the functions it implements.
pub(super) const OO_SRC: &str =
    "https://github.com/ONLYOFFICE/sdkjs/tree/master/cell/model/FormulaObjects";
/// Sparklines in the x14 extension (a third party's description).
pub(super) const X14_SPARKLINE: &str =
    "https://blog.loslab.com/en-us/hotxls-component/hotxls-sparklines-x14-extension-delphi.html";
/// Images placed in cells (a third party's description).
pub(super) const CELL_IMAGE: &str =
    "https://xlsxwriter.readthedocs.io/example_embedded_images.html";
