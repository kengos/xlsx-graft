//! `structure` over the real `fixtures/` and over small workbooks: the sheets
//! and defined names of each result read back, what it prints, and the
//! workbooks and folders it skips or refuses.

use super::variant::{ADDED, DEFINED_NAME, RENAMED};
use crate::{FixturesGenError, run};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::structure::model::Workbook;
use xlsx_graft_write::{DeflateWriter, WriteLimits, ZipWriter};

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// The repository's `fixtures/` folder.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn structure(from: &Path, to: &Path, out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let args: Vec<OsString> = vec!["structure".into(), from.into(), to.into()];
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

/// The names of the sheets, in order, and the defined names with their
/// formulas, of the workbook at `path`.
fn read(path: &Path) -> (Vec<String>, Vec<(String, String)>) {
    let mut diagnostics = Diagnostics::new();
    let file = std::fs::File::open(path).unwrap();
    let mut package = Package::open(file, &Limits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let limits = WriteLimits::default();
    let book = Workbook::read(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    let sheets = book.sheets().iter().map(|s| s.name().to_owned()).collect();
    let names = (book.defined_names().iter())
        .map(|name| (name.name().to_owned(), name.text().to_owned()))
        .collect();
    (sheets, names)
}

/// The sheet names `<stem>-rename` has when the original has `names`.
fn renamed(names: &[String]) -> Vec<String> {
    let mut sheets = names.to_vec();
    sheets[0] = RENAMED.to_owned();
    sheets.insert(0, ADDED.to_owned());
    let last = sheets.pop().unwrap();
    sheets.insert(1, last);
    sheets
}

/// The sheet names `<stem>-delete` has when the original has `names`.
fn deleted(names: &[String]) -> Vec<String> {
    let mut sheets = names[1..].to_vec();
    sheets.push(ADDED.to_owned());
    sheets
}

#[test]
fn each_workbook_of_fixtures_is_written_with_a_sheet_renamed_added_moved_and_deleted() {
    let to = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    structure(&fixtures(), to.path(), &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    let (mut written, mut single, mut several) = (0, 0, 0);
    for path in workbooks() {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let ext = path.extension().unwrap().to_string_lossy().into_owned();
        let (original, _) = read(&path);
        match original.len() {
            1 => single += 1,
            _ => several += 1,
        }
        for (suffix, expected) in [
            ("rename", renamed(&original)),
            ("delete", deleted(&original)),
        ] {
            let result = to.path().join(format!("{stem}-{suffix}.{ext}"));
            assert!(result.exists(), "{stem}-{suffix}: {out}");
            written += 1;
            assert!(out.contains(&format!("{}\n", result.display())), "{out}");
            let (sheets, names) = read(&result);
            assert_eq!(sheets, expected, "{stem}-{suffix}");
            let defined = (DEFINED_NAME.to_owned(), format!("'{RENAMED}'!$A$1"));
            assert_eq!(names.contains(&defined), suffix == "rename", "{stem}");
        }
    }
    assert!(written >= 20, "{written}");
    assert!(single > 0 && several > 0, "{single} {several}");
    assert!(!out.contains("skipped"), "{out}");
}

/// A workbook with one worksheet named `sheet`, which holds `body` after its
/// `sheetData`.
fn parts(sheet: &str, body: &str) -> Vec<(&'static str, String)> {
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
        r#"<Relationships xmlns="{RELS}">{}</Relationships>"#,
        rel("rId1", "worksheet", "worksheets/sheet1.xml"),
    );
    let book = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{TYPES}"><sheets><sheet name="{sheet}" sheetId="1" r:id="rId1"/></sheets></workbook>"#
    );
    let worksheet = format!(r#"<worksheet xmlns="{NS}"><sheetData/>{body}</worksheet>"#);
    vec![
        ("[Content_Types].xml", content_types.to_owned()),
        ("_rels/.rels", main),
        ("xl/workbook.xml", book),
        ("xl/_rels/workbook.xml.rels", related),
        ("xl/worksheets/sheet1.xml", worksheet),
    ]
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

/// What `structure` prints and where it writes, for a folder holding the
/// workbooks `books` (a file name and its bytes).
fn structure_of(books: &[(&str, Vec<u8>)]) -> (String, tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    std::fs::create_dir(&from).unwrap();
    for (name, bytes) in books {
        std::fs::write(from.join(name), bytes).unwrap();
    }
    let to = dir.path().join("to");
    let mut out = Vec::new();
    structure(&from, &to, &mut out).unwrap();
    (String::from_utf8(out).unwrap(), dir, to)
}

#[test]
fn a_small_workbook_is_written_with_both_variants_and_prints_no_kept_reference() {
    let (out, _dir, to) = structure_of(&[("book.xlsx", zip(&parts("One", "")))]);
    let (rename, delete) = (to.join("book-rename.xlsx"), to.join("book-delete.xlsx"));
    assert_eq!(out, format!("{}\n{}\n", rename.display(), delete.display()));
    assert_eq!(read(&rename).0, [ADDED, RENAMED]);
    assert_eq!(read(&delete).0, [ADDED]);
}

#[test]
fn a_variant_the_edit_refuses_is_skipped_with_the_reason_and_the_other_is_written() {
    // The delete variant adds `Graft added`, which the workbook's sheet has.
    let (out, _dir, to) = structure_of(&[("book.xlsx", zip(&parts("graft ADDED", "")))]);
    let source = to.parent().unwrap().join("from/book.xlsx");
    let rename = to.join("book-rename.xlsx");
    let skipped = format!("skipped {} (delete): ", source.display());
    let detail = out.strip_prefix(&format!("{}\n", rename.display()));
    let detail = detail.unwrap().strip_prefix(&skipped).unwrap();
    assert!(detail.contains("letter case aside"), "{out}");
    assert_eq!(read(&rename).0, [ADDED, RENAMED]);
    assert!(!to.join("book-delete.xlsx").exists());
}

#[test]
fn a_workbook_it_cannot_read_is_skipped_for_each_variant_and_the_next_is_written() {
    let no_main = parts("One", "")
        .into_iter()
        .filter(|(name, _)| *name != "_rels/.rels")
        .collect::<Vec<_>>();
    let books = [
        ("a.xlsx", zip(&no_main)),
        ("b.xlsx", zip(&parts("One", ""))),
        ("c.xlsx", b"not a zip".to_vec()),
    ];
    let (out, _dir, to) = structure_of(&books);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 6, "{out}");
    assert!(
        lines[0]
            .ends_with("(rename): the diagnostics are main_document_missing, part_unreferenced")
    );
    assert!(lines[1].contains("a.xlsx (delete): the diagnostics are main_document_missing"));
    assert!(lines[2].ends_with("b-rename.xlsx") && lines[3].ends_with("b-delete.xlsx"));
    assert!(lines[4].contains("c.xlsx (rename): the diagnostics are"));
    assert!(to.join("b-rename.xlsx").exists());
    assert!(!to.join("a-rename.xlsx").exists() && !to.join("c-delete.xlsx").exists());
}

#[test]
fn a_workbook_with_an_entry_that_cannot_be_read_is_skipped() {
    let mut bytes = std::fs::read(fixtures().join("table.xlsx")).unwrap();
    // Give the third entry a compression method no reader supports.
    let central = bytes.windows(4).enumerate();
    let central = central.filter(|(_, w)| w == b"PK\x01\x02");
    let at = central.map(|(at, _)| at).nth(2).unwrap();
    bytes[at + 10] = 99;
    let (out, _dir, to) = structure_of(&[("broken.xlsx", bytes)]);
    assert_eq!(out.matches("skipped").count(), 2, "{out}");
    assert!(!to.join("broken-rename.xlsx").exists());
    assert!(!to.join("broken-delete.xlsx").exists());
}

#[test]
fn a_workbook_that_cannot_be_written_to_its_place_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    std::fs::create_dir(&from).unwrap();
    std::fs::write(from.join("book.xlsx"), zip(&parts("One", ""))).unwrap();
    let to = dir.path().join("to");
    // A folder is where the rename variant is to be written.
    std::fs::create_dir_all(to.join("book-rename.xlsx")).unwrap();
    let mut out = Vec::new();
    structure(&from, &to, &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(out.starts_with("skipped "), "{out}");
    assert!(out.ends_with("book-delete.xlsx\n"), "{out}");
}

#[test]
fn a_folder_named_as_a_workbook_is_skipped_as_a_package_that_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("folder.xlsx")).unwrap();
    let mut out = Vec::new();
    structure(dir.path(), &dir.path().join("to"), &mut out).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap().matches("skipped").count(),
        2
    );
}

#[test]
fn structure_refuses_a_command_line_that_is_not_two_directories() {
    for words in [
        &["structure"][..],
        &["structure", "a"],
        &["structure", "a", "b", "c"],
    ] {
        let args: Vec<OsString> = words.iter().map(OsString::from).collect();
        let err = run(&args, &mut Vec::new()).unwrap_err();
        assert!(matches!(err, FixturesGenError::Usage));
    }
}

#[test]
fn structure_returns_an_io_error_for_a_folder_it_cannot_read_or_write_to() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let err = structure(&missing, &dir.path().join("to"), &mut Vec::new()).unwrap_err();
    assert!(matches!(err, FixturesGenError::Io { .. }), "{err:?}");
    let file = dir.path().join("file");
    std::fs::write(&file, b"").unwrap();
    let err = structure(dir.path(), &file, &mut Vec::new()).unwrap_err();
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
fn structure_returns_an_output_error_when_a_line_cannot_be_printed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(fixtures().join("table.xlsx"), dir.path().join("table.xlsx")).unwrap();
    let err = structure(dir.path(), &dir.path().join("to"), &mut Failing).unwrap_err();
    assert!(matches!(err, FixturesGenError::Output(_)), "{err:?}");
}

#[test]
fn a_reference_the_edit_keeps_is_printed_with_its_reason_part_place_and_sheet() {
    // A data bar's `cfvo@val` is outside the places the edit rewrites. The
    // delete variant deletes the sheet that holds it, so it keeps nothing.
    let body = concat!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="dataBar" priority="1"><dataBar>"#,
        r#"<cfvo type="formula" val="One!$A$1"/><cfvo type="max"/><color rgb="FFFF0000"/>"#,
        "</dataBar></cfRule></conditionalFormatting>",
    );
    let (out, _dir, to) = structure_of(&[("book.xlsx", zip(&parts("One", body)))]);
    let (rename, delete) = (to.join("book-rename.xlsx"), to.join("book-delete.xlsx"));
    let kept = concat!(
        "  kept Outside: /xl/worksheets/sheet1.xml cfvo@val at Some(\"A1\"), ",
        "sheet Some(\"One\")\n",
    );
    assert_eq!(
        out,
        format!("{}\n{kept}{}\n", rename.display(), delete.display())
    );
}

#[test]
fn a_reference_to_a_table_removed_is_printed_with_the_table_as_its_part_writes_it() {
    // The delete variant deletes the sheet `One`, whose table `Sales` goes
    // with it; the defined name's formula spells it `SALES`.
    let mut book = parts("One", "");
    let table = format!(r#"<table xmlns="{NS}" name="Sales" displayName="Sales" ref="A1:B3"/>"#);
    let rels = format!(
        r#"<Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{TYPES}/table" Target="../tables/table1.xml"/></Relationships>"#
    );
    book[0].1 = book[0].1.replace(
        "</Types>",
        r#"<Override PartName="/xl/tables/table1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml"/></Types>"#,
    );
    book[2].1 = book[2].1.replace(
        "</sheets>",
        r#"</sheets><definedNames><definedName name="Tot">SUM(SALES[Amount])</definedName></definedNames>"#,
    );
    book.push(("xl/worksheets/_rels/sheet1.xml.rels", rels));
    book.push(("xl/tables/table1.xml", table));
    let (out, _dir, to) = structure_of(&[("book.xlsx", zip(&book))]);
    let (rename, delete) = (to.join("book-rename.xlsx"), to.join("book-delete.xlsx"));
    let kept = concat!(
        "  kept DeletedTable: /xl/workbook.xml definedName at Some(\"definedName Tot\"), ",
        "sheet None, table \"Sales\"\n",
    );
    assert_eq!(
        out,
        format!("{}\n{}\n{kept}", rename.display(), delete.display())
    );
}

#[test]
fn the_references_a_report_did_not_hold_are_counted_in_one_line_after_the_ones_it_held() {
    // 16,385 data bar values outside the places the edit rewrites, each naming
    // `One`: the rename variant keeps 16,384 and counts one.
    let body = format!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="dataBar" priority="1"><dataBar>{}</dataBar></cfRule></conditionalFormatting>"#,
        r#"<cfvo type="formula" val="One!$A$1"/>"#.repeat(16_385)
    );
    let (out, _dir, to) = structure_of(&[("book.xlsx", zip(&parts("One", &body)))]);
    let (rename, delete) = (to.join("book-rename.xlsx"), to.join("book-delete.xlsx"));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 16_384 + 3, "{}", lines.len());
    assert_eq!(lines[0], rename.display().to_string());
    assert!(
        lines[1..=16_384]
            .iter()
            .all(|l| l.starts_with("  kept Outside"))
    );
    assert_eq!(lines[16_385], "  omitted 1 more");
    assert_eq!(lines[16_386], delete.display().to_string());
}

#[test]
fn an_entry_the_edit_does_not_read_but_cannot_copy_is_skipped_with_the_diagnostics() {
    let mut book = parts("One", "");
    book.push(("docProps/blob.bin", "data".to_owned()));
    let mut bytes = zip(&book);
    // Give the last entry a compression method no reader supports.
    let central = bytes.windows(4).enumerate();
    let mut at = central
        .filter(|(_, w)| w == b"PK\x01\x02")
        .map(|(at, _)| at);
    let at = at.next_back().unwrap();
    bytes[at + 10] = 99;
    let (out, _dir, to) = structure_of(&[("book.xlsx", bytes)]);
    assert_eq!(out.matches("write_entry_unreadable").count(), 2, "{out}");
    assert!(!to.join("book-rename.xlsx").exists());
    assert!(!to.join("book-delete.xlsx").exists());
}
