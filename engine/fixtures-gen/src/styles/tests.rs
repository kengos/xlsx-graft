//! `styles` over the real `fixtures/` and over small workbooks: each result
//! read back, and the workbooks and folders it refuses.

use crate::{FixturesGenError, run};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits as PackageLimits, Package};
use xlsx_graft_read::{Book, FormatKind, ReadLimits, Row, Styles, Value, Workbook};
use xlsx_graft_spool::{MemorySpools, TempFiles};
use xlsx_graft_write::{DeflateWriter, ZipWriter};

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// The repository's `fixtures/` folder.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn styles(from: &Path, to: &Path, out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let args: Vec<OsString> = vec!["styles".into(), from.into(), to.into()];
    run(&args, out)
}

/// The `.xlsx` and `.xlsm` files of `fixtures/`, sorted.
fn workbooks() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("xlsx" | "xlsm")
            )
        })
        .collect();
    paths.sort();
    paths
}

/// The kind of each cell format of the workbook at `path`.
fn kinds(path: &Path) -> Vec<FormatKind> {
    let mut diagnostics = Diagnostics::new();
    let file = std::fs::File::open(path).unwrap();
    let mut package = Package::open(file, &PackageLimits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let book = Workbook::find(&package, &mut diagnostics).unwrap();
    let limits = ReadLimits::default();
    match Styles::read(&mut package, &book, &limits, &mut diagnostics).unwrap() {
        Styles::Formats(kinds) => kinds,
        other => panic!("{other:?}"),
    }
}

/// The text a value is shown as, or its type.
fn shown(value: &Value<'_>) -> String {
    match value {
        Value::String(text) => (*text).to_owned(),
        Value::Date(_) => "date".to_owned(),
        Value::Time(_) => "time".to_owned(),
        Value::DateTime(_) => "datetime".to_owned(),
        Value::Elapsed(_) => "elapsed".to_owned(),
        Value::Number(number) => format!("number {number}"),
        other => format!("{other:?}"),
    }
}

/// The values of column A of the first worksheet of the workbook at `path`,
/// by row, where the workbook has a row, and the diagnostics read gave.
fn column_a(path: &Path) -> (Vec<(u32, String)>, usize) {
    let mut diagnostics = Diagnostics::new();
    let file = std::fs::File::open(path).unwrap();
    let book = Book::open(
        file,
        &PackageLimits::default(),
        &ReadLimits::default(),
        &mut TempFiles::new(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    let index = (0..book.sheets().len())
        .find(|index| book.rows(*index).is_some())
        .unwrap();
    let mut rows = book.rows(index).unwrap();
    let mut row = Row::new();
    let mut found = Vec::new();
    while rows.next_row(&mut row, &mut diagnostics).unwrap() {
        if let Some(value) = row.get(0) {
            found.push((row.number(), shown(&value)));
        }
    }
    (found, diagnostics.records().len())
}

/// What each of A1 to A18 of a sheet that had no cell there is shown as.
const EXPECTED: [&str; 18] = [
    "Bold",
    "Italic underline red",
    "Font name and size",
    "Strike superscript",
    "Solid yellow fill",
    "Gray125 pattern",
    "Thin border",
    "Thick bottom double top",
    "Diagonal",
    "date",
    "time",
    "datetime",
    "elapsed",
    "number 45292.75",
    "number 45292.75",
    "Centre wrap",
    "Rotation top indent",
    "Protection",
];

#[test]
fn each_workbook_of_fixtures_is_written_with_a_cell_format_on_each_cell_of_a1_to_a18() {
    let to = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    styles(&fixtures(), to.path(), &mut out).unwrap();
    let mut written = Vec::new();
    for path in workbooks() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let result = to.path().join(&name);
        if !result.exists() {
            continue;
        }
        written.push(format!("{}\n", result.display()));
        let (before, after) = (kinds(&path), kinds(&result));
        assert!(after.len() > before.len(), "{name}");
        assert_eq!(after[..before.len()], before[..], "{name}");
        let (original, original_diagnostics) = column_a(&path);
        let (found, diagnostics) = column_a(&result);
        assert_eq!(diagnostics, original_diagnostics, "{name}");
        for (row, expected) in (1..).zip(EXPECTED) {
            let had = original.iter().find(|(number, _)| *number == row);
            let now = found.iter().find(|(number, _)| *number == row);
            match (had, now) {
                // The cell was there: it keeps its value, as long as the
                // cell format it has now shows the value the same way.
                // A row that has cells but none in column A reads as `Empty`;
                // a cell of A that is empty keeps being so.
                (Some((_, before)), Some((_, now))) if before == "Empty" => {
                    assert!(now == "Empty" || now == expected, "{name} A{row}: {now}");
                }
                (Some((_, before)), Some((_, now))) if before.starts_with(char::is_alphabetic) => {
                    assert_eq!(now, before, "{name} A{row}");
                }
                (Some(_), Some(_)) => {}
                (None, Some((_, now))) => assert_eq!(now, expected, "{name} A{row}"),
                other => panic!("{name} A{row}: {other:?}"),
            }
        }
        let rest = |rows: &[(u32, String)]| -> Vec<(u32, String)> {
            rows.iter()
                .filter(|(number, _)| *number > 18)
                .cloned()
                .collect()
        };
        assert_eq!(rest(&found), rest(&original), "{name}");
    }
    assert_eq!(String::from_utf8(out).unwrap(), written.concat());
    assert!(written.len() >= 5, "{written:?}");
}

/// A styles part with one cell format.
const STYLES_XML: &str = concat!(
    r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    r#"<fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>"#,
    r#"<fills count="2"><fill><patternFill patternType="none"/></fill>"#,
    r#"<fill><patternFill patternType="gray125"/></fill></fills>"#,
    r#"<borders count="1"><border/></borders>"#,
    r#"<cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellXfs>"#,
    "</styleSheet>",
);

/// A worksheet with a string in A2 and nothing else.
const SHEET_XML: &str = concat!(
    r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    r#"<row r="2"><c r="A2" t="inlineStr"><is><t>keep</t></is></c></row>"#,
    "</sheetData></worksheet>",
);

/// The parts of a workbook with one worksheet and a styles part.
fn parts() -> Vec<(&'static str, String)> {
    let content_types = concat!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/></Types>"#,
    );
    let rel = |id: &str, kind: &str, target: &str| {
        format!(r#"<Relationship Id="{id}" Type="{TYPES}/{kind}" Target="{target}"/>"#)
    };
    let main = format!(
        r#"<Relationships xmlns="{RELS}">{}</Relationships>"#,
        rel("rId1", "officeDocument", "xl/workbook.xml")
    );
    let related = format!(
        r#"<Relationships xmlns="{RELS}">{}{}</Relationships>"#,
        rel("rId1", "worksheet", "worksheets/sheet1.xml"),
        rel("rId2", "styles", "styles.xml")
    );
    let book = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{TYPES}"><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#
    );
    vec![
        ("[Content_Types].xml", content_types.to_owned()),
        ("_rels/.rels", main),
        ("xl/workbook.xml", book),
        ("xl/_rels/workbook.xml.rels", related),
        ("xl/worksheets/sheet1.xml", SHEET_XML.to_owned()),
        ("xl/styles.xml", STYLES_XML.to_owned()),
    ]
}

/// `parts` with the part `name` holding `xml`, or removed when `xml` is
/// `None`.
fn changed(name: &str, xml: Option<&str>) -> Vec<(&'static str, String)> {
    let mut parts = parts();
    match (parts.iter().position(|(part, _)| *part == name), xml) {
        (Some(at), Some(xml)) => parts[at].1 = xml.to_owned(),
        (Some(at), None) => {
            parts.remove(at);
        }
        (None, _) => panic!("no part {name}"),
    }
    parts
}

/// What `styles` gives for a folder holding the workbook `parts`, written as
/// a zip: what it returns, what it printed, and the folder it wrote to.
fn styles_of(
    parts: &[(&str, String)],
) -> (
    Result<(), FixturesGenError>,
    String,
    tempfile::TempDir,
    PathBuf,
) {
    styles_of_zip(zip(parts))
}

/// The zip that holds `parts`.
fn zip(parts: &[(&str, String)]) -> Vec<u8> {
    let mut spools = MemorySpools;
    let mut zip = ZipWriter::new(Vec::new());
    for (name, xml) in parts {
        let mut content = DeflateWriter::new(&mut spools, 1 << 20);
        content.write_all(xml.as_bytes()).unwrap();
        let content = content.finish().unwrap();
        zip.write_deflated(name.as_bytes(), content).unwrap();
    }
    zip.finish().unwrap()
}

/// What `styles` gives for a folder holding the workbook `bytes`.
fn styles_of_zip(
    bytes: Vec<u8>,
) -> (
    Result<(), FixturesGenError>,
    String,
    tempfile::TempDir,
    PathBuf,
) {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    std::fs::create_dir(&from).unwrap();
    std::fs::write(from.join("book.xlsx"), bytes).unwrap();
    let to = dir.path().join("to");
    let mut out = Vec::new();
    let result = styles(&from, &to, &mut out);
    (result, String::from_utf8(out).unwrap(), dir, to)
}

/// The detail of the `Repack` error `result` is.
fn refused(result: Result<(), FixturesGenError>) -> String {
    match result {
        Err(FixturesGenError::Repack { detail, .. }) => detail,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_small_workbook_is_written_with_the_cells_the_catalog_gives_and_its_own_kept() {
    let (result, out, _dir, to) = styles_of(&parts());
    result.unwrap();
    let path = to.join("book.xlsx");
    assert_eq!(out, format!("{}\n", path.display()));
    let (found, diagnostics) = column_a(&path);
    assert_eq!(diagnostics, 0);
    let mut expected: Vec<(u32, String)> = (1..).zip(EXPECTED.map(str::to_owned)).collect();
    expected[1].1 = "keep".to_owned();
    assert_eq!(found, expected);
    assert_eq!(kinds(&path).len(), 1 + 18);
}

#[test]
fn a_workbook_it_cannot_read_or_with_two_styles_relationships_is_refused_with_the_codes() {
    let no_main = changed(
        "_rels/.rels",
        Some(&format!(r#"<Relationships xmlns="{RELS}"/>"#)),
    );
    let broken_book = changed("xl/workbook.xml", Some("<workbook>"));
    let no_sheets = changed(
        "xl/workbook.xml",
        Some(&format!(r#"<workbook xmlns="{NS}"><sheets/></workbook>"#)),
    );
    let mut two_styles = changed(
        "xl/_rels/workbook.xml.rels",
        Some(&format!(
            r#"<Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{TYPES}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{TYPES}/styles" Target="styles.xml"/><Relationship Id="rId3" Type="{TYPES}/styles" Target="styles2.xml"/></Relationships>"#
        )),
    );
    two_styles.push(("xl/styles2.xml", STYLES_XML.to_owned()));
    let cases = [
        (no_main, "main_document_missing, part_unreferenced"),
        (broken_book, "xml_schema_invalid"),
        (no_sheets, "xml_schema_invalid"),
        (two_styles, "relationship_type_duplicate"),
    ];
    for (parts, codes) in cases {
        let (result, out, _dir, to) = styles_of(&parts);
        assert_eq!(refused(result), format!("the diagnostics are {codes}"));
        assert_eq!(out, "");
        assert!(!to.join("book.xlsx").exists());
    }
}

#[test]
fn a_workbook_with_no_worksheet_or_no_styles_part_gives_nothing() {
    let chartsheet = changed(
        "xl/_rels/workbook.xml.rels",
        Some(&format!(
            r#"<Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{TYPES}/chartsheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{TYPES}/styles" Target="styles.xml"/></Relationships>"#
        )),
    );
    let no_styles = changed(
        "xl/_rels/workbook.xml.rels",
        Some(&format!(
            r#"<Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{TYPES}/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#
        )),
    );
    let no_part = changed("xl/worksheets/sheet1.xml", None);
    for parts in [chartsheet, no_styles, no_part] {
        let (result, out, _dir, to) = styles_of(&parts);
        result.unwrap();
        assert_eq!(out, "");
        assert!(!to.join("book.xlsx").exists());
    }
}

#[test]
fn a_workbook_it_cannot_write_is_an_error_and_leaves_no_file() {
    let broken_styles = changed("xl/styles.xml", Some("<styleSheet><fonts>"));
    let (result, out, _dir, to) = styles_of(&broken_styles);
    assert!(refused(result).starts_with("the styles part is refused"));
    assert_eq!(out, "");
    assert!(!to.join("book.xlsx").exists());
    let broken_sheet = changed("xl/worksheets/sheet1.xml", Some("<worksheet><sheetData>"));
    let (result, _, _dir, to) = styles_of(&broken_sheet);
    assert!(refused(result).contains("xml_"));
    assert!(!to.join("book.xlsx").exists());
    let no_r = changed(
        "xl/worksheets/sheet1.xml",
        Some("<worksheet><sheetData><row><c/></row></sheetData></worksheet>"),
    );
    let (result, _, _dir, _) = styles_of(&no_r);
    assert!(refused(result).contains("a <row> has no r attribute"));
}

#[test]
fn a_styles_part_whose_data_is_damaged_is_an_error_with_the_diagnostics() {
    let mut bytes = zip(&parts());
    let name = b"xl/styles.xml";
    let at = bytes.windows(name.len()).position(|w| w == name).unwrap();
    // The deflated data follows the name in the entry's local header.
    bytes[at + name.len() + 2] ^= 0xFF;
    let (result, out, _dir, to) = styles_of_zip(bytes);
    assert!(refused(result).starts_with("the diagnostics are"));
    assert_eq!(out, "");
    assert!(!to.join("book.xlsx").exists());
}

#[test]
fn a_file_that_is_not_a_package_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("book.xlsx"), b"not a zip").unwrap();
    let result = styles(dir.path(), &dir.path().join("to"), &mut Vec::new());
    assert!(refused(result).starts_with("the diagnostics are"));
}

#[test]
fn styles_refuses_a_command_line_that_is_not_two_directories() {
    for words in [
        &["styles"][..],
        &["styles", "a"],
        &["styles", "a", "b", "c"],
    ] {
        let args: Vec<OsString> = words.iter().map(OsString::from).collect();
        let err = run(&args, &mut Vec::new()).unwrap_err();
        assert!(matches!(err, FixturesGenError::Usage));
    }
}

#[test]
fn styles_returns_an_io_error_for_a_folder_it_cannot_read_or_write_to() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let err = styles(&missing, &dir.path().join("to"), &mut Vec::new()).unwrap_err();
    assert!(matches!(err, FixturesGenError::Io { .. }), "{err:?}");
    let file = dir.path().join("file");
    std::fs::write(&file, b"").unwrap();
    let err = styles(dir.path(), &file, &mut Vec::new()).unwrap_err();
    assert!(matches!(err, FixturesGenError::Io { .. }), "{err:?}");
}

/// A writer that fails.
struct Failing;

impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("closed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn styles_returns_an_output_error_when_the_path_cannot_be_printed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(fixtures().join("table.xlsx"), dir.path().join("table.xlsx")).unwrap();
    let err = styles(dir.path(), &dir.path().join("to"), &mut Failing).unwrap_err();
    assert!(matches!(err, FixturesGenError::Output(_)), "{err:?}");
}
