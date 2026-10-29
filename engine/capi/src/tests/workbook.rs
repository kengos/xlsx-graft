//! Workbooks for the handles to read, built as a test asks: a zip of
//! stored entries, its central directory and its end record, holding a
//! package of a shared strings table of `a` and `b`, styles whose cell
//! formats 1 to 4 are a date, a time, a date-time and elapsed time, and the
//! sheets the test gives.
//!
//! The read crate's tests and the CLI's build zips with builders of their
//! own; theirs are test code this crate cannot link, so the few lines a
//! well-formed zip takes are written again here, as the CLI's.
//!
//! `tests/capi` and engine/napi include this file by its path: it names
//! nothing of this crate (`crate::`), so it builds in any of them.

use std::path::PathBuf;
use tempfile::TempDir;

/// The Transitional SpreadsheetML namespace.
const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// The prefix of the Transitional relationship types.
const TYPES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// The Transitional `worksheet` relationship type.
const WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
/// The Transitional `dialogsheet` relationship type.
const DIALOGSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet";
/// The macro sheet relationship type.
const MACROSHEET: &str = "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet";
/// The Transitional `chartsheet` relationship type.
const CHARTSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet";

/// A sheet: its name, its `state` (empty: none written), the type of its
/// relationship (none: its `r:id` names no relationship) and its part's
/// XML.
pub struct Sheet<'a> {
    pub name: &'a str,
    pub state: &'a str,
    pub rel_type: Option<&'a str>,
    pub xml: String,
}

/// A worksheet named `name` whose `sheetData` holds `rows`.
pub fn worksheet<'a>(name: &'a str, rows: &str) -> Sheet<'a> {
    let xml = format!(r#"<worksheet xmlns="{NS}"><sheetData>{rows}</sheetData></worksheet>"#);
    Sheet {
        name,
        state: "",
        rel_type: Some(WORKSHEET),
        xml,
    }
}

/// A chartsheet named `name`.
pub fn chartsheet(name: &str) -> Sheet<'_> {
    of_type(name, CHARTSHEET)
}

/// A dialog sheet named `name`.
pub fn dialogsheet(name: &str) -> Sheet<'_> {
    of_type(name, DIALOGSHEET)
}

/// A macro sheet named `name`.
pub fn macrosheet(name: &str) -> Sheet<'_> {
    of_type(name, MACROSHEET)
}

/// A sheet named `name` whose relationship is of `rel_type`, its part an
/// empty element the handles never read.
fn of_type<'a>(name: &'a str, rel_type: &'a str) -> Sheet<'a> {
    Sheet {
        name,
        state: "",
        rel_type: Some(rel_type),
        xml: format!(r#"<sheet xmlns="{NS}"/>"#),
    }
}

/// A sheet named `name` whose `r:id` names no relationship.
pub fn unrelated(name: &str) -> Sheet<'_> {
    Sheet {
        name,
        state: "",
        rel_type: None,
        xml: String::new(),
    }
}

/// The bytes of a workbook of `sheets`, in order, each at
/// `xl/sheets/sheet{n}.xml`.
pub fn xlsx(sheets: &[Sheet<'_>]) -> Vec<u8> {
    let rel = |id: &str, rel_type: &str, target: &str| {
        format!(r#"<Relationship Id="{id}" Type="{rel_type}" Target="{target}"/>"#)
    };
    let rels = |body: &str| {
        let ns = "http://schemas.openxmlformats.org/package/2006/relationships";
        format!(r#"<Relationships xmlns="{ns}">{body}</Relationships>"#)
    };
    let listed: String = (sheets.iter().zip(1..))
        .map(|(sheet, n)| {
            let (name, state) = (sheet.name, sheet.state);
            let state = if state.is_empty() {
                String::new()
            } else {
                format!(r#" state="{state}""#)
            };
            format!(r#"<sheet name="{name}" sheetId="{n}"{state} r:id="rId{n}"/>"#)
        })
        .collect();
    let workbook =
        format!(r#"<workbook xmlns="{NS}" xmlns:r="{TYPES}"><sheets>{listed}</sheets></workbook>"#);
    let mut related = rel("rIdS", &format!("{TYPES}/sharedStrings"), "sst.xml");
    related.push_str(&rel("rIdT", &format!("{TYPES}/styles"), "styles.xml"));
    for (sheet, n) in sheets.iter().zip(1..) {
        if let Some(rel_type) = sheet.rel_type {
            let target = format!("sheets/sheet{n}.xml");
            related.push_str(&rel(&format!("rId{n}"), rel_type, &target));
        }
    }
    let content_types = concat!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/>"#,
        "</Types>",
    );
    let sst = format!(r#"<sst xmlns="{NS}"><si><t>a</t></si><si><t>b</t></si></sst>"#);
    let formats: String = [0, 14, 21, 22, 46]
        .iter()
        .map(|id| format!(r#"<xf numFmtId="{id}"/>"#))
        .collect();
    let styles = format!(r#"<styleSheet xmlns="{NS}"><cellXfs>{formats}</cellXfs></styleSheet>"#);
    let main = rel(
        "rId1",
        &format!("{TYPES}/officeDocument"),
        "xl/workbook.xml",
    );
    let mut entries = vec![
        ("[Content_Types].xml".to_owned(), content_types.to_owned()),
        ("_rels/.rels".to_owned(), rels(&main)),
        ("xl/workbook.xml".to_owned(), workbook),
        ("xl/_rels/workbook.xml.rels".to_owned(), rels(&related)),
        ("xl/sst.xml".to_owned(), sst),
        ("xl/styles.xml".to_owned(), styles),
    ];
    for (sheet, n) in sheets.iter().zip(1..) {
        if sheet.rel_type.is_some() {
            entries.push((format!("xl/sheets/sheet{n}.xml"), sheet.xml.clone()));
        }
    }
    zip(&entries)
}

/// `bytes`, written to `book.xlsx` in a new directory, which is removed
/// when the returned handle drops.
pub fn write(bytes: &[u8]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book.xlsx");
    std::fs::write(&path, bytes).unwrap();
    (dir, path)
}

/// The zip of `entries`, each stored, in order.
fn zip(entries: &[(String, String)]) -> Vec<u8> {
    let mut zip = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in entries {
        let offset = u32::try_from(zip.len()).unwrap();
        let size = u32::try_from(data.len()).unwrap();
        let crc32 = crc32fast::hash(data.as_bytes());
        let name = name.as_bytes();
        let name_len = u16::try_from(name.len()).unwrap();
        zip.extend(0x0403_4b50_u32.to_le_bytes());
        // Version needed, flags, method (stored), time and date.
        zip.extend([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        zip.extend(crc32.to_le_bytes());
        zip.extend(size.to_le_bytes());
        zip.extend(size.to_le_bytes());
        zip.extend(name_len.to_le_bytes());
        zip.extend([0; 2]);
        zip.extend(name);
        zip.extend(data.as_bytes());

        directory.extend(0x0201_4b50_u32.to_le_bytes());
        // Version made by, version needed, flags, method, time and date.
        directory.extend([20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend(crc32.to_le_bytes());
        directory.extend(size.to_le_bytes());
        directory.extend(size.to_le_bytes());
        directory.extend(name_len.to_le_bytes());
        // Extra and comment lengths, disk, internal and external attributes.
        directory.extend([0; 12]);
        directory.extend(offset.to_le_bytes());
        directory.extend(name);
    }
    let start = u32::try_from(zip.len()).unwrap();
    let count = u16::try_from(entries.len()).unwrap().to_le_bytes();
    let length = u32::try_from(directory.len()).unwrap();
    zip.extend(&directory);
    zip.extend(0x0605_4b50_u32.to_le_bytes());
    zip.extend([0; 4]);
    zip.extend(count);
    zip.extend(count);
    zip.extend(length.to_le_bytes());
    zip.extend(start.to_le_bytes());
    zip.extend([0; 2]);
    zip
}
