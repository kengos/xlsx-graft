//! `layout` over small workbooks: what it writes into the first sheet and
//! the workbook, what it prints for an edit the engine refuses, and the
//! workbooks and folders it skips or refuses.

use super::sheet::{MERGES, PASSWORD, SALT, SPIN_COUNT};
use crate::{FixturesGenError, run};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::sheet::PasswordHash;
use xlsx_graft_write::{DeflateWriter, ZipWriter};

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A sheet of a small workbook: its `state` attribute (`""` or
/// ` state="hidden"`), the relationship type of its part and its part's
/// text.
struct Sheet {
    state: &'static str,
    kind: &'static str,
    xml: String,
}

/// A worksheet, visible, with `body` after its `sheetData`.
fn worksheet(body: &str) -> Sheet {
    Sheet {
        state: "",
        kind: "worksheet",
        xml: format!(r#"<worksheet xmlns="{NS}"><sheetData/>{body}</worksheet>"#),
    }
}

/// The parts of a workbook whose sheets `Sheet 1`, `Sheet 2` and on are
/// `sheets`.
fn parts(sheets: &[Sheet]) -> Vec<(String, String)> {
    let rel = |id: String, kind: &str, target: String| {
        format!(r#"<Relationship Id="{id}" Type="{TYPES}/{kind}" Target="{target}"/>"#)
    };
    let mut related = Vec::new();
    let mut listed = String::new();
    let mut files = Vec::new();
    for (at, sheet) in sheets.iter().enumerate() {
        let n = at + 1;
        let target = format!("worksheets/sheet{n}.xml");
        related.push(rel(format!("rId{n}"), sheet.kind, target.clone()));
        listed += &format!(
            r#"<sheet name="Sheet {n}" sheetId="{n}"{} r:id="rId{n}"/>"#,
            sheet.state
        );
        files.push((format!("xl/{target}"), sheet.xml.clone()));
    }
    let content_types = concat!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/></Types>"#,
    );
    let main = rel("rId1".into(), "officeDocument", "xl/workbook.xml".into());
    let book =
        format!(r#"<workbook xmlns="{NS}" xmlns:r="{TYPES}"><sheets>{listed}</sheets></workbook>"#);
    let mut all = vec![
        ("[Content_Types].xml".to_owned(), content_types.to_owned()),
        (
            "_rels/.rels".to_owned(),
            format!(r#"<Relationships xmlns="{RELS}">{main}</Relationships>"#),
        ),
        ("xl/workbook.xml".to_owned(), book),
        (
            "xl/_rels/workbook.xml.rels".to_owned(),
            format!(
                r#"<Relationships xmlns="{RELS}">{}</Relationships>"#,
                related.concat()
            ),
        ),
    ];
    all.extend(files);
    all
}

/// The zip that holds `parts`.
fn zip(parts: &[(String, String)]) -> Vec<u8> {
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

/// What `layout` prints and where it writes, for a folder holding `book.xlsx`
/// of `sheets`.
fn layout_of(sheets: &[Sheet]) -> (String, tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    std::fs::create_dir(&from).unwrap();
    std::fs::write(from.join("book.xlsx"), zip(&parts(sheets))).unwrap();
    let to = dir.path().join("to");
    let mut out = Vec::new();
    layout(&from, &to, &mut out).unwrap();
    (String::from_utf8(out).unwrap(), dir, to.join("book.xlsx"))
}

fn layout(from: &Path, to: &Path, out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let args: Vec<OsString> = vec!["layout".into(), from.into(), to.into()];
    run(&args, out)
}

/// The text of the part `name` of the workbook at `path`.
fn part_text(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut diagnostics = Diagnostics::new();
    let mut package = Package::open(file, &Limits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let index = (package.parts().iter())
        .position(|part| part.name().as_str() == name)
        .unwrap();
    let limit = xlsx_graft_diagnostics::Limit::default_of(1 << 28);
    let text = package
        .read_xml_part(index, limit, &mut diagnostics)
        .unwrap()
        .unwrap();
    String::from_utf8(text).unwrap()
}

/// The `state` of each `sheet` element of the workbook at `path`, and its
/// `activeTab`.
fn states(path: &Path) -> (Vec<String>, Option<String>) {
    let text = part_text(path, "/xl/workbook.xml");
    let sheets = text.split("<sheet ").skip(1);
    let states = sheets.map(|sheet| match sheet.split_once("state=\"") {
        Some((_, rest)) if sheet.find("state=\"") < sheet.find("/>") => {
            rest.split('"').next().unwrap().to_owned()
        }
        _ => "visible".to_owned(),
    });
    let active = text
        .split_once("activeTab=\"")
        .map(|(_, rest)| rest.split('"').next().unwrap().to_owned());
    (states.collect(), active)
}

const SHEET1: &str = "/xl/worksheets/sheet1.xml";

#[test]
fn a_small_workbook_has_its_first_sheet_laid_out_and_prints_only_its_path() {
    let (out, _dir, result) = layout_of(&[worksheet("")]);
    assert_eq!(out, format!("{}\n", result.display()));
    let text = part_text(&result, SHEET1);
    let hash = PasswordHash::sha512(PASSWORD, SALT, SPIN_COUNT).unwrap();
    assert_eq!(hash.spin_count(), 100_000);
    for wanted in [
        r#"<mergeCells count="1"><mergeCell ref="AA1000:AB1001"/></mergeCells>"#,
        r#"<col min="2" max="4" width="20" customWidth="1"/>"#,
        r#"<col min="6" max="6" width="30" customWidth="1"/>"#,
        r#"<row r="1" ht="30" customHeight="1"/>"#,
        r#"<row r="500" ht="24" customHeight="1"/>"#,
        r#"state="frozen""#,
        r#"algorithmName="SHA-512""#,
        r#"spinCount="100000""#,
    ] {
        assert!(text.contains(wanted), "{wanted}: {text}");
    }
    // One sheet: no sheet is made active, hidden or very hidden.
    assert_eq!(states(&result), (vec!["visible".to_owned()], None));
}

#[test]
fn the_salt_is_the_bytes_0_to_15_and_the_merges_are_far_from_the_fixtures_cells() {
    assert_eq!(SALT.to_vec(), (0..16).collect::<Vec<u8>>());
    assert_eq!(MERGES, ["AA1000:AB1001", "AD2000:AE2001"]);
}

#[test]
fn a_merge_over_a_range_the_sheet_has_is_made_at_the_second_range_and_printed() {
    let body = r#"<mergeCells count="1"><mergeCell ref="AB1001:AC1002"/></mergeCells>"#;
    let (out, _dir, result) = layout_of(&[worksheet(body)]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[1].starts_with("  merged AD2000:AE2001, not AA1000:AB1001 ("),
        "{out}"
    );
    let text = part_text(&result, SHEET1);
    assert!(text.contains(r#"ref="AD2000:AE2001""#), "{text}");
    assert!(!text.contains(r#"ref="AA1000:AB1001""#), "{text}");
    assert!(text.contains(r#"count="2""#), "{text}");
}

#[test]
fn no_merge_is_made_when_both_ranges_are_over_ranges_the_sheet_has() {
    let body = concat!(
        r#"<mergeCells count="2"><mergeCell ref="AA1000:AA1001"/>"#,
        r#"<mergeCell ref="AD2000:AD2001"/></mergeCells>"#,
    );
    let (out, _dir, result) = layout_of(&[worksheet(body)]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[1].starts_with("  merged no range: AA1000:AB1001 ("),
        "{out}"
    );
    assert!(lines[1].contains(", AD2000:AE2001 ("), "{out}");
    let text = part_text(&result, SHEET1);
    assert!(text.contains(r#"count="2""#), "{text}");
    assert!(text.contains("state=\"frozen\""), "{text}");
}

#[test]
fn sheets_are_made_active_hidden_and_very_hidden_by_how_many_there_are() {
    for (count, expected, active) in [
        (2, vec!["visible", "visible"], Some("1")),
        (3, vec!["visible", "hidden", "visible"], Some("2")),
        (
            4,
            vec!["visible", "hidden", "veryHidden", "visible"],
            Some("3"),
        ),
        (
            5,
            vec!["visible", "hidden", "veryHidden", "visible", "visible"],
            Some("4"),
        ),
    ] {
        let sheets: Vec<Sheet> = (0..count).map(|_| worksheet("")).collect();
        let (out, _dir, result) = layout_of(&sheets);
        assert_eq!(out, format!("{}\n", result.display()), "{count}");
        let (states, tab) = states(&result);
        assert_eq!(states, expected, "{count}");
        assert_eq!(tab.as_deref(), active, "{count}");
    }
}

#[test]
fn a_state_or_an_active_sheet_the_workbook_does_not_allow_is_left_and_printed() {
    // The first and last sheets are hidden: the last cannot be made active,
    // and, with the second visible, the third cannot be very hidden once
    // the second is hidden.
    let hide = |sheet: Sheet| Sheet {
        state: r#" state="hidden""#,
        ..sheet
    };
    let sheets = [
        hide(worksheet("")),
        worksheet(""),
        worksheet(""),
        hide(worksheet("")),
    ];
    let (out, _dir, result) = layout_of(&sheets);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert!(
        lines[1].starts_with("  did not make \"Sheet 4\" the active sheet: "),
        "{out}"
    );
    assert!(
        lines[2].starts_with("  did not very hide \"Sheet 3\": "),
        "{out}"
    );
    assert_eq!(states(&result).0, ["hidden", "hidden", "visible", "hidden"]);
}

#[test]
fn a_first_sheet_that_is_not_a_worksheet_is_not_edited_and_the_workbook_is_still_written() {
    let chart = Sheet {
        state: "",
        kind: "chartsheet",
        xml: format!(r#"<chartsheet xmlns="{NS}"/>"#),
    };
    let (out, _dir, result) = layout_of(&[chart, worksheet(""), worksheet("")]);
    let note = "  did not edit the first sheet: it is not a worksheet\n";
    assert_eq!(out, format!("{}\n{note}", result.display()));
    assert_eq!(states(&result).0, ["visible", "hidden", "visible"]);
    assert!(!part_text(&result, "/xl/worksheets/sheet2.xml").contains("mergeCell"));
}

#[test]
fn a_first_sheet_that_cannot_be_read_is_not_edited_and_the_workbook_is_still_written() {
    let broken = Sheet {
        xml: format!(r#"<worksheet xmlns="{NS}"><sheetData></worksheet>"#),
        ..worksheet("")
    };
    let (out, _dir, result) = layout_of(&[broken, worksheet("")]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[1].starts_with("  did not edit the first sheet: the diagnostics are "),
        "{out}"
    );
    assert_eq!(states(&result).1.as_deref(), Some("1"));
}

#[test]
fn a_workbook_with_no_sheet_is_skipped_with_the_diagnostics() {
    let mut book = parts(&[worksheet("")]);
    book[2].1 = format!(r#"<workbook xmlns="{NS}"><sheets/></workbook>"#);
    book[3].1 = format!(r#"<Relationships xmlns="{RELS}"/>"#);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("book.xlsx"), zip(&book)).unwrap();
    let mut out = Vec::new();
    layout(dir.path(), &dir.path().join("to"), &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("book.xlsx: the diagnostics are "), "{out}");
}

#[test]
fn a_workbook_it_cannot_read_is_skipped_and_the_next_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    std::fs::create_dir(&from).unwrap();
    std::fs::write(from.join("a.xlsx"), b"not a zip").unwrap();
    std::fs::write(from.join("b.xlsx"), zip(&parts(&[worksheet("")]))).unwrap();
    std::fs::create_dir(from.join("c.xlsx")).unwrap();
    let to = dir.path().join("to");
    let mut out = Vec::new();
    layout(&from, &to, &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert!(
        lines[0].starts_with("skipped ") && lines[0].contains("a.xlsx: "),
        "{out}"
    );
    assert!(lines[1].ends_with("b.xlsx"), "{out}");
    assert!(
        lines[2].starts_with("skipped ") && lines[2].contains("c.xlsx: "),
        "{out}"
    );
    assert!(to.join("b.xlsx").exists() && !to.join("a.xlsx").exists());
}

#[test]
fn a_workbook_the_engine_refuses_is_skipped_with_the_diagnostics() {
    let no_main: Vec<(String, String)> = parts(&[worksheet("")])
        .into_iter()
        .filter(|(name, _)| name != "_rels/.rels")
        .collect();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("book.xlsx"), zip(&no_main)).unwrap();
    let mut out = Vec::new();
    layout(dir.path(), &dir.path().join("to"), &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(
        out.contains("book.xlsx: the diagnostics are main_document_missing"),
        "{out}"
    );
}

#[test]
fn a_workbook_that_cannot_be_written_to_its_place_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    std::fs::create_dir(&from).unwrap();
    std::fs::write(from.join("book.xlsx"), zip(&parts(&[worksheet("")]))).unwrap();
    let to = dir.path().join("to");
    // A folder is where the workbook is to be written.
    std::fs::create_dir_all(to.join("book.xlsx")).unwrap();
    let mut out = Vec::new();
    layout(&from, &to, &mut out).unwrap();
    assert!(String::from_utf8(out).unwrap().starts_with("skipped "));
}

#[test]
fn layout_refuses_a_command_line_that_is_not_two_directories() {
    for words in [
        &["layout"][..],
        &["layout", "a"],
        &["layout", "a", "b", "c"],
    ] {
        let args: Vec<OsString> = words.iter().map(OsString::from).collect();
        let err = run(&args, &mut Vec::new()).unwrap_err();
        assert!(matches!(err, FixturesGenError::Usage));
    }
}

#[test]
fn layout_returns_an_io_error_for_a_folder_it_cannot_read_or_write_to() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let err = layout(&missing, &dir.path().join("to"), &mut Vec::new()).unwrap_err();
    assert!(matches!(err, FixturesGenError::Io { .. }), "{err:?}");
    let file = dir.path().join("file");
    std::fs::write(&file, b"").unwrap();
    let err = layout(dir.path(), &file, &mut Vec::new()).unwrap_err();
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
fn layout_returns_an_output_error_when_a_line_cannot_be_printed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("book.xlsx"), zip(&parts(&[worksheet("")]))).unwrap();
    let err = layout(dir.path(), &dir.path().join("to"), &mut Failing).unwrap_err();
    assert!(matches!(err, FixturesGenError::Output(_)), "{err:?}");
}

#[test]
fn an_entry_the_edit_does_not_read_but_cannot_copy_is_skipped_with_the_diagnostics() {
    let mut book = parts(&[worksheet("")]);
    book.push(("docProps/blob.bin".to_owned(), "data".to_owned()));
    let mut bytes = zip(&book);
    // Give the last entry a compression method no reader supports.
    let central = bytes.windows(4).enumerate();
    let mut at = central
        .filter(|(_, w)| w == b"PK\x01\x02")
        .map(|(at, _)| at);
    let at = at.next_back().unwrap();
    bytes[at + 10] = 99;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("book.xlsx"), bytes).unwrap();
    let to = dir.path().join("to");
    let mut out = Vec::new();
    layout(dir.path(), &to, &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("write_entry_unreadable"), "{out}");
    assert!(!to.join("book.xlsx").exists());
}

/// A worksheet with `cols` before its `sheetData`.
fn with_cols(cols: &str) -> Sheet {
    Sheet {
        state: "",
        kind: "worksheet",
        xml: format!(r#"<worksheet xmlns="{NS}"><cols>{cols}</cols><sheetData/></worksheet>"#),
    }
}

#[test]
fn the_width_30_is_on_the_middle_column_of_the_first_col_that_covers_two_columns() {
    let cols = r#"<col min="1" max="1" width="7"/><col min="2" max="4" width="9" style="2"/>"#;
    let (_, _dir, result) = layout_of(&[with_cols(cols)]);
    let text = part_text(&result, SHEET1);
    assert!(
        text.contains(r#"<col min="3" max="3" width="30" style="2" customWidth="1"/>"#),
        "{text}"
    );
    assert!(!text.contains(r#"min="6""#), "{text}");
}

#[test]
fn the_width_30_is_on_column_f_when_no_col_covers_two_columns_in_the_sheet() {
    for cols in [
        r#"<col min="1" max="1" width="7"/>"#,
        r#"<col min="x" max="3"/><col max="3"/><col min="3" max="3"/><col min="4" max="99999"/><col min="16380" max="20000"/><col min="0" max="1"/>"#,
    ] {
        let (_, _dir, result) = layout_of(&[with_cols(cols)]);
        let text = part_text(&result, SHEET1);
        assert!(
            text.contains(r#"<col min="6" max="6" width="30" customWidth="1"/>"#),
            "{text}"
        );
    }
}

#[test]
fn the_middle_column_is_the_one_between_min_and_max_not_between_1_and_max() {
    let (_, _dir, result) = layout_of(&[with_cols(r#"<col min="2" max="6" width="9"/>"#)]);
    let text = part_text(&result, SHEET1);
    assert!(
        text.contains(r#"<col min="4" max="4" width="30" customWidth="1"/>"#),
        "{text}"
    );
}

#[test]
fn a_col_with_an_end_tag_or_in_another_namespace_is_found_by_its_local_name() {
    let (_, _dir, result) = layout_of(&[with_cols(r#"<col min="1" max="3"></col>"#)]);
    let text = part_text(&result, SHEET1);
    assert!(
        text.contains(r#"<col min="2" max="2" width="30" customWidth="1"/>"#),
        "{text}"
    );
    // A `col` in another namespace (`x:col` in `urn:x`) is chosen too;
    // `SheetEdit` does not take the `x:col` for a column, writes it unchanged
    // and writes a new `col` for column 2.
    let prefixed = r#"<x:col xmlns:x="urn:x" min="1" max="3"/>"#;
    let (_, _dir, result) = layout_of(&[with_cols(prefixed)]);
    let text = part_text(&result, SHEET1);
    assert!(
        text.contains(r#"<col min="2" max="2" width="30" customWidth="1"/>"#),
        "{text}"
    );
}
