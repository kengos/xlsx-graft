//! The parts of a `cells` workbook that hold no cell value: the content
//! types, the relationships, the workbook, the styles, and the start and end
//! of the worksheet. They are fixed text, except the workbook, which differs
//! by date system. The standard gives them as ECMA-376-2:2021 §7.2.3 content
//! types and §6.5 relationships, and ECMA-376-1:2016 §18.2 workbook, §18.8
//! styles and §18.3 worksheet.

use xlsx_graft_cell::DateSystem;

/// The XML declaration of `xl/workbook.xml`; the other parts repeat it as a
/// literal, since `concat!` takes only literals.
const DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

/// The `[Content_Types].xml` part: the content types of the workbook, the
/// worksheet, the styles and the shared strings.
pub(super) const CONTENT_TYPES: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
    "<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>",
    "<Default Extension=\"xml\" ContentType=\"application/xml\"/>",
    "<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>",
    "<Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>",
    "<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>",
    "<Override PartName=\"/xl/sharedStrings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml\"/>",
    "</Types>",
);

/// The `_rels/.rels` part: the package's one relationship, to the workbook.
pub(super) const PACKAGE_RELS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>",
    "</Relationships>",
);

/// The `xl/_rels/workbook.xml.rels` part: the worksheet, the styles and the
/// shared strings.
pub(super) const WORKBOOK_RELS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>",
    "<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
    "<Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings\" Target=\"sharedStrings.xml\"/>",
    "</Relationships>",
);

/// The `xl/styles.xml` part: the number formats 164 to 167 and, in
/// `cellXfs`, the cell format 0 (General) and 1 to 4, which use them.
pub(super) const STYLES: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
    "<numFmts count=\"4\">",
    "<numFmt numFmtId=\"164\" formatCode=\"yyyy\\-mm\\-dd\"/>",
    "<numFmt numFmtId=\"165\" formatCode=\"hh:mm:ss.000\"/>",
    "<numFmt numFmtId=\"166\" formatCode=\"yyyy\\-mm\\-dd\\ hh:mm:ss.000\"/>",
    "<numFmt numFmtId=\"167\" formatCode=\"[h]:mm:ss.000\"/>",
    "</numFmts>",
    "<fonts count=\"1\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts>",
    "<fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills>",
    "<borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders>",
    "<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>",
    "<cellXfs count=\"5\">",
    "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>",
    "<xf numFmtId=\"164\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>",
    "<xf numFmtId=\"165\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>",
    "<xf numFmtId=\"166\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>",
    "<xf numFmtId=\"167\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>",
    "</cellXfs>",
    "<cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles>",
    "</styleSheet>",
);

/// What the worksheet starts with, up to its first row: the two columns'
/// widths.
pub(super) const SHEET_START: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
    "<cols><col min=\"1\" max=\"1\" width=\"60\" customWidth=\"1\"/>",
    "<col min=\"2\" max=\"2\" width=\"40\" customWidth=\"1\"/></cols>",
    "<sheetData>",
);

/// What the worksheet ends with, after its last row.
pub(super) const SHEET_END: &str = "</sheetData></worksheet>";

/// The `xl/workbook.xml` part of a workbook in `system`: one worksheet,
/// `Cells`, and what gives the date system. `Compatibility1900`, the 1900
/// that counts 1900-02-29, which does not exist, is what a workbook with
/// neither `conformance="strict"` nor `date1904="1"` is in. A workbook in
/// `Base1900` has the `conformance="strict"` attribute on `workbook` (the
/// reader takes it as the standard's 1900, ECMA-376-1:2016 §18.2.27), while
/// the namespaces stay Transitional, to check whether Excel opens such a
/// workbook without a repair. A workbook in `Base1904` has a `workbookPr`
/// of `date1904="1"` (§18.2.28).
pub(super) fn workbook(system: DateSystem) -> String {
    let (conformance, properties) = match system {
        DateSystem::Base1900 => (" conformance=\"strict\"", ""),
        DateSystem::Base1904 => ("", "<workbookPr date1904=\"1\"/>"),
        _ => ("", ""),
    };
    format!(
        "{DECLARATION}<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"{conformance}>\
         {properties}<sheets><sheet name=\"Cells\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
    )
}
