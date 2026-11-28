//! The parts of the built-in blank workbook: fixed text for the content
//! types, the relationships, the workbook and the styles, and the worksheet
//! built with [`Defaults::sheet_format`](super::Defaults::sheet_format).
//! The standard gives them as ECMA-376-2:2021 §7.2.3 content types and §6.5
//! relationships, and ECMA-376-1:2016 §18.2 workbook, §18.8 styles and
//! §18.3 worksheet.

use super::{DefaultFont, Defaults};
use crate::WriteError;
use std::convert::Infallible;
use xlsx_graft_cell::{number_text, xml_attribute};

/// The `[Content_Types].xml` part: the content types of the workbook, the
/// worksheet and the styles.
const CONTENT_TYPES: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
    "<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>",
    "<Default Extension=\"xml\" ContentType=\"application/xml\"/>",
    "<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>",
    "<Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>",
    "<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>",
    "</Types>",
);

/// The `_rels/.rels` part: the package's one relationship, to the workbook.
const PACKAGE_RELS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>",
    "</Relationships>",
);

/// The `xl/workbook.xml` part: one worksheet, `Sheet1`.
const WORKBOOK: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" ",
    "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
    "<sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets>",
    "</workbook>",
);

/// The `xl/_rels/workbook.xml.rels` part: the worksheet and the styles.
const WORKBOOK_RELS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>",
    "<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
    "</Relationships>",
);

/// The `xl/styles.xml` part for `font`: that font, the two fills every
/// styles part has (`none` and `gray125`), one border with no line, the cell
/// style format and the cell format 0 (General) that use them, and the cell
/// style `Normal`. The font name is written the way `engine/style` writes
/// one: with XML attribute escaping only (`&`, `<`, `>`, `"`, tab, line feed
/// and carriage return), and no `_xHHHH_` escaping.
pub(super) fn styles(font: &DefaultFont) -> String {
    // The size is finite, so `number_text` returns `Ok`.
    let size = number_text(font.size()).unwrap_or_default();
    let mut name = String::new();
    let _: Result<(), Infallible> = xml_attribute(font.name(), |piece| {
        name.push_str(piece);
        Ok(())
    });
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
            "<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
            "<fonts count=\"1\"><font><sz val=\"{}\"/><name val=\"{}\"/></font></fonts>",
            "<fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills>",
            "<borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders>",
            "<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>",
            "<cellXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/></cellXfs>",
            "<cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles>",
            "</styleSheet>",
        ),
        size, name
    )
}

/// The worksheet part: no cell, and the `sheetFormatPr` of `defaults`.
fn sheet(defaults: &Defaults) -> Result<Vec<u8>, WriteError> {
    let mut out = Vec::new();
    out.extend_from_slice(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" ",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
        )
        .as_bytes(),
    );
    defaults.sheet_format().write(&mut out, "")?;
    out.extend_from_slice(b"<sheetData/></worksheet>");
    Ok(out)
}

/// The entries of the zip, each as its name and its content, with
/// `[Content_Types].xml` first.
pub(super) fn entries(defaults: &Defaults) -> Result<[(&'static str, Vec<u8>); 6], WriteError> {
    Ok([
        ("[Content_Types].xml", CONTENT_TYPES.as_bytes().to_vec()),
        ("_rels/.rels", PACKAGE_RELS.as_bytes().to_vec()),
        ("xl/workbook.xml", WORKBOOK.as_bytes().to_vec()),
        (
            "xl/_rels/workbook.xml.rels",
            WORKBOOK_RELS.as_bytes().to_vec(),
        ),
        ("xl/worksheets/sheet1.xml", sheet(defaults)?),
        ("xl/styles.xml", styles(defaults.font()).into_bytes()),
    ])
}
